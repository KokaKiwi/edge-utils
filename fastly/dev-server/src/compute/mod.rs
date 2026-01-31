use std::net::SocketAddr;

use crate::context::Context;

#[bon::builder]
pub async fn run(ctx: Context, listen_addr: SocketAddr) {}
