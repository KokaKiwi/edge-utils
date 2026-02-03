use axum::extract::{Json, Path, State};
use chrono::{DateTime, Utc};
use redb::{ReadableDatabase, ReadableTable};
use serde::{Deserialize, Serialize};

use crate::api::{Context, Result, Router, error::Error};
use crate::tables::{METADATA_TABLE, SecretStoreMetadata, SecretStoreTable};
use crate::util::JsonRecord;

mod secrets;

#[derive(Debug, Clone, Serialize)]
struct SecretStore {
    id: String,
    name: String,
    created_at: DateTime<Utc>,
}

pub fn router() -> Router {
    use axum::routing;

    Router::new()
        .route(
            "/",
            routing::get(list_secret_stores).post(create_secret_store),
        )
        .route(
            "/{id}",
            routing::get(get_secret_store).delete(delete_secret_store),
        )
        .merge(secrets::router())
}

#[derive(Debug, Clone, Default, Serialize)]
struct SecretStoreList {
    data: Vec<SecretStore>,
}

async fn list_secret_stores(State(ctx): State<Context>) -> Result<Json<SecretStoreList>> {
    let tx = ctx.db.begin_read()?;

    let metadata_table = match tx.open_table(METADATA_TABLE) {
        Ok(table) => table,
        Err(redb::TableError::TableDoesNotExist(_)) => {
            return Ok(Json(SecretStoreList::default()));
        }
        Err(e) => return Err(e.into()),
    };
    let Some(metadata_record) = metadata_table.get(&())? else {
        return Ok(Json(SecretStoreList::default()));
    };
    let metadata = &metadata_record.value().0;

    let entries = metadata
        .secret_stores
        .iter()
        .map(|(id, store_meta)| SecretStore {
            id: id.clone(),
            name: store_meta.name.clone(),
            created_at: store_meta.created_at,
        })
        .collect::<Vec<SecretStore>>();

    Ok(Json(SecretStoreList { data: entries }))
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateSecretStoreRequest {
    pub name: String,
}

async fn create_secret_store(
    State(ctx): State<Context>,
    Json(payload): Json<CreateSecretStoreRequest>,
) -> Result<Json<SecretStore>> {
    let tx = ctx.db.begin_write()?;

    let store = {
        let mut metadata_table = tx.open_table(METADATA_TABLE)?;
        let mut metadata = metadata_table
            .get(&())?
            .map(|record| record.value().0.clone())
            .unwrap_or_default();

        let now = Utc::now();
        let id = ulid::Ulid::new().to_string();

        let store_meta = SecretStoreMetadata {
            name: payload.name.clone(),
            created_at: now,
        };
        metadata.secret_stores.insert(id.clone(), store_meta);

        metadata_table.insert(&(), &JsonRecord(metadata))?;

        SecretStore {
            id,
            name: payload.name,
            created_at: now,
        }
    };

    tx.commit()?;

    Ok(Json(store))
}

async fn get_secret_store(
    State(ctx): State<Context>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<Json<SecretStore>> {
    let tx = ctx.db.begin_read()?;

    let metadata_table = match tx.open_table(METADATA_TABLE) {
        Ok(table) => table,
        Err(redb::TableError::TableDoesNotExist(_)) => {
            return Err(Error::builder()
                .not_found()
                .message("Secret store not found")
                .build());
        }
        Err(e) => return Err(e.into()),
    };
    let Some(metadata_record) = metadata_table.get(&())? else {
        return Err(Error::builder()
            .not_found()
            .message("Secret store not found")
            .build());
    };
    let metadata = &metadata_record.value().0;

    let store_meta = match metadata.secret_stores.get(&id) {
        Some(meta) => meta,
        None => {
            return Err(Error::builder()
                .not_found()
                .message("Secret store not found")
                .build());
        }
    };

    Ok(Json(SecretStore {
        id: id.clone(),
        name: store_meta.name.clone(),
        created_at: store_meta.created_at,
    }))
}

async fn delete_secret_store(State(ctx): State<Context>, Path(id): Path<String>) -> Result<()> {
    let tx = ctx.db.begin_write()?;

    {
        let mut metadata_table = tx.open_table(METADATA_TABLE)?;
        let mut metadata = metadata_table
            .get(&())?
            .map(|record| record.value().0.clone())
            .unwrap_or_default();

        if metadata.secret_stores.remove(&id).is_none() {
            return Err(Error::builder()
                .not_found()
                .message("Secret store not found")
                .build());
        }

        metadata_table.insert(&(), &JsonRecord(metadata))?;
    }

    tx.delete_table(SecretStoreTable::new(&id))?;

    tx.commit()?;

    Ok(())
}
