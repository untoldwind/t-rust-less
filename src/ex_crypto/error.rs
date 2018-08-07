error_chain! {
    types {
        Error, ErrorKind, ResultExt, Result;
    }

    foreign_links {
        OpenSSL(::openssl::error::ErrorStack);
        Io(::std::io::Error);
        SystemTime(::std::time::SystemTimeError);
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
    }
}
