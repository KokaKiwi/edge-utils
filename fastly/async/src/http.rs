use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use fastly::handle::PendingRequestHandle;
use fastly::http::{
    Response,
    request::{PendingRequest, SendErrorCause},
};

use crate::task::with_reactor;

/// A future that resolves when a backend HTTP response arrives.
pub struct PendingResponse {
    /// The pending handle, or `None` once the result has been consumed.
    handle: Option<PendingRequestHandle>,
}

impl Future for PendingResponse {
    type Output = Result<Response, SendErrorCause>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        let handle = this.handle.take().expect("polled after completion");

        // Check if the reactor already has a result from a previous select_handles call.
        if let Some(result) = with_reactor(|r| r.take_result(&handle)) {
            return Poll::Ready(match result {
                Ok((r, b)) => Ok(Response::from_handles(r, b)),
                Err(e) => Err(e),
            });
        }

        // Register or refresh the waker so the reactor can wake us.
        with_reactor(|r| r.register_pending_request(&handle, cx.waker().clone()));
        this.handle = Some(handle);
        Poll::Pending
    }
}

impl Drop for PendingResponse {
    fn drop(&mut self) {
        if let Some(ref handle) = self.handle {
            with_reactor(|r| r.unregister_pending_request(handle));
        }
    }
}

impl From<PendingRequestHandle> for PendingResponse {
    fn from(handle: PendingRequestHandle) -> Self {
        Self {
            handle: Some(handle),
        }
    }
}

impl From<PendingRequest> for PendingResponse {
    fn from(pending: PendingRequest) -> Self {
        PendingRequestHandle::from(pending).into()
    }
}
