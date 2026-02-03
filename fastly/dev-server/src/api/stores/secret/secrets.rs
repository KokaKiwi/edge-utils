use axum::extract::{Json, Path, State};
use bytes::Bytes;
use chrono::{DateTime, Utc};
use redb::{ReadableDatabase, ReadableTable};
use serde::{Deserialize, Serialize};

use crate::api::{Context, Result, Router, error::Error};
use crate::tables::{SecretStoreItemMetadata, SecretStoreTable as TableDefinition};
use crate::util::JsonRecord;

pub fn router() -> Router {
    use axum::routing;

    Router::new()
        .route(
            "/{store_id}/secrets",
            routing::get(list_secrets)
                .post(create_secret)
                .put(recreate_secret),
        )
        .route(
            "/{store_id}/secrets/{secret_name}",
            routing::get(get_secret).delete(delete_secret),
        )
}

#[derive(Debug, Clone, Serialize)]
struct Secret {
    name: String,
    #[serde(with = "serde_with::As::<serde_with::base64::Base64>")]
    digest: Bytes,
    created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Default, Serialize)]
struct SecretListResponse {
    data: Vec<Secret>,
}

async fn list_secrets(
    Path(store_id): Path<String>,
    State(ctx): State<Context>,
) -> Result<Json<SecretListResponse>> {
    let tx = ctx.db.begin_read()?;

    let definition = TableDefinition::new(&store_id);

    let table = match tx.open_table(definition) {
        Ok(table) => table,
        Err(redb::TableError::TableDoesNotExist(_)) => {
            return Ok(Json(SecretListResponse::default()));
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
                digest: compute_digest(&item.secret),
                created_at: item.created_at,
            }
        })
        .collect::<Vec<Secret>>();

    Ok(Json(SecretListResponse { data: entries }))
}

#[derive(Debug, Clone, Deserialize)]
struct CreateSecretRequest {
    name: String,
    secret: String,
}

async fn create_secret(
    Path(store_id): Path<String>,
    State(ctx): State<Context>,
    Json(payload): Json<CreateSecretRequest>,
) -> Result<Json<Secret>> {
    let tx = ctx.db.begin_write()?;

    let secret = {
        let definition = TableDefinition::new(&store_id);

        let mut table = tx.open_table(definition)?;

        // Check if secret already exists
        if table.get(&payload.name)?.is_some() {
            return Err(Error::builder()
                .conflict()
                .message("Secret with this name already exists. Use PUT to recreate it.")
                .build());
        }

        let now = Utc::now();

        let meta = SecretStoreItemMetadata {
            name: payload.name.clone(),
            secret: Bytes::from(payload.secret.as_bytes().to_vec()),
            created_at: now,
        };

        table.insert(&payload.name, &JsonRecord(meta.clone()))?;

        Secret {
            name: payload.name,
            digest: compute_digest(&meta.secret),
            created_at: meta.created_at,
        }
    };

    tx.commit()?;

    Ok(Json(secret))
}

async fn recreate_secret(
    Path(store_id): Path<String>,
    State(ctx): State<Context>,
    Json(payload): Json<CreateSecretRequest>,
) -> Result<Json<Secret>> {
    let tx = ctx.db.begin_write()?;

    let secret = {
        let definition = TableDefinition::new(&store_id);

        let mut table = tx.open_table(definition)?;

        let now = Utc::now();

        let meta = SecretStoreItemMetadata {
            name: payload.name.clone(),
            secret: Bytes::from(payload.secret.as_bytes().to_vec()),
            created_at: now,
        };

        table.insert(&payload.name, &JsonRecord(meta.clone()))?;

        Secret {
            name: payload.name,
            digest: compute_digest(&meta.secret),
            created_at: meta.created_at,
        }
    };

    tx.commit()?;

    Ok(Json(secret))
}

async fn get_secret(
    Path((store_id, secret_name)): Path<(String, String)>,
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

    let record = match table.get(&secret_name)? {
        Some(record) => record,
        None => {
            return Err(Error::builder()
                .not_found()
                .message("Secret not found")
                .build());
        }
    };
    let item = record.value().0;

    let secret = Secret {
        name: secret_name,
        digest: compute_digest(&item.secret),
        created_at: item.created_at,
    };

    Ok(Json(secret))
}

async fn delete_secret(
    Path((store_id, secret_name)): Path<(String, String)>,
    State(ctx): State<Context>,
) -> Result<()> {
    let tx = ctx.db.begin_write()?;

    {
        let definition = TableDefinition::new(&store_id);

        let mut table = tx.open_table(definition)?;

        if table.remove(&secret_name)?.is_none() {
            return Err(Error::builder()
                .not_found()
                .message("Secret not found")
                .build());
        }
    }

    tx.commit()?;

    Ok(())
}

/// Compute a digest (opaque identifier) of the plaintext secret value
/// This allows determining if a secret value has changed without exposing the plaintext
fn compute_digest(secret: &Bytes) -> Bytes {
    use sha2::{Digest, Sha256};

    let mut hasher = Sha256::new();
    hasher.update(secret);
    let result = hasher.finalize();

    // Return as bytes
    Bytes::from(result.to_vec())
}
