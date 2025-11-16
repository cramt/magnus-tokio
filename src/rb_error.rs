use magnus::{Error, ExceptionClass, Module, Ruby, value::Lazy};

use crate::TOKIO_MODULE;

static ERROR_MODULE: Lazy<ExceptionClass> = Lazy::new(|ruby| {
    ruby.get_inner(&TOKIO_MODULE)
        .define_error("Error", ruby.exception_standard_error())
        .unwrap()
});

static CANT_MAKE_PIPE_MODULE: Lazy<ExceptionClass> = Lazy::new(|ruby| {
    ruby.get_inner(&ERROR_MODULE)
        .define_error("CantMakePipe", ruby.exception_standard_error())
        .unwrap()
});

static MALFORMED_DESERILIZATION: Lazy<ExceptionClass> = Lazy::new(|ruby| {
    ruby.get_inner(&ERROR_MODULE)
        .define_error("MalformedDeseralization", ruby.exception_standard_error())
        .unwrap()
});

pub fn cant_make_pipe(ruby: &Ruby, text: String) -> Error {
    Error::new(ruby.get_inner(&CANT_MAKE_PIPE_MODULE), text)
}

pub fn malformed_deserilization(ruby: &Ruby, text: String) -> Error {
    Error::new(ruby.get_inner(&MALFORMED_DESERILIZATION), text)
}

pub fn init(ruby: &Ruby) {
    Lazy::force(&ERROR_MODULE, ruby);
    Lazy::force(&CANT_MAKE_PIPE_MODULE, ruby);
}
