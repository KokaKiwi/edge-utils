use axum::extract::{Form, Json, Path, State};
use chrono::{DateTime, Utc};
use redb::{ReadableDatabase, ReadableTable};
use serde::{Deserialize, Serialize};

use crate::api::{Context, Result, Router, error::Error};
use crate::tables::{ConfigStoreItemMetadata, ConfigStoreTable as TableDefinition};
use crate::util::JsonRecord;

pub fn router() -> Router {
    use axum::routing;

    Router::new()
        .route("/{store_id}/items", routing::get(list_config_store_items))
        .route("/{store_id}/item", routing::post(create_config_store_item))
        .route(
            "/{store_id}/item/{item_key}",
            routing::get(get_config_store_item)
                .put(update_config_store_item)
                .delete(delete_config_store_item),
        )
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct ConfigStoreItem {
    store_id: String,
    item_key: String,
    item_value: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    deleted_at: Option<DateTime<Utc>>,
}

async fn list_config_store_items(
    Path(store_id): Path<String>,
    State(ctx): State<Context>,
) -> Result<Json<Vec<ConfigStoreItem>>> {
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

            ConfigStoreItem {
                store_id: store_id.clone(),
                item_key,
                item_value: item.item_value,
                created_at: item.created_at,
                updated_at: item.updated_at,
                deleted_at: None,
            }
        })
        .collect::<Vec<ConfigStoreItem>>();

    Ok(Json(entries))
}

#[derive(Debug, Clone, Deserialize)]
struct CreateConfigStoreItem {
    item_key: String,
    item_value: String,
}

async fn create_config_store_item(
    Path(store_id): Path<String>,
    State(ctx): State<Context>,
    Form(payload): Form<CreateConfigStoreItem>,
) -> Result<Json<ConfigStoreItem>> {
    let tx = ctx.db.begin_write()?;

    let item = {
        let definition = TableDefinition::new(&store_id);

        let mut table = tx.open_table(definition)?;

        let now = Utc::now();

        let meta = ConfigStoreItemMetadata {
            item_value: payload.item_value,
            created_at: now,
            updated_at: now,
        };
        let item = ConfigStoreItem {
            store_id: store_id.clone(),
            item_key: payload.item_key.clone(),
            item_value: meta.item_value.clone(),
            created_at: meta.created_at,
            updated_at: meta.updated_at,
            deleted_at: None,
        };

        table.insert(&payload.item_key, &JsonRecord(meta))?;

        item
    };

    tx.commit()?;

    Ok(Json(item))
}

async fn get_config_store_item(
    Path((store_id, item_key)): Path<(String, String)>,
    State(ctx): State<Context>,
) -> Result<Json<ConfigStoreItem>> {
    let tx = ctx.db.begin_read()?;

    let definition = TableDefinition::new(&store_id);

    let table = match tx.open_table(definition) {
        Ok(table) => table,
        Err(redb::TableError::TableDoesNotExist(_)) => {
            return Err(Error::builder()
                .not_found()
                .message("Config store not found")
                .build());
        }
        Err(e) => return Err(e.into()),
    };

    let record = match table.get(&item_key)? {
        Some(record) => record,
        None => {
            return Err(Error::builder()
                .not_found()
                .message("Config store item not found")
                .build());
        }
    };
    let item = record.value().0;

    let config_store_item = ConfigStoreItem {
        store_id: store_id.clone(),
        item_key: item_key.clone(),
        item_value: item.item_value,
        created_at: item.created_at,
        updated_at: item.updated_at,
        deleted_at: None,
    };

    Ok(Json(config_store_item))
}

#[derive(Debug, Clone, Deserialize)]
struct UpdateConfigStoreItem {
    item_value: String,
}

async fn update_config_store_item(
    Path((store_id, item_key)): Path<(String, String)>,
    State(ctx): State<Context>,
    Form(payload): Form<UpdateConfigStoreItem>,
) -> Result<Json<ConfigStoreItem>> {
    let tx = ctx.db.begin_write()?;

    let item = {
        let definition = TableDefinition::new(&store_id);

        let mut table = tx.open_table(definition)?;

        let now = Utc::now();

        let meta = ConfigStoreItemMetadata {
            item_value: payload.item_value,
            created_at: now,
            updated_at: now,
        };
        let item = ConfigStoreItem {
            store_id: store_id.clone(),
            item_key: item_key.clone(),
            item_value: meta.item_value.clone(),
            created_at: meta.created_at,
            updated_at: meta.updated_at,
            deleted_at: None,
        };

        table.insert(&item_key, &JsonRecord(meta))?;

        item
    };

    tx.commit()?;

    Ok(Json(item))
}

async fn delete_config_store_item(
    Path((store_id, item_key)): Path<(String, String)>,
    State(ctx): State<Context>,
) -> Result<Json<()>> {
    let tx = ctx.db.begin_write()?;

    {
        let definition = TableDefinition::new(&store_id);

        let mut table = tx.open_table(definition)?;

        table.remove(&item_key)?;
    }

    tx.commit()?;

    Ok(Json(()))
}
