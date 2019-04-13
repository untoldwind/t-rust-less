use atty::Stream;

pub fn status() {
  if atty::is(Stream::Stdout) {

  } else {

  }
}
