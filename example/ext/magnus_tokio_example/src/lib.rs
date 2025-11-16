use magnus::error::Result;
use magnus::value::Lazy;
use magnus::{Error, Module, RModule, Value, function, method};
use magnus_tokio::future_to_async_task;
use once_cell::sync::Lazy as StdLazy;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::runtime::Runtime;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[magnus::wrap(class = "MyModule::ExampleStruct", free_immediately, size)]
struct ExampleStruct {
    sleep_time: i32,
}

impl ExampleStruct {
    pub fn sleep_time(&self) -> i32 {
        self.sleep_time
    }
}

static MY_MODULE: Lazy<RModule> = Lazy::new(|ruby| ruby.define_module("MyModule").unwrap());

static RUNTIME: StdLazy<Runtime> = StdLazy::new(|| Runtime::new().unwrap());

fn sleep(milis: i32) -> Result<Value> {
    future_to_async_task(&*RUNTIME, async move {
        tokio::time::sleep(Duration::from_millis(milis.try_into().unwrap_or_default())).await;
        ExampleStruct { sleep_time: milis }
    })
}

#[magnus::init]
fn init(ruby: &magnus::Ruby) -> std::result::Result<(), Error> {
    let module = ruby.get_inner(&MY_MODULE);
    let class = module.define_class("ExampleStruct", ruby.class_object())?;

    class.define_method("sleep_time", method!(ExampleStruct::sleep_time, 0))?;
    module.define_module_function("sleep", function!(sleep, 1))?;
    Ok(())
}
