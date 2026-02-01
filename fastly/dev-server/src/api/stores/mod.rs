use std::collections::HashMap;

use redb::TableDefinition;
use serde::{Deserialize, Serialize};

use super::Router;
use crate::util::JsonRecord;

mod config;
mod kv;
mod secret;

const METADATA_TABLE: TableDefinition<(), JsonRecord<Metadata>> = TableDefinition::new("__meta__");

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct Metadata {
    #[serde(default)]
    pub config_stores: HashMap<String, config::ConfigStoreMetadata>,
    #[serde(default)]
    pub kv_stores: HashMap<String, kv::KVStoreMetadata>,
    #[serde(default)]
    pub secret_stores: HashMap<String, secret::SecretStoreMetadata>,
}

pub fn router() -> Router {
    Router::new()
        .nest("/config", config::router())
        .nest("/kv", kv::router())
        .nest("/secret", secret::router())
}
