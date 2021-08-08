use zeroize::Zeroizing;

pub trait CapnpSerializable: Sized {
  fn deserialize_capnp(raw: &[u8]) -> capnp::Result<Self>;

  fn serialize_capnp(self) -> capnp::Result<Zeroizing<Vec<u8>>>;
}

pub trait CapnpSerializing: Sized {
  type Owned: for<'a> capnp::traits::Owned<'a>;

  fn from_reader(reader: <Self::Owned as capnp::traits::Owned>::Reader) -> capnp::Result<Self>;

  fn to_builder(&self, builder: <Self::Owned as capnp::traits::Owned>::Builder) -> capnp::Result<()>;
}

impl<T> CapnpSerializable for T
where
  T: CapnpSerializing,
{
  fn deserialize_capnp(mut raw: &[u8]) -> capnp::Result<Self> {
    let reader = capnp::serialize::read_message_from_flat_slice(&mut raw, capnp::message::ReaderOptions::new())?;
    let type_reader = reader.get_root::<<T::Owned as capnp::traits::Owned>::Reader>()?;

    Self::from_reader(type_reader)
  }

  fn serialize_capnp(self) -> capnp::Result<zeroize::Zeroizing<Vec<u8>>> {
    let mut message = capnp::message::Builder::new(crate::memguard::weak::ZeroingHeapAllocator::default());
    let builder = message.init_root::<<T::Owned as capnp::traits::Owned>::Builder>();

    self.to_builder(builder)?;

    Ok(capnp::serialize::write_message_to_words(&message).into())
  }
}
