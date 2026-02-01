use std::path::Path;
use std::sync::Arc;

use miette::{IntoDiagnostic, Result};
use redb::Database;

pub fn open_db(db_path: &Path) -> Result<Arc<Database>> {
    let mut db = Database::create(db_path).into_diagnostic()?;

    if db.compact().into_diagnostic()? {
        tracing::info!("Database was compacted");
    }

    Ok(Arc::new(db))
}
