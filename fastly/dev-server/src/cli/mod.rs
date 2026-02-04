use std::sync::Arc;

use redb::Database;

mod run;

#[derive(Debug, clap::Parser)]
pub enum Command {
    /// Run the Fastly dev server
    Run(run::Options),
}

pub async fn run(cmd: Command, db: Arc<Database>) -> miette::Result<()> {
    match cmd {
        Command::Run(opts) => run::run(opts, db).await,
    }
}
