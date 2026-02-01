use axum::extract::{Json, Path, State};
use bytes::Bytes;
use chrono::{DateTime, Utc};
use redb::{ReadableDatabase, ReadableTable};
use serde::Serialize;

use super::table::{KVStoreItemMetadata, TableDefinition};
use crate::api::{Context, Result, Router, error::Error};
use crate::util::JsonRecord;

pub fn router() -> Router {
    use axum::routing;

    Router::new()
        .route("/{store_id}/keys", routing::get(list_kv_keys))
        .route(
            "/{store_id}/keys/{key}",
            routing::get(get_kv_item)
                .put(upsert_kv_item)
                .delete(delete_kv_item),
        )
}

#[derive(Debug, Clone, Serialize)]
struct KVKey {
    key: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

async fn list_kv_keys(
    Path(store_id): Path<String>,
    State(ctx): State<Context>,
) -> Result<Json<Vec<KVKey>>> {
    let tx = ctx.db.begin_read()?;

    let definition = TableDefinition::new(&store_id);

    let table = match tx.open_table(definition) {
        Ok(table) => table,
        Err(redb::TableError::TableDoesNotExist(_)) => {
            return Ok(Json(vec![]));
        }
        Err(e) => return Err(e.into()),
    };

    let entries = table
        .iter()?
        .filter_map(|entry| entry.ok())
        .map(|(key, record)| {
            let item_key = key.value();
            let item = record.value().0;

            KVKey {
                key: item_key,
                created_at: item.created_at,
                updated_at: item.updated_at,
            }
        })
        .collect::<Vec<KVKey>>();

    Ok(Json(entries))
}

async fn get_kv_item(
    Path((store_id, key)): Path<(String, String)>,
    State(ctx): State<Context>,
) -> Result<Bytes> {
    let tx = ctx.db.begin_read()?;

    let definition = TableDefinition::new(&store_id);

    let table = match tx.open_table(definition) {
        Ok(table) => table,
        Err(redb::TableError::TableDoesNotExist(_)) => {
            return Err(Error::builder()
                .not_found()
                .message("KV store not found")
                .build());
        }
        Err(e) => return Err(e.into()),
    };

    let record = match table.get(&key)? {
        Some(record) => record,
        None => {
            return Err(Error::builder()
                .not_found()
                .message("KV store item not found")
                .build());
        }
    };
    let item = record.value().0;

    Ok(item.value)
}

async fn upsert_kv_item(
    Path((store_id, key)): Path<(String, String)>,
    State(ctx): State<Context>,
    body: Bytes,
) -> Result<()> {
    let tx = ctx.db.begin_write()?;

    {
        let definition = TableDefinition::new(&store_id);

        let mut table = tx.open_table(definition)?;

        let now = Utc::now();

        let meta = KVStoreItemMetadata {
            value: body,
            created_at: now,
            updated_at: now,
        };

        table.insert(&key, &JsonRecord(meta))?;
    }

    tx.commit()?;
    ctx.reload.notify_one();

    Ok(())
}

async fn delete_kv_item(
    Path((store_id, key)): Path<(String, String)>,
    State(ctx): State<Context>,
) -> Result<()> {
    let tx = ctx.db.begin_write()?;

    {
        let definition = TableDefinition::new(&store_id);

        let mut table = tx.open_table(definition)?;

        table.remove(&key)?;
    }

    tx.commit()?;
    ctx.reload.notify_one();

    Ok(())
}
