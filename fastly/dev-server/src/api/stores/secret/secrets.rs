use axum::extract::{Form, Json, Path, State};
use bytes::Bytes;
use chrono::{DateTime, Utc};
use redb::{ReadableDatabase, ReadableTable};
use serde::{Deserialize, Serialize};

use super::table::{SecretMetadata, TableDefinition};
use crate::api::{Context, Result, Router, error::Error};
use crate::util::JsonRecord;

pub fn router() -> Router {
    use axum::routing;

    Router::new()
        .route(
            "/{store_id}/secrets",
            routing::get(list_secrets).post(create_secret),
        )
        .route(
            "/{store_id}/secrets/{name}",
            routing::get(get_secret).delete(delete_secret),
        )
}

#[derive(Debug, Clone, Serialize)]
struct Secret {
    name: String,
    created_at: DateTime<Utc>,
}

async fn list_secrets(
    Path(store_id): Path<String>,
    State(ctx): State<Context>,
) -> Result<Json<Vec<Secret>>> {
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
            let name = key.value();
            let item = record.value().0;

            Secret {
                name,
                created_at: item.created_at,
            }
        })
        .collect::<Vec<Secret>>();

    Ok(Json(entries))
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateSecretRequest {
    pub name: String,
    #[serde(with = "serde_with::As::<serde_with::base64::Base64>")]
    pub secret: Bytes,
}

async fn create_secret(
    Path(store_id): Path<String>,
    State(ctx): State<Context>,
    Form(payload): Form<CreateSecretRequest>,
) -> Result<Json<Secret>> {
    let tx = ctx.db.begin_write()?;

    let secret = {
        let definition = TableDefinition::new(&store_id);

        let mut table = tx.open_table(definition)?;

        let now = Utc::now();

        let meta = SecretMetadata {
            name: payload.name.clone(),
            secret: payload.secret,
            created_at: now,
        };

        table.insert(&payload.name, &JsonRecord(meta))?;

        Secret {
            name: payload.name,
            created_at: now,
        }
    };

    tx.commit()?;
    ctx.reload.notify_one();

    Ok(Json(secret))
}

async fn get_secret(
    Path((store_id, name)): Path<(String, String)>,
    State(ctx): State<Context>,
) -> Result<Json<Secret>> {
    let tx = ctx.db.begin_read()?;

    let definition = TableDefinition::new(&store_id);

    let table = match tx.open_table(definition) {
        Ok(table) => table,
        Err(redb::TableError::TableDoesNotExist(_)) => {
            return Err(Error::builder()
                .not_found()
                .message("Secret store not found")
                .build());
        }
        Err(e) => return Err(e.into()),
    };

    let record = match table.get(&name)? {
        Some(record) => record,
        None => {
            return Err(Error::builder()
                .not_found()
                .message("Secret not found")
                .build());
        }
    };
    let item = record.value().0;

    Ok(Json(Secret {
        name: item.name,
        created_at: item.created_at,
    }))
}

async fn delete_secret(
    Path((store_id, name)): Path<(String, String)>,
    State(ctx): State<Context>,
) -> Result<()> {
    let tx = ctx.db.begin_write()?;

    {
        let definition = TableDefinition::new(&store_id);

        let mut table = tx.open_table(definition)?;

        table.remove(&name)?;
    }

    tx.commit()?;
    ctx.reload.notify_one();

    Ok(())
}
