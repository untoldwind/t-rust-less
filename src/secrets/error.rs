use ex_crypto;

error_chain! {
    types {
        Error, ErrorKind, ResultExt, Result;
    }

    links {
        Crypto(ex_crypto::error::Error, ex_crypto::error::ErrorKind);
    }

    errors {
    }
}
