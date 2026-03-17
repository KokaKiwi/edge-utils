# fastly-async

Async/await support for [Fastly Compute](https://www.fastly.com/products/edge-cloud-network/compute) edge functions.

Fastly Compute runs WebAssembly handlers synchronously — there is no built-in async runtime. This crate provides a lightweight single-threaded executor and an I/O reactor backed by `fastly_async_io::select`, enabling standard `async`/`await` code and concurrent backend requests with `futures::join!` / `futures::try_join!`.

## Usage

Add the dependency with the `macros` feature to use the `#[fastly_async::main]` entry-point attribute:

```toml
[dependencies]
fastly-async = { version = "0.1", features = ["macros"] }
```

### Entry point

```rust
use fastly::{Request, Response};

#[fastly_async::main]
async fn main(req: Request) -> Response {
    // async handler body
}
```

The macro rewrites the function into the synchronous `fn main()` expected by Fastly Compute, wrapping the body in `fastly_async::task::block_on`.

### Concurrent backend requests

`fastly_async::http::PendingResponse` wraps a `PendingRequestHandle` as a `Future`. Multiple in-flight requests can be awaited concurrently:

```rust
use fastly::{Request, Response};
use fastly_async::http::PendingResponse;

#[fastly_async::main]
async fn main(_req: Request) -> Response {
    let a = PendingResponse::from(
        Request::get("https://example.com/a")
            .send_async("backend")
            .expect("send failed"),
    );
    let b = PendingResponse::from(
        Request::get("https://example.com/b")
            .send_async("backend")
            .expect("send failed"),
    );

    let (res_a, res_b) = futures::try_join!(a, b).expect("request failed");

    Response::from_status(200)
        .with_body_text_plain(&format!("{} {}\n", res_a.get_status(), res_b.get_status()))
}
```

## How it works

The executor is a `thread_local` run queue (`VecDeque<Runnable>`) driven by `block_on`. Each iteration of the event loop:

1. **Drain** — run all ready tasks until the queue is empty.
2. **Check** — if the root task is complete, return its value.
3. **Wait** — call `fastly_async_io::select` to block until one pending handle becomes ready, then wake its registered future.

The reactor (`task::reactor`) maps handle IDs (`u32`) to `Waker`s. `PendingResponse` registers its handle on each `Poll::Pending` and deregisters it on drop, so cancelled futures clean up correctly.

## Feature flags

| Feature | Default | Description |
|---------|---------|-------------|
| `macros` | no | Re-exports `#[fastly_async::main]` from `fastly-async-macros` |

## License

Licensed under the [Opinionated Queer License v1.3](../../LICENSE.md).
