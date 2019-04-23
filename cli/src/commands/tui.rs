use cursive::{theme, Cursive};
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

pub fn create_tui() -> Cursive {
  let mut siv = Cursive::default();

  match theme::load_toml(THEME) {
    Ok(theme) => {
      siv.set_theme(theme);
    }
    Err(error) => {
      error!("{:?}", error);
    }
  }

  siv
}
