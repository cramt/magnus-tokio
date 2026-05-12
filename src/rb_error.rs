use magnus::{Error, ExceptionClass, Module, Ruby, value::Lazy};

use crate::TOKIO_MODULE;

static ERROR_MODULE: Lazy<ExceptionClass> = Lazy::new(|ruby| {
    ruby.get_inner(&TOKIO_MODULE)
        .define_error("Error", ruby.exception_standard_error())
        .unwrap()
});

static CANT_MAKE_PIPE: Lazy<ExceptionClass> = Lazy::new(|ruby| {
    ruby.get_inner(&ERROR_MODULE)
        .define_error("CantMakePipe", ruby.exception_standard_error())
        .unwrap()
});

static TASK_ABORTED: Lazy<ExceptionClass> = Lazy::new(|ruby| {
    ruby.get_inner(&ERROR_MODULE)
        .define_error("TaskAborted", ruby.exception_standard_error())
        .unwrap()
});

pub fn cant_make_pipe(ruby: &Ruby, text: String) -> Error {
    Error::new(ruby.get_inner(&CANT_MAKE_PIPE), text)
}

pub fn task_aborted(ruby: &Ruby) -> Error {
    Error::new(
        ruby.get_inner(&TASK_ABORTED),
        "tokio task did not produce a result (runtime dropped, task panicked, or pipe closed early)"
            .to_string(),
    )
}

pub fn init(ruby: &Ruby) {
    Lazy::force(&ERROR_MODULE, ruby);
    Lazy::force(&CANT_MAKE_PIPE, ruby);
    Lazy::force(&TASK_ABORTED, ruby);
}
