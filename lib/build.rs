use std::fs;
use std::io::ErrorKind;

fn main() -> Result<(), Box<dyn std::error::Error>> {
  let api_src = fs::metadata("src/api.capnp")?;

  let rebuild_api = match fs::metadata("src/api_capnp.rs") {
    Ok(target) => target.modified()? < api_src.modified()?,
    Err(error) => match error.kind() {
      ErrorKind::NotFound => true,
      _ => return Err(error.into()),
    },
  };

  let secrets_store_src = fs::metadata("src/secrets_store.capnp")?;

  let rebuild_secrets_store = match fs::metadata("src/secrets_store_capnp.rs") {
    Ok(target) => target.modified()? < secrets_store_src.modified()?,
    Err(error) => match error.kind() {
      ErrorKind::NotFound => true,
      _ => return Err(error.into()),
    },
  };

  if rebuild_api {
    print!("Building api");
    capnpc::CompilerCommand::new()
      .file("src/api.capnp")
      .output_path(".")
      .run()?;
  }

  if rebuild_secrets_store {
    print!("Building store");
    capnpc::CompilerCommand::new()
      .file("src/secrets_store.capnp")
      .output_path(".")
      .run()?;
  }

  Ok(())
}
