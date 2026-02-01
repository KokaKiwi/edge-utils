use std::net::SocketAddr;
use std::sync::Arc;

use redb::Database;
use tokio::sync::Notify;

pub async fn run(_db: Arc<Database>, reload: Arc<Notify>, _listen_addr: SocketAddr) {
    loop {
        tokio::select! {
            _ = reload.notified() => {
                // Reload the compute server
            }
            _ = tokio::signal::ctrl_c() => {
                break;
            }
        }
    }
}
