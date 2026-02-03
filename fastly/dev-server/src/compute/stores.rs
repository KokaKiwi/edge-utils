use std::sync::Arc;

use redb::{Database, ReadTransaction, ReadableDatabase, ReadableTable};
use viceroy_lib::ExecuteCtxBuilder;

use crate::tables::{
    ConfigStoreMetadata, ConfigStoreTable, KVStoreMetadata, KVStoreTable, METADATA_TABLE,
    SecretStoreMetadata, SecretStoreTable,
};

pub fn init_stores(
    db: &Database,
    builder: ExecuteCtxBuilder,
) -> Result<ExecuteCtxBuilder, redb::Error> {
    let tx = db.begin_read().unwrap();

    let Some(metadata_table) = open_table(&tx, METADATA_TABLE)? else {
        return Ok(builder);
    };
    let Some(metadata_record) = metadata_table.get(&())? else {
        return Ok(builder);
    };
    let metadata = &metadata_record.value().0;

    let config_stores: Vec<_> = metadata
        .config_stores
        .iter()
        .map(|(name, meta)| (name.as_str(), meta))
        .collect();
    let builder = init_config_stores(builder, &tx, &config_stores)?;

    let kv_stores: Vec<_> = metadata
        .kv_stores
        .iter()
        .map(|(name, meta)| (name.as_str(), meta))
        .collect();
    let builder = init_kv_stores(builder, &tx, &kv_stores)?;

    let secret_stores: Vec<_> = metadata
        .secret_stores
        .iter()
        .map(|(name, meta)| (name.as_str(), meta))
        .collect();
    let builder = init_secret_stores(builder, &tx, &secret_stores)?;

    Ok(builder)
}

fn init_config_stores(
    builder: ExecuteCtxBuilder,
    tx: &ReadTransaction,
    stores: &[(&str, &ConfigStoreMetadata)],
) -> Result<ExecuteCtxBuilder, redb::Error> {
    use viceroy_lib::config::{Dictionaries, Dictionary};

    let mut dictionaries = Dictionaries::default();

    for (id, meta) in stores {
        let table_def = ConfigStoreTable::new(id);
        let Some(table) = open_table(tx, table_def)? else {
            continue;
        };

        let data = table
            .iter()?
            .filter_map(|res| res.ok())
            .map(|(key, entry)| {
                let key = key.value().clone();
                let value = entry.value().0.item_value.clone();
                (key, value)
            })
            .collect();
        let dictionary = Dictionary::InlineToml {
            contents: Arc::new(data),
        };
        dictionaries.insert(id.to_string(), dictionary.clone());
        dictionaries.insert(meta.name.clone(), dictionary);
    }

    Ok(builder.with_dictionaries(dictionaries))
}

fn init_kv_stores(
    builder: ExecuteCtxBuilder,
    tx: &ReadTransaction,
    stores: &[(&str, &KVStoreMetadata)],
) -> Result<ExecuteCtxBuilder, redb::Error> {
    use viceroy_lib::config::{ObjectKey, ObjectStoreKey, ObjectStores};
    use viceroy_lib::wiggle_abi::types::KvInsertMode;

    let object_stores = ObjectStores::default();

    for (id, meta) in stores {
        let table_def = KVStoreTable::new(id);
        let Some(table) = open_table(tx, table_def)? else {
            continue;
        };

        let entries = table.iter()?.filter_map(|res| res.ok());
        for entry in entries {
            let (key, record) = entry;
            let key = key.value().clone();
            let value = record.value().0.value.clone();

            let store_key = ObjectStoreKey::new(id.to_string());
            let store_alias_key = ObjectStoreKey::new(meta.name.clone());
            let object_key = ObjectKey::new(key).expect("Invalid KV store key");

            object_stores
                .insert(
                    store_key,
                    object_key.clone(),
                    value.to_vec(),
                    KvInsertMode::Overwrite,
                    Some(1),
                    None,
                    None,
                )
                .expect("Failed to insert into object store");
            object_stores
                .insert(
                    store_alias_key,
                    object_key.clone(),
                    value.to_vec(),
                    KvInsertMode::Overwrite,
                    Some(1),
                    None,
                    None,
                )
                .expect("Failed to insert into object store");
        }
    }

    Ok(builder.with_object_stores(object_stores))
}

fn init_secret_stores(
    builder: ExecuteCtxBuilder,
    tx: &ReadTransaction,
    stores: &[(&str, &SecretStoreMetadata)],
) -> Result<ExecuteCtxBuilder, redb::Error> {
    use viceroy_lib::config::{SecretStore, SecretStores};

    let mut secret_stores = SecretStores::default();

    for (id, meta) in stores {
        let table_def = SecretStoreTable::new(id);
        let Some(table) = open_table(tx, table_def)? else {
            continue;
        };

        let mut secret_store = SecretStore::new();

        for entry in table.iter()?.filter_map(|res| res.ok()) {
            let (key, record) = entry;
            let key = key.value().clone();
            let value = record.value().0.secret.clone();
            secret_store.add_secret(key, value);
        }
        secret_stores.add_store(id.to_string(), secret_store.clone());
        secret_stores.add_store(meta.name.clone(), secret_store);
    }

    Ok(builder.with_secret_stores(secret_stores))
}

fn open_table<K: redb::Key, V: redb::Value>(
    tx: &ReadTransaction,
    table_def: redb::TableDefinition<K, V>,
) -> Result<Option<redb::ReadOnlyTable<K, V>>, redb::Error> {
    match tx.open_table(table_def) {
        Ok(table) => Ok(Some(table)),
        Err(redb::TableError::TableDoesNotExist(_)) => Ok(None),
        Err(err) => Err(err.into()),
    }
}
