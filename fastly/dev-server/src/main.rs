use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use clap::Parser;
use miette::Result;
use tokio::sync::Notify;

mod api;
mod compute;
mod context;
mod trace;
mod util;

#[derive(Debug, Parser)]
struct Options {
    /// Path to the Wasm file to run
    pub file: PathBuf,

    /// Path to the persistent store
    #[clap(long, default_value = "./fastly-dev-store.db")]
    pub store_path: PathBuf,

    /// Address to bind the HTTP server to
    #[clap(long, default_value = "127.0.0.1:7676")]
    pub http_addr: SocketAddr,
    /// Address to bind the API server to
    #[clap(long, default_value = "127.0.0.1:7677")]
    pub api_addr: SocketAddr,
}

#[tokio::main]
async fn main() -> Result<()> {
    let opts = Options::parse();

    let _guard = trace::setup_tracing();

    let db = context::open_db(&opts.store_path)?;
    let reload = Arc::new(Notify::new());

    let api = tokio::spawn(api::run(db.clone(), reload.clone(), opts.api_addr));
    let compute = tokio::spawn(compute::run(db.clone(), reload.clone(), opts.http_addr));

    let _ = tokio::try_join!(api, compute).expect("Tasks failed");

    Ok(())
}
