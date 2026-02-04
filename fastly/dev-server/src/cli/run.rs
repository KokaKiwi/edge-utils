use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use miette::IntoDiagnostic;
use redb::Database;

#[derive(Debug, clap::Parser)]
pub struct Options {
    /// Path to the Wasm file to run
    pub file: PathBuf,

    /// Address to bind the HTTP server to
    #[clap(
        long,
        default_value = "127.0.0.1:7676",
        env = "FASTLY_DEV_SERVER_HTTP_ADDR"
    )]
    pub http_addr: SocketAddr,
    /// Address to bind the API server to
    #[clap(
        long,
        default_value = "127.0.0.1:7677",
        env = "FASTLY_DEV_SERVER_API_ADDR"
    )]
    pub api_addr: SocketAddr,
}

pub async fn run(opts: Options, db: Arc<Database>) -> miette::Result<()> {
    use std::time::Duration;

    use tokio_graceful_shutdown::{SubsystemBuilder, SubsystemHandle, Toplevel};

    Toplevel::new(async move |s: &mut SubsystemHandle| {
        let api_subsys = SubsystemBuilder::new("api", {
            let db = db.clone();
            let listen_addr = opts.api_addr;

            async move |subsys: &mut SubsystemHandle| crate::api::run(subsys, db, listen_addr).await
        });
        s.start(api_subsys);

        let compute_subsys = SubsystemBuilder::new("compute", {
            let db = db.clone();
            let module_path = opts.file.to_path_buf();
            let listen_addr = opts.http_addr;

            async move |subsys: &mut SubsystemHandle| {
                crate::compute::run(subsys, db, module_path, listen_addr).await
            }
        });
        s.start(compute_subsys);
    })
    .catch_signals()
    .handle_shutdown_requests(Duration::from_millis(1000))
    .await
    .into_diagnostic()
}
