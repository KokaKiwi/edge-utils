use std::path::Path;
use std::sync::Arc;

use miette::{IntoDiagnostic, Result};
use redb::Database;

#[derive(Clone)]
pub struct Context {
    pub db: Arc<Database>,
}

impl Context {
    pub fn new(db_path: &Path) -> Result<Self> {
        let mut db = {
            let builder = Database::builder();

            builder.create(db_path).into_diagnostic()?
        };
        let _ = db.check_integrity().into_diagnostic()?;

        Ok(Self { db: Arc::new(db) })
    }
}
