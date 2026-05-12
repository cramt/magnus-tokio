# Magnus Tokio

Bridge Rust `tokio` futures to Ruby `Async::Task`s from a Magnus-based native extension.

When you call `future_result_to_async_task(runtime, future, into_err)` from a Ruby
thread, it returns an `Async::Task`. Awaiting that task (`task.wait`) suspends the
calling Ruby fiber until the Rust future completes; `Ok(T)` becomes a Ruby value,
`Err(E)` is mapped to a `magnus::Error` by your `into_err` closure.

The handoff uses a unix pipe as a readiness signal (one byte) and a shared
`Mutex<Option<Result<T, E>>>` for the value, so `T` and `E` do not need to be
serializable.

## Usage

```rust
use magnus::value::Lazy;
use magnus::ExceptionClass;
use magnus_tokio::future_result_to_async_task;
use once_cell::sync::Lazy as StdLazy;
use tokio::runtime::Runtime;

static RUNTIME: StdLazy<Runtime> = StdLazy::new(|| Runtime::new().unwrap());
static MY_ERROR: Lazy<ExceptionClass> = Lazy::new(|ruby| {
    ruby.class_object().define_error("MyError", ruby.exception_standard_error()).unwrap()
});

fn do_work() -> magnus::error::Result<magnus::Value> {
    future_result_to_async_task(
        &*RUNTIME,
        async {
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            Ok::<_, String>(42_i32)
        },
        |ruby, msg| magnus::Error::new(ruby.get_inner(&MY_ERROR), msg),
    )
}
```

See `example/` for a full Ruby gem layout.

## Caveats

- Unix only (uses `tokio::net::unix::pipe` and raw fds).
- Cancellation: cancelling the returned `Async::Task` does not abort the tokio
  future. The Rust task runs to completion and its result is discarded. Use
  tokio-level timeouts inside the future if you need a hard deadline.
- The future, `T`, and `E` must all be `Send + 'static`. Ruby `Value`s are not
  `Send` — use Magnus-wrapped Rust structs as your `T`/`E` instead.

## Errors

Defined under `Tokio::Error`:

- `Tokio::Error::CantMakePipe` — `pipe(2)` failed.
- `Tokio::Error::TaskAborted` — the tokio task did not produce a result (runtime
  dropped, task panicked, or the pipe closed before the byte signal arrived).
