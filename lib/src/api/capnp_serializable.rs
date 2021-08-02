use zeroize::Zeroizing;

pub trait CapnpSerializable: Sized {
  fn deserialize_capnp(raw: &[u8]) -> capnp::Result<Self>;

  fn serialize_capnp(self) -> capnp::Result<Zeroizing<Vec<u8>>>;
}

impl CapnpSerializable for () {
  fn deserialize_capnp(_raw: &[u8]) -> capnp::Result<Self> {
    Ok(())
  }

  fn serialize_capnp(self) -> capnp::Result<Zeroizing<Vec<u8>>> {
    Ok(vec![].into())
  }
}

impl CapnpSerializable for bool {
  fn deserialize_capnp(raw: &[u8]) -> capnp::Result<Self> {
    Ok(raw[0] > 0)
  }

  fn serialize_capnp(self) -> capnp::Result<Zeroizing<Vec<u8>>> {
    if self {
      Ok(vec![1].into())
    } else {
      Ok(vec![0].into())
    }
  }
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

impl CapnpSerializing for String {
  type Owned = capnp::text::Owned;

  fn from_reader(reader: capnp::text::Reader) -> capnp::Result<Self> {
    Ok(reader.to_string())
  }

  fn to_builder(&self, mut builder: capnp::text::Builder) -> capnp::Result<()> {
    builder.push_str(self);
    Ok(())
  }
}
