use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::util::JsonRecord;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ConfigStoreItemMetadata {
    pub item_value: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub type TableDefinition<'a> =
    redb::TableDefinition<'a, String, JsonRecord<ConfigStoreItemMetadata>>;
