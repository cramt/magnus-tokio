use magnus::{function, Error};
use tokio::io::AsyncWriteExt;
use once_cell::sync::Lazy;
use std::os::unix::io::{AsRawFd};
use std::time::Duration;
use tokio::runtime::{Runtime};

static RUNTIME: Lazy<Runtime> = Lazy::new(|| {
    Runtime::new().unwrap()
});

fn my_cool_async_function() -> i32 {
    let (mut sender, receiver) = RUNTIME.block_on(
        RUNTIME.spawn(async move {
            tokio::net::unix::pipe::pipe().unwrap()
        })
    ).unwrap();
    let receiver_fd = receiver.as_raw_fd();
    std::mem::forget(receiver);
    RUNTIME.spawn(async move {
        tokio::time::sleep(Duration::from_secs(5)).await;
        sender.write_all(b"test").await.unwrap();
    });
    receiver_fd
}

#[magnus::init]
fn init(ruby: &magnus::Ruby) -> Result<(), Error> {
    let module = ruby.define_module("MyThing")?;
    module.define_module_function("some_async_thing", function!(my_cool_async_function, 0))?;
    Ok(())
}
