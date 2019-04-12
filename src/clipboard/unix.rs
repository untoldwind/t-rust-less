use super::{ClipboardError, ClipboardResult};
use xcb::{Atom, Connection, Window};

macro_rules! try_continue {
  ( $expr:expr ) => {
    match $expr {
      Some(val) => val,
      None => continue,
    }
  };
}

#[derive(Clone, Debug)]
pub struct Atoms {
  pub primary: Atom,
  pub clipboard: Atom,
  pub property: Atom,
  pub targets: Atom,
  pub string: Atom,
  pub utf8_string: Atom,
  pub incr: Atom,
}

pub struct Context {
  pub connection: Connection,
  pub screen: i32,
  pub window: Window,
  pub atoms: Atoms,
}

impl Context {
  pub fn new(displayname: Option<&str>) -> ClipboardResult<Self> {
    let (connection, screen) = Connection::connect(None)?;
    let window = connection.generate_id();

    {
      let screen = connection
        .get_setup()
        .roots()
        .nth(screen as usize)
        .ok_or(ClipboardError("Invalid screen".to_string()))?;
      xcb::create_window(
        &connection,
        xcb::COPY_FROM_PARENT as u8,
        window,
        screen.root(),
        0,
        0,
        1,
        1,
        0,
        xcb::WINDOW_CLASS_INPUT_OUTPUT as u16,
        screen.root_visual(),
        &[(
          xcb::CW_EVENT_MASK,
          xcb::EVENT_MASK_STRUCTURE_NOTIFY | xcb::EVENT_MASK_PROPERTY_CHANGE,
        )],
      );
      connection.flush();
    }

    let atoms = Atoms {
      primary: xcb::ATOM_PRIMARY,
      clipboard: Self::get_atom(&connection, "CLIPBOARD")?,
      property: Self::get_atom(&connection, "THIS_CLIPBOARD_OUT")?,
      targets: Self::get_atom(&connection, "TARGETS")?,
      string: xcb::ATOM_STRING,
      utf8_string: Self::get_atom(&connection, "UTF8_STRING")?,
      incr: Self::get_atom(&connection, "INCR")?,
    };

    Ok(Context {
      connection,
      screen,
      window,
      atoms,
    })
  }

  pub fn get_atom(connection: &Connection, name: &str) -> ClipboardResult<Atom> {
    xcb::intern_atom(connection, false, name)
      .get_reply()
      .map(|reply| reply.atom())
      .map_err(Into::into)
  }

  fn run(&self) {
    while let Some(event) = self.connection.wait_for_event() {
      match event.response_type() & !0x80 {
        xcb::SELECTION_REQUEST => {
          let event = unsafe { xcb::cast_event::<xcb::SelectionRequestEvent>(&event) };
        }
        _ => (),
      }
    }
  }
}
