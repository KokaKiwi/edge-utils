use std::net::SocketAddr;
use std::sync::Arc;

use redb::Database;
use tokio::sync::Notify;

mod error;
mod stores;

#[derive(Clone)]
struct Context {
    pub db: Arc<Database>,
    pub reload: Arc<Notify>,
}

type Result<T> = std::result::Result<T, error::Error>;
type Router = axum::Router<Context>;

pub async fn run(db: Arc<Database>, reload: Arc<Notify>, listen_addr: SocketAddr) {
    use tokio::net::TcpListener;

    let ctx = Context { db, reload };

    let app = router().with_state(ctx);

    let listener = TcpListener::bind(listen_addr)
        .await
        .expect("Failed to bind API server");
    tracing::info!("API server listening on {listen_addr}");

    axum::serve(listener, app)
        .with_graceful_shutdown(crate::util::shutdown_signal())
        .await
        .expect("API server failed");
}

fn router() -> Router {
    use tower_http::trace::TraceLayer;

    use crate::util::OtelTrace;

    let trace_layer = TraceLayer::new_for_http()
        .make_span_with(OtelTrace)
        .on_response(OtelTrace)
        .on_body_chunk(())
        .on_eos(())
        .on_failure(OtelTrace);

    Router::new()
        .nest("/resources/stores", stores::router())
        .layer(trace_layer)
}
