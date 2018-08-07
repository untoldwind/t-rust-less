use std::convert::From;
use std::sync::PoisonError;

error_chain! {
    types {
        Error, ErrorKind, ResultExt, Result;
    }

    foreign_links {
        Io(::std::io::Error);
    }

    errors {
        MutexPoisoned {
            description("Mutex is poisoned")
            display("Mutex is poisoned")
        }
    }
}

impl<T> From<PoisonError<T>> for Error {
    fn from(_err : PoisonError<T>) -> Error {
        ErrorKind::MutexPoisoned.into()
    }
}