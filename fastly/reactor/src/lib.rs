use std::cell::RefCell;
use std::collections::VecDeque;
use std::future::Future;
use std::pin::pin;
use std::task::{Context, Poll, Waker};

use async_task::{Runnable, Task};

#[cfg(feature = "macros")]
pub use fastly_reactor_macros::main;

pub use reactor::with_reactor;

mod reactor;

thread_local! {
    static QUEUE: RefCell<VecDeque<Runnable>> = const { RefCell::new(VecDeque::new()) };
}

fn schedule(runnable: Runnable) {
    QUEUE.with(|q| q.borrow_mut().push_back(runnable));
}

/// Spawn a future onto the local executor. Returns a Task that can be awaited.
pub fn spawn<F, T>(fut: F) -> Task<T>
where
    F: Future<Output = T> + 'static,
    T: 'static,
{
    let (runnable, task) = async_task::spawn_local(fut, schedule);
    schedule(runnable);
    task
}

/// Run a future to completion, driving I/O with the reactor.
pub fn block_on<F: Future + 'static>(fut: F) -> F::Output
where
    F::Output: 'static,
{
    let (runnable, task) = async_task::spawn_local(fut, schedule);
    schedule(runnable);
    let mut task = pin!(task);

    loop {
        // Phase 1: drain all ready runnables
        loop {
            let r = QUEUE.with(|q| q.borrow_mut().pop_front());
            match r {
                Some(r) => {
                    r.run();
                }
                None => break,
            }
        }

        // Phase 2: check if the root task completed
        let noop = Waker::noop();
        let mut cx = Context::from_waker(noop);
        if let Poll::Ready(val) = task.as_mut().poll(&mut cx) {
            return val;
        }

        // Phase 3: block on I/O reactor — wakes a registered future
        let had_handles = reactor::with_reactor(|r| r.wait());
        if !had_handles {
            panic!("block_on: deadlock — no runnables and no I/O handles registered");
        }
    }
}
