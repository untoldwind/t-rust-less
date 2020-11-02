use rand::{distributions, thread_rng, Rng};
use t_rust_less_lib::secrets_store_capnp::block;
use t_rust_less_lib::{
  memguard::SecretBytes,
  secrets_store::cipher::{Cipher, RUST_RSA_AES_GCM},
};

pub fn generate_fixtures() {
  let cipher = &RUST_RSA_AES_GCM;

  let mut rng = thread_rng();
  let seal_nonce = rng
    .sample_iter(&distributions::Standard)
    .take(cipher.seal_min_nonce_length())
    .collect::<Vec<u8>>();
  let seal_key = SecretBytes::random(&mut rng, cipher.seal_key_length());

  let (public_key, private_key) = cipher.generate_key_pair().unwrap();

  let crypted_private = cipher.seal_private_key(&seal_key, &seal_nonce, &private_key).unwrap();

  println!("const SEAL_NONCE : &[u8] = &hex!(\"{}\");", hex::encode(seal_nonce));
  println!(
    "const SEAL_KEY : &[u8] = &hex!(\"{}\");",
    hex::encode(seal_key.borrow().as_bytes())
  );
  println!(
    "const CRYPTED_KEY : &[u8] = &hex!(\"{}\n\");",
    long_hex(&crypted_private)
  );

  println!("const MESSAGES : &[&[u8]] = &[");
  for _ in 0..10 {
    make_message(cipher, public_key.clone())
  }
  println!("];");
}

fn long_hex<T: AsRef<[u8]>>(data: T) -> String {
  hex::encode(data)
    .chars()
    .enumerate()
    .flat_map(|(i, c)| if i % 80 == 0 { vec!['\n', ' ', ' ', c] } else { vec![c] })
    .collect()
}

fn make_message<T: Cipher>(cipher: &T, public_key: Vec<u8>) {
  let mut message = capnp::message::Builder::new_default();

  let id1 = "recipient1";

  let mut block = message.init_root::<block::Builder>();
  let headers = block.reborrow().init_headers(1);

  let private_data = SecretBytes::from("Hello, secret".to_string());

  let crypted_data = cipher
    .encrypt(&[(id1, public_key)], &private_data, headers.get(0))
    .unwrap();
  block.set_content(&crypted_data);

  let message_payload: &[u8] = &capnp::serialize::write_message_to_words(&message);

  println!("&hex!(\"{}\n\"),", long_hex(message_payload));
}
