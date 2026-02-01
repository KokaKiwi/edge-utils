use bytes::Bytes;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::util::JsonRecord;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KVStoreItemMetadata {
    #[serde(with = "serde_with::As::<serde_with::base64::Base64>")]
    pub value: Bytes,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub type TableDefinition<'a> = redb::TableDefinition<'a, String, JsonRecord<KVStoreItemMetadata>>;
