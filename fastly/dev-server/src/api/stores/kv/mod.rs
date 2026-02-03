use axum::extract::{Json, Path, State};
use chrono::{DateTime, Utc};
use http::StatusCode;
use redb::{ReadableDatabase, ReadableTable};
use serde::{Deserialize, Serialize};

use crate::api::{Context, Result, Router, error::Error};
use crate::tables::{KVStoreMetadata, KVStoreTable, METADATA_TABLE};
use crate::util::JsonRecord;

mod keys;

#[derive(Debug, Clone, Serialize)]
struct KVStore {
    id: String,
    name: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    deleted_at: Option<DateTime<Utc>>,
}

pub fn router() -> Router {
    use axum::routing;

    Router::new()
        .route("/", routing::get(list_kv_stores).post(create_kv_store))
        .route("/{id}", routing::get(get_kv_store).delete(delete_kv_store))
        .merge(keys::router())
}

#[derive(Debug, Clone, Default, Serialize)]
struct KVStoreListResponse {
    data: Vec<KVStore>,
}

async fn list_kv_stores(State(ctx): State<Context>) -> Result<Json<KVStoreListResponse>> {
    let tx = ctx.db.begin_read()?;

    let metadata_table = match tx.open_table(METADATA_TABLE) {
        Ok(table) => table,
        Err(redb::TableError::TableDoesNotExist(_)) => {
            return Ok(Json(KVStoreListResponse::default()));
        }
        Err(e) => return Err(e.into()),
    };
    let Some(metadata_record) = metadata_table.get(&())? else {
        return Ok(Json(KVStoreListResponse::default()));
    };
    let metadata = &metadata_record.value().0;

    let entries = metadata
        .kv_stores
        .iter()
        .map(|(id, store_meta)| KVStore {
            id: id.clone(),
            name: store_meta.name.clone(),
            created_at: store_meta.created_at,
            updated_at: store_meta.updated_at,
            deleted_at: None,
        })
        .collect::<Vec<KVStore>>();

    Ok(Json(KVStoreListResponse { data: entries }))
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateKVStoreRequest {
    pub name: String,
}

async fn create_kv_store(
    State(ctx): State<Context>,
    Json(payload): Json<CreateKVStoreRequest>,
) -> Result<Json<KVStore>> {
    let tx = ctx.db.begin_write()?;

    let store = {
        let mut metadata_table = tx.open_table(METADATA_TABLE)?;
        let mut metadata = metadata_table
            .get(&())?
            .map(|record| record.value().0.clone())
            .unwrap_or_default();

        let now = Utc::now();
        let id = ulid::Ulid::new().to_string();

        let store_meta = KVStoreMetadata {
            name: payload.name.clone(),
            created_at: now,
            updated_at: now,
        };
        metadata.kv_stores.insert(id.clone(), store_meta);

        metadata_table.insert(&(), &JsonRecord(metadata))?;

        KVStore {
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

async fn get_kv_store(
    State(ctx): State<Context>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<Json<KVStore>> {
    let tx = ctx.db.begin_read()?;

    let metadata_table = match tx.open_table(METADATA_TABLE) {
        Ok(table) => table,
        Err(redb::TableError::TableDoesNotExist(_)) => {
            return Err(Error::builder()
                .not_found()
                .message("KV store not found")
                .build());
        }
        Err(e) => return Err(e.into()),
    };
    let Some(metadata_record) = metadata_table.get(&())? else {
        return Err(Error::builder()
            .not_found()
            .message("KV store not found")
            .build());
    };
    let metadata = &metadata_record.value().0;

    let store_meta = match metadata.kv_stores.get(&id) {
        Some(meta) => meta,
        None => {
            return Err(Error::builder()
                .not_found()
                .message("KV store not found")
                .build());
        }
    };

    Ok(Json(KVStore {
        id: id.clone(),
        name: store_meta.name.clone(),
        created_at: store_meta.created_at,
        updated_at: store_meta.updated_at,
        deleted_at: None,
    }))
}

async fn delete_kv_store(State(ctx): State<Context>, Path(id): Path<String>) -> Result<StatusCode> {
    let tx = ctx.db.begin_write()?;

    {
        let mut metadata_table = tx.open_table(METADATA_TABLE)?;
        let mut metadata = metadata_table
            .get(&())?
            .map(|record| record.value().0.clone())
            .unwrap_or_default();

        if metadata.kv_stores.remove(&id).is_none() {
            return Err(Error::builder()
                .not_found()
                .message("KV store not found")
                .build());
        }

        metadata_table.insert(&(), &JsonRecord(metadata))?;
    }

    tx.delete_table(KVStoreTable::new(&id))?;

    tx.commit()?;

    Ok(StatusCode::NO_CONTENT)
}
