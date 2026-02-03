use std::collections::HashMap;

use bytes::Bytes;
use chrono::{DateTime, Utc};
use redb::TableDefinition;
use serde::{Deserialize, Serialize};

use crate::util::JsonRecord;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Metadata {
    #[serde(default)]
    pub config_stores: HashMap<String, ConfigStoreMetadata>,
    #[serde(default)]
    pub kv_stores: HashMap<String, KVStoreMetadata>,
    #[serde(default)]
    pub secret_stores: HashMap<String, SecretStoreMetadata>,
}

pub type MetaDataTable<'a> = TableDefinition<'a, (), JsonRecord<Metadata>>;

pub const METADATA_TABLE: MetaDataTable = TableDefinition::new("__meta__");

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ConfigStoreMetadata {
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ConfigStoreItemMetadata {
    pub item_value: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub type ConfigStoreTable<'a> = TableDefinition<'a, String, JsonRecord<ConfigStoreItemMetadata>>;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct KVStoreMetadata {
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KVStoreItemMetadata {
    #[serde(with = "serde_with::As::<serde_with::base64::Base64>")]
    pub value: Bytes,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub type KVStoreTable<'a> = TableDefinition<'a, String, JsonRecord<KVStoreItemMetadata>>;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SecretStoreMetadata {
    pub name: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretStoreItemMetadata {
    pub name: String,
    #[serde(with = "serde_with::As::<serde_with::base64::Base64>")]
    pub secret: Bytes,
    pub created_at: DateTime<Utc>,
}

pub type SecretStoreTable<'a> = TableDefinition<'a, String, JsonRecord<SecretStoreItemMetadata>>;
