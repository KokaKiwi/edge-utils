use std::path::PathBuf;

use clap::Parser;
use miette::Result;

mod api;
mod cli;
mod compute;
mod context;
mod tables;
mod trace;
mod util;

#[derive(Debug, Parser)]
struct Options {
    /// Path to the persistent store
    #[clap(
        long,
        default_value = "./fastly-dev-store.db",
        env = "FASTLY_DEV_SERVER_STORE_PATH"
    )]
    pub store_path: PathBuf,

    #[clap(subcommand)]
    pub command: cli::Command,
}

#[tokio::main]
async fn main() -> Result<()> {
    let opts = Options::parse();

    let _guard = trace::setup_tracing();

    let db = context::open_db(&opts.store_path)?;

    cli::run(opts.command, db).await
}
