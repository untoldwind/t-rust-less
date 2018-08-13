use std::convert::From;
use std::sync::PoisonError;

error_chain! {
    types {
        Error, ErrorKind, ResultExt, Result;
    }

    foreign_links {
        OpenSSL(::openssl::error::ErrorStack);
        Io(::std::io::Error);
        Url(::url::ParseError);
    }

    errors {
        MutexPoisoned {
            description("Mutex is poisoned")
            display("Mutex is poisoned")
        }

        InvalidStoreUrl(url: String) {
            description("Invalid store url")
            display("Invalid store url: {}", url)
        }
    }
}

impl<T> From<PoisonError<T>> for Error {
    fn from(_err: PoisonError<T>) -> Error {
        ErrorKind::MutexPoisoned.into()
    }
}
