use cursive::CursiveRunnable;
use log::error;

const THEME: &str = r##"
shadow = false
borders = "simple"

[colors]
    background = "#000000"

    shadow     = ["#000000", "black"]
    view       = "#222222"

    primary   = "#FFFFFF"
    secondary = "#dddddd"
    tertiary  = "#444444"

    title_primary   = "#ff5555"
    title_secondary = "#ffff55"

    highlight          = "#ffffff"
    highlight_inactive = "#5555FF"
"##;

pub fn create_tui() -> CursiveRunnable {
  let mut siv = cursive::default();

  if let Err(error) = siv.load_toml(THEME) {
    error!("{:?}", error);
  }

  siv
}
