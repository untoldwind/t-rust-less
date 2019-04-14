macro_rules! error_convert_from {
  ($from_type:ty, $to_type:ident, $tgt:ident (display)) => {
    impl From<$from_type> for $to_type {
      fn from(error: $from_type) -> Self {
        $to_type::$tgt(format!("{}", error))
      }
    }
  };
  ($from_type:ty, $to_type:ident, $tgt:ident (direct)) => {
    impl From<$from_type> for $to_type {
      fn from(error: $from_type) -> Self {
        $to_type::$tgt(error)
      }
    }
  };
}
