use axum::extract::{Form, Json, Path, State};
use chrono::{DateTime, Utc};
use redb::{ReadableDatabase, ReadableTable};
use serde::{Deserialize, Serialize};

use crate::api::{Context, Result, Router, error::Error};
use crate::tables::{ConfigStoreMetadata, ConfigStoreTable, METADATA_TABLE};
use crate::util::JsonRecord;

mod items;

#[derive(Debug, Clone, Serialize)]
struct ConfigStore {
    id: String,
    name: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    deleted_at: Option<DateTime<Utc>>,
}

pub fn router() -> Router {
    use axum::routing;

    Router::new()
        .route(
            "/",
            routing::get(list_config_stores).post(create_config_store),
        )
        .route(
            "/{id}",
            routing::get(get_config_store).delete(delete_config_store),
        )
        .merge(items::router())
}

async fn list_config_stores(State(ctx): State<Context>) -> Result<Json<Vec<ConfigStore>>> {
    let tx = ctx.db.begin_read()?;

    let metadata_table = match tx.open_table(METADATA_TABLE) {
        Ok(table) => table,
        Err(redb::TableError::TableDoesNotExist(_)) => {
            return Ok(Json(vec![]));
        }
        Err(e) => return Err(e.into()),
    };
    let Some(metadata_record) = metadata_table.get(&())? else {
        return Ok(Json(vec![]));
    };
    let metadata = &metadata_record.value().0;

    let entries = metadata
        .config_stores
        .iter()
        .map(|(id, store_meta)| ConfigStore {
            id: id.clone(),
            name: store_meta.name.clone(),
            created_at: store_meta.created_at,
            updated_at: store_meta.updated_at,
            deleted_at: None,
        })
        .collect::<Vec<ConfigStore>>();

    Ok(Json(entries))
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateConfigStoreRequest {
    pub name: String,
}

async fn create_config_store(
    State(ctx): State<Context>,
    Form(payload): Form<CreateConfigStoreRequest>,
) -> Result<Json<ConfigStore>> {
    let tx = ctx.db.begin_write()?;

    let store = {
        let mut metadata_table = tx.open_table(METADATA_TABLE)?;
        let mut metadata = metadata_table
            .get(&())?
            .map(|record| record.value().0.clone())
            .unwrap_or_default();

        let now = Utc::now();
        let id = ulid::Ulid::new().to_string();

        let store_meta = ConfigStoreMetadata {
            name: payload.name.clone(),
            created_at: now,
            updated_at: now,
        };
        metadata.config_stores.insert(id.clone(), store_meta);

        metadata_table.insert(&(), &JsonRecord(metadata))?;

        ConfigStore {
            id,
            name: payload.name,
            created_at: now,
            updated_at: now,
            deleted_at: None,
        }
    };

    tx.commit()?;

    Ok(Json(store))
}

async fn get_config_store(
    State(ctx): State<Context>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<Json<ConfigStore>> {
    let tx = ctx.db.begin_read()?;

    let metadata_table = match tx.open_table(METADATA_TABLE) {
        Ok(table) => table,
        Err(redb::TableError::TableDoesNotExist(_)) => {
            return Err(Error::builder()
                .not_found()
                .message("Config store not found")
                .build());
        }
        Err(e) => return Err(e.into()),
    };
    let Some(metadata_record) = metadata_table.get(&())? else {
        return Err(Error::builder()
            .not_found()
            .message("Config store not found")
            .build());
    };
    let metadata = &metadata_record.value().0;

    let store_meta = match metadata.config_stores.get(&id) {
        Some(meta) => meta,
        None => {
            return Err(Error::builder()
                .not_found()
                .message("Config store not found")
                .build());
        }
    };

    Ok(Json(ConfigStore {
        id: id.clone(),
        name: store_meta.name.clone(),
        created_at: store_meta.created_at,
        updated_at: store_meta.updated_at,
        deleted_at: None,
    }))
}

async fn delete_config_store(State(ctx): State<Context>, Path(id): Path<String>) -> Result<()> {
    let tx = ctx.db.begin_write()?;

    {
        let mut metadata_table = tx.open_table(METADATA_TABLE)?;
        let mut metadata = metadata_table
            .get(&())?
            .map(|record| record.value().0.clone())
            .unwrap_or_default();

        if metadata.config_stores.remove(&id).is_none() {
            return Err(Error::builder()
                .not_found()
                .message("Config store not found")
                .build());
        }

        metadata_table.insert(&(), &JsonRecord(metadata))?;
    }

    tx.delete_table(ConfigStoreTable::new(&id))?;

    tx.commit()?;

    Ok(())
}
