mod rb_error;

use magnus::error::Result;
use magnus::value::{Lazy, Qnil, ReprValue};
use magnus::{IntoValue, RModule, Ruby, Value, kwargs};
use std::os::fd::{IntoRawFd, OwnedFd};
use std::sync::{Arc, Mutex};
use tokio::io::AsyncWriteExt;
use tokio::runtime::Runtime;

static TOKIO_MODULE: Lazy<RModule> = Lazy::new(|ruby| ruby.define_module("Tokio").unwrap());

static LAZY_INIT: Lazy<Qnil> = Lazy::new(|ruby| {
    ruby.require("async").unwrap();
    ruby.require("io/stream").unwrap();
    Lazy::force(&TOKIO_MODULE, &ruby);
    rb_error::init(&ruby);
    ruby.qnil()
});

/// Wraps an `OwnedFd` so we can hand the raw fd to Ruby's `IO.for_fd` while
/// guaranteeing that — if the handoff never completes (proc never invoked,
/// proc errors before transfer) — `OwnedFd`'s `Drop` closes the fd.
struct FdGuard {
    fd: Option<OwnedFd>,
}

impl FdGuard {
    fn new(fd: OwnedFd) -> Self {
        Self { fd: Some(fd) }
    }

    fn into_raw(mut self) -> std::os::fd::RawFd {
        self.fd
            .take()
            .expect("FdGuard already consumed")
            .into_raw_fd()
    }
}

/// Bridge a Rust future returning `Result<T, E>` to a Ruby `Async::Task`.
///
/// The future is spawned on the supplied tokio runtime. When it completes,
/// the result is placed in a shared slot and a single byte is written to a
/// pipe. The Ruby fiber backing the returned `Async::Task` reads that byte
/// (suspending via the Async scheduler), then materializes the result on the
/// Ruby thread: `Ok(T)` becomes a Ruby value via `IntoValue`; `Err(E)` is
/// mapped to a `magnus::Error` via `into_err`.
///
/// `into_err` is invoked on the Ruby thread, so it may safely allocate Ruby
/// objects via the supplied `Ruby` handle (e.g. to look up an
/// `ExceptionClass` from a `Lazy`).
///
/// Cancellation: if the returned task is cancelled before the future
/// completes, the underlying tokio task is NOT aborted — it runs to
/// completion and its result is discarded. Plan accordingly (e.g. use
/// tokio-level timeouts inside the future).
pub fn future_result_to_async_task<F, T, E, IntoErr>(
    runtime: &Runtime,
    future: F,
    into_err: IntoErr,
) -> Result<Value>
where
    F: Future<Output = std::result::Result<T, E>> + Send + 'static,
    T: IntoValue + Send + 'static,
    E: Send + 'static,
    IntoErr: FnOnce(&Ruby, E) -> magnus::Error + Send + 'static,
{
    let ruby = Ruby::get().unwrap();
    Lazy::force(&LAZY_INIT, &ruby);

    let (mut sender, receiver) = {
        let _enter = runtime.enter();
        tokio::net::unix::pipe::pipe()
            .map_err(|x| rb_error::cant_make_pipe(&ruby, x.to_string()))?
    };

    // Move ownership of the receive end out of tokio into a plain OwnedFd so
    // we can hand its raw fd to Ruby's IO.for_fd(..., autoclose: true). The
    // FdGuard ensures the fd is closed if that handoff never completes.
    let receiver_fd = receiver
        .into_nonblocking_fd()
        .map_err(|x| rb_error::cant_make_pipe(&ruby, x.to_string()))?;
    let guard = FdGuard::new(receiver_fd);

    let slot: Arc<Mutex<Option<std::result::Result<T, E>>>> = Arc::new(Mutex::new(None));
    let slot_writer = slot.clone();

    runtime.spawn(async move {
        let output = future.await;
        if let Ok(mut g) = slot_writer.lock() {
            *g = Some(output);
        }
        // Ignore write errors: the reader may have closed early because the
        // Ruby task was cancelled. The result is dropped along with the slot.
        let _ = sender.write_all(&[0u8]).await;
    });

    let guard_cell = Arc::new(Mutex::new(Some(guard)));
    let into_err_cell: Arc<Mutex<Option<IntoErr>>> = Arc::new(Mutex::new(Some(into_err)));

    let block = ruby.proc_from_fn(move |ruby, _args, _block| -> Result<Value> {
        let fd = guard_cell
            .lock()
            .map_err(|_| rb_error::task_aborted(ruby))?
            .take()
            .ok_or_else(|| rb_error::task_aborted(ruby))?
            .into_raw();

        let io: Value = ruby
            .class_io()
            .funcall("for_fd", (fd, kwargs!("autoclose" => true)))?;
        let _: Value = io.funcall("binmode", ())?;
        let _: Value = io.funcall("nonblock=", (true,))?;
        let stream: Value = ruby.class_io().funcall("Stream", (io,))?;
        let read_val: Value = stream.funcall("read", (1,))?;

        if read_val.is_nil() {
            return Err(rb_error::task_aborted(ruby));
        }

        let into_err = into_err_cell
            .lock()
            .map_err(|_| rb_error::task_aborted(ruby))?
            .take()
            .ok_or_else(|| rb_error::task_aborted(ruby))?;

        let result = slot
            .lock()
            .map_err(|_| rb_error::task_aborted(ruby))?
            .take()
            .ok_or_else(|| rb_error::task_aborted(ruby))?;

        match result {
            Ok(val) => Ok(val.into_value_with(ruby)),
            Err(err) => Err(into_err(ruby, err)),
        }
    });

    let task: Value = ruby
        .module_kernel()
        .funcall_with_block("Async", (), block)?;
    Ok(task)
}

/// Bridge an infallible Rust future to a Ruby `Async::Task`.
///
/// Equivalent to `future_result_to_async_task` over `Ok::<_, Infallible>(_)`.
pub fn future_to_async_task<F>(runtime: &Runtime, future: F) -> Result<Value>
where
    F: Future + Send + 'static,
    F::Output: IntoValue + Send + 'static,
{
    future_result_to_async_task::<_, F::Output, std::convert::Infallible, _>(
        runtime,
        async move { Ok::<_, std::convert::Infallible>(future.await) },
        |_, _| unreachable!("infallible future cannot produce Err"),
    )
}
