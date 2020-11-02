#[allow(dead_code)]
mod clipboard;
#[allow(dead_code)]
mod fixtures;

fn main() {
  env_logger::Builder::from_default_env()
    .filter(None, log::LevelFilter::Debug)
    .target(env_logger::Target::Stderr)
    .init();

  fixtures::generate_fixtures();
  //  clipboard::experimental_clipboard();
}
