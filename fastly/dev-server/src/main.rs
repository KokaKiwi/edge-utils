use std::net::SocketAddr;
use std::path::PathBuf;

use clap::Parser;
use miette::{IntoDiagnostic, Result};

mod api;
mod compute;
mod context;
mod tables;
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
    use std::time::Duration;

    use tokio_graceful_shutdown::{SubsystemBuilder, SubsystemHandle, Toplevel};

    let opts = Options::parse();

    let _guard = trace::setup_tracing();

    let db = context::open_db(&opts.store_path)?;

    Toplevel::new(async move |s: &mut SubsystemHandle| {
        let api_subsys = SubsystemBuilder::new("api", {
            let db = db.clone();
            let listen_addr = opts.api_addr;

            async move |subsys: &mut SubsystemHandle| api::run(subsys, db, listen_addr).await
        });
        s.start(api_subsys);

        let compute_subsys = SubsystemBuilder::new("compute", {
            let db = db.clone();
            let module_path = opts.file.to_path_buf();
            let listen_addr = opts.http_addr;

            async move |subsys: &mut SubsystemHandle| {
                compute::run(subsys, db, module_path, listen_addr).await
            }
        });
        s.start(compute_subsys);
    })
    .catch_signals()
    .handle_shutdown_requests(Duration::from_millis(1000))
    .await
    .into_diagnostic()
}
