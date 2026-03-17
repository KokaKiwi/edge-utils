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
    /// The pending request handle, if still pending. Set to `None` after completion.
    handle: Option<PendingRequestHandle>,
}

impl PendingResponse {
    fn new(handle: PendingRequestHandle) -> Self {
        Self {
            handle: Some(handle),
        }
    }
}

impl Future for PendingResponse {
    type Output = Result<Response, SendErrorCause>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        use fastly::handle::PollHandleResult;

        let this = self.get_mut();
        let handle = this.handle.take().expect("polled after completion");

        match handle.poll() {
            PollHandleResult::Done(result) => {
                Poll::Ready(result.map(|(resp_handle, body_handle)| {
                    Response::from_handles(resp_handle, body_handle)
                }))
            }
            PollHandleResult::Pending(handle) => {
                #[cfg_attr(not(target_env = "p1"), allow(deprecated))]
                with_reactor(|r| r.register(handle.as_u32(), cx.waker().clone()));
                this.handle = Some(handle);
                Poll::Pending
            }
        }
    }
}

impl Drop for PendingResponse {
    fn drop(&mut self) {
        if let Some(ref handle) = self.handle {
            #[cfg_attr(not(target_env = "p1"), allow(deprecated))]
            with_reactor(|r| r.unregister(handle.as_u32()));
        }
    }
}

impl From<PendingRequestHandle> for PendingResponse {
    fn from(handle: PendingRequestHandle) -> Self {
        Self::new(handle)
    }
}

impl From<PendingRequest> for PendingResponse {
    fn from(pending: PendingRequest) -> Self {
        Self::new(pending.into())
    }
}
