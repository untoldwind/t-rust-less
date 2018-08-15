error_chain! {
    types {
        Error, ErrorKind, ResultExt, Result;
    }

    foreign_links {
        OpenSSL(::openssl::error::ErrorStack);
        Io(::std::io::Error);
    }

    errors {
        InvalidHeader(t: String) {
            description("Invalid header")
            display("Invalid header: {}", t)
        }

        InvalidContent(t: String) {
            description("Invalid content")
            display("Invalid content: {}", t)
        }

        InvalidMPI {
            description("Invalid MPI")
            display("Invalid MPI")
        }

        Nom(kind: ::nom::ErrorKind) {
            description("parsing error")
            display("parsing error: {:?}", kind)
        }
    }
}

impl<T> From<::nom::Err<T>> for Error {
    fn from(err: ::nom::Err<T>) -> Error {
        let kind = err.into_error_kind();
        Error::from_kind(ErrorKind::Nom(kind))
    }
}
