macro_rules! stry {
  ($expr:expr) => {
    match $expr {
      std::result::Result::Ok(val) => val,
      std::result::Result::Err(err) => return capnp::capability::Promise::err(err.into()),
    }
  };
}
