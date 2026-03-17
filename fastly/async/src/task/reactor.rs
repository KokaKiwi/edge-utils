use std::cell::RefCell;
use std::collections::HashMap;
use std::task::Waker;

#[derive(Debug, Default)]
pub struct Reactor {
    /// Maps handle value → waker to fire when ready
    registry: HashMap<u32, Waker>,
    /// Parallel vec of handles for passing to fastly_async_io::select
    handles: Vec<u32>,
}

impl Reactor {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register or update the waker for a handle.
    /// If already registered, the waker is replaced (always update).
    pub fn register(&mut self, handle: u32, waker: Waker) {
        if self.registry.insert(handle, waker).is_none() {
            // Only push to handles vec on first registration
            self.handles.push(handle);
        }
    }

    /// Deregister a handle (e.g. when the future is dropped before completion).
    pub fn unregister(&mut self, handle: u32) {
        if self.registry.remove(&handle).is_some()
            && let Some(pos) = self.handles.iter().position(|&h| h == handle)
        {
            self.handles.swap_remove(pos);
        }
    }

    /// Block until one handle is ready, fire its waker, return true.
    /// Returns false immediately if no handles are registered.
    pub fn wait(&mut self) -> bool {
        if self.handles.is_empty() {
            return false;
        }

        let mut done_index: u32 = 0;

        // SAFETY: handles is a valid Vec<u32>, done_index is stack-allocated.
        // fastly_async_io::select blocks until one handle is ready.
        let status = unsafe {
            fastly_sys::fastly_async_io::select(
                self.handles.as_ptr(),
                self.handles.len(),
                0, // timeout_ms = 0 means no timeout (block indefinitely)
                &raw mut done_index,
            )
        };
        assert_eq!(status, fastly_shared::FastlyStatus::OK, "select failed");

        let ready_handle = self.handles.swap_remove(done_index as usize);
        if let Some(waker) = self.registry.remove(&ready_handle) {
            waker.wake();
        }

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
