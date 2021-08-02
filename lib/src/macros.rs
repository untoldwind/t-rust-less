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

macro_rules! impl_capnp_serialize {
  ($for_type:ty, $capnp_module:ident) => {
    impl crate::api::CapnpSerializable for $for_type {
      fn deserialize_capnp(mut raw: &[u8]) -> capnp::Result<Self> {
        let reader = capnp::serialize::read_message_from_flat_slice(&mut raw, capnp::message::ReaderOptions::new())?;
        let type_reader = reader.get_root::<$capnp_module::Reader>()?;

        Self::from_reader(type_reader)
      }

      fn serialize_capnp(&self) -> capnp::Result<Vec<u8>> {
        let mut message = capnp::message::Builder::new(crate::memguard::weak::ZeroingHeapAllocator::default());
        let builder = message.init_root::<$capnp_module::Builder>();

        self.to_builder(builder)?;

        Ok(capnp::serialize::write_message_to_words(&message))
      }
    }
  };
}
