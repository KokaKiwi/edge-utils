use std::cell::RefCell;
use std::collections::HashMap;
use std::task::Waker;

use fastly::handle::{BodyHandle, PendingRequestHandle, ResponseHandle, select_handles};
use fastly::http::request::SendErrorCause;
use ordermap::OrderMap;

#[derive(Default)]
pub struct Reactor {
    /// Wakers for pending handles, keyed by handle id.
    entries: OrderMap<u32, Waker>,
    /// Results for completed handles, keyed by handle id.
    results: HashMap<u32, Result<(ResponseHandle, BodyHandle), SendErrorCause>>,
}

impl Reactor {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a pending handle id with the reactor, or refresh its waker if already present.
    pub fn register_pending_request(&mut self, handle: &PendingRequestHandle, waker: Waker) {
        #[cfg_attr(not(target_env = "p1"), allow(deprecated))]
        let handle_id = handle.as_u32();
        self.entries.insert(handle_id, waker);
    }

    /// Deregister a handle by id (e.g. when the future is dropped before completion).
    pub fn unregister_pending_request(&mut self, handle: &PendingRequestHandle) {
        #[cfg_attr(not(target_env = "p1"), allow(deprecated))]
        let handle_id = handle.as_u32();
        self.entries.swap_remove(&handle_id);
        self.results.remove(&handle_id);
    }

    /// Take the stored result for a completed handle.
    pub fn take_result(
        &mut self,
        handle: &PendingRequestHandle,
    ) -> Option<Result<(ResponseHandle, BodyHandle), SendErrorCause>> {
        #[cfg_attr(not(target_env = "p1"), allow(deprecated))]
        let handle_id = handle.as_u32();
        self.results.remove(&handle_id)
    }

    /// Block until one handle is ready, store its result, fire its waker, return true.
    /// Returns false immediately if no handles are registered.
    pub fn wait(&mut self) -> bool {
        if self.entries.is_empty() {
            return false;
        }

        let handles: Vec<PendingRequestHandle> = self
            .entries
            .keys()
            .copied()
            .map(PendingRequestHandle::from_u32)
            .collect();

        let (result, ready_index, _remaining) = select_handles(handles);
        let (ready_id, ready_waker) = self.entries.remove_index(ready_index).unwrap();

        // Store the result and wake the corresponding future.
        self.results.insert(ready_id, result);
        ready_waker.wake();

        true
    }
}

thread_local! {
    static REACTOR: RefCell<Reactor> = RefCell::new(Reactor::new());
}

pub fn with_reactor<F, R>(f: F) -> R
where
    F: FnOnce(&mut Reactor) -> R,
{
    REACTOR.with(|r| f(&mut r.borrow_mut()))
}
