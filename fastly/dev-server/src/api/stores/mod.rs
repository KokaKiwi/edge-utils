use super::Router;

mod config;
mod kv;
mod secret;

pub fn router() -> Router {
    Router::new()
        .nest("/config", config::router())
        .nest("/kv", kv::router())
        .nest("/secret", secret::router())
}
