mod clipboard;

fn main() {
  env_logger::Builder::from_default_env()
    .filter(None, log::LevelFilter::Debug)
    .target(env_logger::Target::Stderr)
    .init();

  clipboard::experimental_clipboard();
}
