use magnus::error::Result;
use magnus::value::Lazy;
use magnus::{Error, Module, Object, RModule, Value, function, method};
use magnus_tokio::future_result_to_async_task;
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[magnus::wrap(class = "MyModule::ErrorStruct", free_immediately, size)]
struct ErrorStruct {
    message: String,
}

impl ErrorStruct {
    pub fn new(message: String) -> Self {
        Self { message }
    }

    pub fn message(&self) -> String {
        self.message.clone()
    }
}

static MY_MODULE: Lazy<RModule> = Lazy::new(|ruby| ruby.define_module("MyModule").unwrap());

static ERROR_CLASS: Lazy<magnus::ExceptionClass> = Lazy::new(|ruby| {
    ruby.get_inner(&MY_MODULE)
        .define_error("Error", ruby.exception_standard_error())
        .unwrap()
});

static RUNTIME: StdLazy<Runtime> = StdLazy::new(|| Runtime::new().unwrap());

fn sleep(milis: i32) -> Result<Value> {
    let ruby = magnus::Ruby::get().unwrap();
    future_result_to_async_task(
        &*RUNTIME,
        async move {
            tokio::time::sleep(Duration::from_millis(milis.try_into().unwrap_or_default())).await;
            Ok::<_, ErrorStruct>(ExampleStruct { sleep_time: milis })
        },
        ruby.get_inner(&ERROR_CLASS),
    )
}

fn fail_after(milis: i32) -> Result<Value> {
    let ruby = magnus::Ruby::get().unwrap();
    future_result_to_async_task(
        &*RUNTIME,
        async move {
            tokio::time::sleep(Duration::from_millis(milis.try_into().unwrap_or_default())).await;
            Err::<ExampleStruct, _>(ErrorStruct::new("Something went wrong".to_string()))
        },
        ruby.get_inner(&ERROR_CLASS),
    )
}

#[magnus::init]
fn init(ruby: &magnus::Ruby) -> std::result::Result<(), Error> {
    let module = ruby.get_inner(&MY_MODULE);
    let class = module.define_class("ExampleStruct", ruby.class_object())?;
    class.define_method("sleep_time", method!(ExampleStruct::sleep_time, 0))?;

    let error_struct_class = module.define_class("ErrorStruct", ruby.class_object())?;
    error_struct_class.define_singleton_method("new", function!(ErrorStruct::new, 1))?;
    error_struct_class.define_method("message", method!(ErrorStruct::message, 0))?;

    Lazy::force(&ERROR_CLASS, ruby);

    module.define_module_function("sleep", function!(sleep, 1))?;
    module.define_module_function("fail_after", function!(fail_after, 1))?;
    Ok(())
}
