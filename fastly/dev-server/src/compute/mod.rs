use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use miette::IntoDiagnostic;
use redb::Database;
use tokio_graceful_shutdown::SubsystemHandle;
use viceroy_lib::{ExecuteCtx, ProfilingStrategy};

mod compat;
mod stores;
mod util;

pub async fn run(
    subsys: &mut SubsystemHandle,
    db: Arc<Database>,
    module_path: PathBuf,
    listen_addr: SocketAddr,
) -> miette::Result<()> {
    use axum::serve::IncomingStream;
    use tokio::net::TcpListener;

    let exec_ctx = ExecuteCtx::build(
        module_path,
        ProfilingStrategy::None,
        Default::default(),
        None,
        Default::default(),
        false,
    )
    .into_diagnostic()?
    .finish();

    let make_service = tower::service_fn(move |stream: IncomingStream<TcpListener>| {
        let exec_ctx = {
            let builder = exec_ctx.new_instance();
            let builder = stores::init_stores(&db, builder).expect("Failed to initialize stores");

            Arc::new(builder.finish())
        };

        let local_addr = listen_addr;
        let remote_addr = *stream.remote_addr();

        async move {
            use axum::error_handling::HandleErrorLayer;
            use tower_http::trace::TraceLayer;

            use crate::util::OtelTrace;

            let trace_layer = TraceLayer::new_for_http()
                .make_span_with(OtelTrace)
                .on_response(OtelTrace)
                .on_body_chunk(())
                .on_eos(())
                .on_failure(OtelTrace);

            let viceroy_service = util::ViceroyService::new(exec_ctx, local_addr, remote_addr);

            let service = tower::ServiceBuilder::new()
                .layer(HandleErrorLayer::new(async |err| {
                    (
                        http::StatusCode::INTERNAL_SERVER_ERROR,
                        format!("Internal server error: {err}"),
                    )
                }))
                .layer(trace_layer)
                .layer(util::ViceroyCompatLayer)
                .service(viceroy_service);

            Ok(service)
        }
    });

    let listener = TcpListener::bind(listen_addr).await.into_diagnostic()?;
    tracing::info!("Compute server listening on {listen_addr}");

    let cancel = subsys.create_cancellation_token();

    axum::serve(listener, make_service)
        .with_graceful_shutdown(cancel.cancelled_owned())
        .await
        .into_diagnostic()
}
