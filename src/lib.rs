mod rb_error;

use bincode::de::read::SliceReader;
use magnus::error::Result;
use magnus::value::{Lazy, Qnil, ReprValue};
use magnus::{Error, IntoValue, Module, RModule, RString, Ruby, Value, kwargs};
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::os::unix::io::AsRawFd;
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

pub fn future_to_async_task<F>(runtime: &Runtime, future: F) -> Result<Value>
where
    F: Future + Send + 'static,
    F::Output: Serialize + IntoValue + DeserializeOwned + Send + 'static,
{
    fn fd_to_async_task<T>(ruby: &Ruby, fd: i32) -> Result<Value>
    where
        T: IntoValue + DeserializeOwned + Send + 'static,
    {
        let block = ruby.proc_from_fn(move |ruby, _args, _block| {
            let io: Value = ruby
                .class_io()
                .funcall("for_fd", (fd, kwargs!("autoclose" => true)))?;
            let _: Value = io.funcall("binmode", ())?;
            let _: Value = io.funcall("nonblock=", (true,))?;
            let stream: Value = ruby.class_io().funcall("Stream", (io,))?;
            let string: RString = stream.funcall("read", ())?;
            let bytes = string.to_bytes();
            let obj: T = bincode::serde::decode_from_reader(
                SliceReader::new(bytes.as_ref()),
                bincode::config::standard(),
            )
            .map_err(|x| rb_error::malformed_deserilization(&ruby, x.to_string()))?;

            Ok(obj.into_value_with(ruby))
        });
        let task: Value = ruby
            .module_kernel()
            .funcall_with_block("Async", (), block)?;
        Ok(task)
    }
    let ruby = Ruby::get().unwrap();
    Lazy::force(&LAZY_INIT, &ruby);

    let (mut sender, receiver) = runtime
        .block_on(runtime.spawn(async move { tokio::net::unix::pipe::pipe() }))
        .map_err(|x| rb_error::cant_make_pipe(&ruby, x.to_string()))?
        .map_err(|x| rb_error::cant_make_pipe(&ruby, x.to_string()))?;
    let receiver_fd = receiver.as_raw_fd();
    std::mem::forget(receiver);
    runtime.spawn(async move {
        let result = future.await;
        let result = bincode::serde::encode_to_vec(result, bincode::config::standard()).unwrap();
        let r = sender.write_all(&result).await.unwrap();
        r
    });

    fd_to_async_task::<F::Output>(&ruby, receiver_fd)
}
