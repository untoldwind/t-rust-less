#![cfg(all(unix, feature = "with_xcb"))]

use std::thread;

use log::debug;
use std::sync::{Arc, RwLock};
use xcb::{Atom, Connection, Window};

use super::{ClipboardError, ClipboardResult, SelectionProvider};

#[derive(Clone, Debug)]
struct Atoms {
  pub primary: Atom,
  pub clipboard: Atom,
  pub targets: Atom,
  pub string: Atom,
  pub utf8_string: Atom,
  pub text_plain: Atom,
  pub text_plain_utf8: Atom,
}

struct Context {
  pub connection: Connection,
  pub screen: i32,
  pub window: Window,
  pub atoms: Atoms,
}

impl Context {
  fn new(displayname: Option<&str>) -> ClipboardResult<Self> {
    let (connection, screen) = Connection::connect(displayname)?;
    let window = connection.generate_id();

    {
      let screen = connection
        .get_setup()
        .roots()
        .nth(screen as usize)
        .ok_or_else(|| ClipboardError("Invalid screen".to_string()))?;
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
      targets: Self::get_atom(&connection, "TARGETS")?,
      string: xcb::ATOM_STRING,
      utf8_string: Self::get_atom(&connection, "UTF8_STRING")?,
      text_plain: Self::get_atom(&connection, "text/plain")?,
      text_plain_utf8: Self::get_atom(&connection, "ttext/plain;charset=utf-8")?,
    };

    debug!("{:?}", atoms);

    Ok(Context {
      connection,
      screen,
      window,
      atoms,
    })
  }

  fn destroy(&self) {
    xcb::destroy_window(&self.connection, self.window);
    self.connection.flush();
  }

  fn get_atom(connection: &Connection, name: &str) -> ClipboardResult<Atom> {
    xcb::intern_atom(connection, false, name)
      .get_reply()
      .map(|reply| reply.atom())
      .map_err(Into::into)
  }
}

pub struct Clipboard {
  context: Arc<Context>,
  handle: RwLock<Option<thread::JoinHandle<()>>>,
}

impl Clipboard {
  pub fn new<T>(selection_provider: T) -> ClipboardResult<Clipboard>
  where
    T: SelectionProvider + 'static,
  {
    let context = Arc::new(Context::new(None)?);

    let handle = thread::spawn({
      let cloned = context.clone();
      move || run(cloned, selection_provider)
    });

    Ok(Clipboard {
      context,
      handle: RwLock::new(Some(handle)),
    })
  }

  pub fn destroy(&self) {
    self.context.destroy()
  }

  pub fn wait(&self) -> ClipboardResult<()> {
    let mut maybe_handle = self.handle.write().unwrap();
    if let Some(handle) = maybe_handle.take() {
      handle.join().map_err(|_| ClipboardError("wait timeout".to_string()))?;
    }
    Ok(())
  }
}

impl Drop for Clipboard {
  fn drop(&mut self) {
    self.destroy()
  }
}

fn run<T>(context: Arc<Context>, mut selection_provider: T)
where
  T: SelectionProvider,
{
  if xcb::set_selection_owner_checked(
    &context.connection,
    context.window,
    context.atoms.clipboard,
    xcb::CURRENT_TIME,
  )
  .request_check()
  .is_err()
  {
    return;
  }

  context.connection.flush();

  while let Some(event) = context.connection.wait_for_event() {
    match event.response_type() & !0x80 {
      xcb::SELECTION_REQUEST => {
        let event = unsafe { xcb::cast_event::<xcb::SelectionRequestEvent>(&event) };
        let target = event.target();
        let mut property = event.property();

        debug!("Selection target: {}", target);

        if target == context.atoms.targets {
          xcb::change_property(
            &context.connection,
            xcb::PROP_MODE_REPLACE as u8,
            event.requestor(),
            property,
            xcb::ATOM_ATOM,
            32,
            &[context.atoms.targets, context.atoms.utf8_string,  context.atoms.text_plain, context.atoms.text_plain_utf8],
          );
        } else if target == context.atoms.string
          || target == context.atoms.utf8_string
          || target == context.atoms.text_plain
          || target == context.atoms.text_plain_utf8
        {
          match selection_provider.get_selection() {
            Some(value) => {
              xcb::change_property(
                &context.connection,
                xcb::PROP_MODE_REPLACE as u8,
                event.requestor(),
                property,
                target,
                8,
                value.as_bytes(),
              );
            }
            None => {
              xcb::set_selection_owner(
                &context.connection,
                xcb::NONE,
                context.atoms.clipboard,
                xcb::CURRENT_TIME,
              );

              context.connection.flush();

              break;
            }
          }
        } else {
          debug!("Reply with NONE");
          property = xcb::ATOM_NONE;
        }

        xcb::send_event(
          &context.connection,
          false,
          event.requestor(),
          xcb::EVENT_MASK_NO_EVENT,
          &xcb::SelectionNotifyEvent::new(event.time(), event.requestor(), event.selection(), target, property),
        );

        context.connection.flush();
      }
      xcb::SELECTION_CLEAR => {
        debug!("Lost selection ownership");

        break;
      }
      xcb::DESTROY_NOTIFY => {
        debug!("Window destroyed");

        break;
      }
      ignored => debug!("Ignore event {}", ignored),
    }
  }

  debug!("Ending event loop");
}
