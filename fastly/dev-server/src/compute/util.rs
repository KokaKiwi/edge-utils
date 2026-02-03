use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;
use std::task;

use axum::body::Body as AxumBody;
use http::{Request, Response};
use hyper014::Body as Hyper014Body;
use tower::{Layer, Service};
use viceroy_lib::{ExecuteCtx, body::Body as ViceroyBody};

use super::compat;

pub struct ViceroyCompatLayer;

impl<S> Layer<S> for ViceroyCompatLayer {
    type Service = ViceroyCompat<S>;

    fn layer(&self, inner: S) -> Self::Service {
        ViceroyCompat { inner }
    }
}

#[derive(Clone)]
pub struct ViceroyCompat<S> {
    inner: S,
}

impl<S> Service<Request<AxumBody>> for ViceroyCompat<S>
where
    S: Service<Request<Hyper014Body>, Response = Response<ViceroyBody>>,
{
    type Response = Response<AxumBody>;
    type Error = S::Error;
    type Future = ViceroyCompatFuture<S::Future>;

    fn poll_ready(&mut self, cx: &mut task::Context<'_>) -> task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<AxumBody>) -> Self::Future {
        let req = req.map(compat::axum_body_to_hyper014);
        let fut = self.inner.call(req);
        ViceroyCompatFuture { inner: fut }
    }
}

#[pin_project::pin_project]
pub struct ViceroyCompatFuture<Fut> {
    #[pin]
    inner: Fut,
}

impl<Fut, E> Future for ViceroyCompatFuture<Fut>
where
    Fut: Future<Output = Result<Response<ViceroyBody>, E>>,
{
    type Output = Result<Response<AxumBody>, E>;

    fn poll(self: Pin<&mut Self>, cx: &mut task::Context<'_>) -> task::Poll<Self::Output> {
        let this = self.project();

        let resp = task::ready!(this.inner.poll(cx))?;
        let resp = resp.map(compat::viceroy_body_to_axum);

        task::Poll::Ready(Ok(resp))
    }
}

#[derive(Clone)]
pub struct ViceroyService {
    exec_ctx: Arc<ExecuteCtx>,
    local_addr: SocketAddr,
    remote_addr: SocketAddr,
}

impl ViceroyService {
    pub fn new(exec_ctx: Arc<ExecuteCtx>, local_addr: SocketAddr, remote_addr: SocketAddr) -> Self {
        Self {
            exec_ctx,
            local_addr,
            remote_addr,
        }
    }
}

impl Service<Request<Hyper014Body>> for ViceroyService {
    type Response = Response<ViceroyBody>;
    type Error = viceroy_lib::error::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut task::Context<'_>) -> task::Poll<Result<(), Self::Error>> {
        task::Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request<Hyper014Body>) -> Self::Future {
        let exec_ctx = self.exec_ctx.clone();
        let local_addr = self.local_addr;
        let remote_addr = self.remote_addr;

        let req = compat::axum_request_to_hyper014(req);

        Box::pin(async move {
            let resp = exec_ctx
                .handle_request_with_runtime_error(req, local_addr, remote_addr)
                .await?;

            Ok(compat::hyper014_response_to_axum(resp))
        })
    }
}
