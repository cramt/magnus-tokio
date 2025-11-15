use bincode::de::read::SliceReader;
use magnus::error::Result;
use magnus::value::ReprValue;
use magnus::{Error, IntoValue, Module, RString, Ruby, Value, function, kwargs, method};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tokio::io::AsyncWriteExt;
use once_cell::sync::Lazy;
use std::os::unix::io::{AsRawFd};
use std::time::Duration;
use tokio::runtime::{Runtime};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[magnus::wrap(class = "Tokio::ExampleStruct", free_immediately, size)]
struct ExampleStruct {
    sleep_time: i32
}

impl ExampleStruct {
    pub fn sleep_time(&self) -> i32 {
        self.sleep_time
    }
}

static RUNTIME: Lazy<Runtime> = Lazy::new(|| {
    Runtime::new().unwrap()
});

fn future_to_rb_io<F>(future: F) -> Result<Value> where   
        F: Future + Send + 'static,
        F::Output: Serialize + IntoValue + DeserializeOwned + Send + 'static,
{

    fn fd_to_async_task<T>(ruby: &Ruby, fd: i32) -> Result<Value> where T: IntoValue + DeserializeOwned + Send + 'static {
        let block = ruby.proc_from_fn(move |ruby, _args, _block| {
            let io: Value = ruby.class_io().funcall("for_fd", (fd, kwargs!("autoclose" => true)))?;
            let _:Value = io.funcall("binmode", ())?;
            let _:Value = io.funcall("nonblock=", (true,))?;
            let stream: Value = ruby.class_io().funcall("Stream", (io,))?;
            let string: RString = stream.funcall("read", ())?;
            let bytes = string.to_bytes();
            let obj: T = bincode::serde::decode_from_reader(SliceReader::new(bytes.to_vec().as_slice()), bincode::config::standard()).unwrap();
            
            Ok(obj.into_value_with(ruby))
        } );
        let task: Value = ruby.module_kernel().funcall_with_block("Async", (), block)?;
        Ok(task)
    }
    let ruby = Ruby::get().unwrap();

    
    let (mut sender, receiver) = RUNTIME.block_on(
        RUNTIME.spawn(async move {
            tokio::net::unix::pipe::pipe().unwrap()
        })
    ).unwrap();
    let receiver_fd = receiver.as_raw_fd();
    std::mem::forget(receiver);
    RUNTIME.spawn(async move {
        let result = future.await;
        let result = bincode::serde::encode_to_vec(result, bincode::config::standard()).unwrap();
        let r = sender.write_all(&result).await.unwrap();
        r
    });
    fd_to_async_task::<F::Output>(&ruby, receiver_fd)
}

fn sleep(milis: i32) -> Result<Value> {
    future_to_rb_io(async move {
        tokio::time::sleep(Duration::from_millis(milis.try_into().unwrap_or_default())).await;
        ExampleStruct {
            sleep_time: milis
        }
    })
}

#[magnus::init]
fn init(ruby: &magnus::Ruby) -> std::result::Result<(), Error> {
    let module = ruby.define_module("Tokio")?;
    let class = module.define_class("ExampleStruct", ruby.class_object())?;

    class.define_method("sleep_time", method!(ExampleStruct::sleep_time, 0))?;
    module.define_module_function("sleep", function!(sleep, 1))?;
    Ok(())
}
