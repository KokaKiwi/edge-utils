use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::Router;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SecretStoreMetadata {
    name: String,
    created_at: DateTime<Utc>,
}

pub fn router() -> Router {
    Router::new()
}
