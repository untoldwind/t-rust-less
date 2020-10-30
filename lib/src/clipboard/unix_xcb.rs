#![cfg(all(unix, feature = "with_xcb"))]

use std::thread;

use log::debug;
use std::sync::{Arc, RwLock};
use xcb::{Atom, Connection, Window};

use super::{ClipboardError, ClipboardResult, SelectionProvider};
use crate::api::{Event, EventHub};
use crate::clipboard::debounce::SelectionDebounce;
use std::sync::atomic::{AtomicBool, Ordering};
use zeroize::Zeroize;

#[derive(Clone, Debug)]
struct Atoms {
  pub primary: Atom,
  pub clipboard: Atom,
  pub targets: Atom,
  pub string: Atom,
  pub utf8_string: Atom,
}

struct Context {
  pub connection: Connection,
  pub screen: i32,
  pub window: Window,
  pub atoms: Atoms,
  open: AtomicBool,
  provider: Arc<RwLock<dyn SelectionProvider>>,
  store_name: String,
  block_id: String,
  event_hub: Arc<dyn EventHub>,
}

impl Context {
  fn new(
    display_name: &str,
    store_name: String,
    block_id: String,
    event_hub: Arc<dyn EventHub>,
    provider: Arc<RwLock<dyn SelectionProvider>>,
  ) -> ClipboardResult<Self> {
    let (connection, screen) = Connection::connect(Some(display_name))?;
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
    };

    debug!("{:?}", atoms);

    Ok(Context {
      connection,
      screen,
      window,
      atoms,
      open: AtomicBool::new(true),
      store_name,
      block_id,
      event_hub,
      provider,
    })
  }

  fn destroy(&self) {
    if self.open.swap(false, Ordering::Relaxed) {
      xcb::destroy_window(&self.connection, self.window);
      self.connection.flush();
    }
  }

  fn get_atom(connection: &Connection, name: &str) -> ClipboardResult<Atom> {
    xcb::intern_atom(connection, false, name)
      .get_reply()
      .map(|reply| reply.atom())
      .map_err(Into::into)
  }

  fn is_open(&self) -> bool {
    self.open.load(Ordering::Relaxed)
  }

  fn currently_providing(&self) -> Option<String> {
    self.provider.read().ok()?.current_selection_name()
  }

  fn provide_next(&self) {
    if let Ok(mut provider) = self.provider.write() {
      provider.get_selection();
    }
  }
}

pub struct Clipboard {
  context: Arc<Context>,
  handle: RwLock<Option<thread::JoinHandle<()>>>,
}

impl Clipboard {
  pub fn new<T>(
    display_name: &str,
    selection_provider: T,
    store_name: String,
    block_id: String,
    event_hub: Arc<dyn EventHub>,
  ) -> ClipboardResult<Clipboard>
  where
    T: SelectionProvider + 'static,
  {
    let context = Arc::new(Context::new(
      display_name,
      store_name,
      block_id,
      event_hub,
      Arc::new(RwLock::new(selection_provider)),
    )?);

    let handle = thread::spawn({
      let cloned = context.clone();
      move || run(cloned)
    });

    Ok(Clipboard {
      context,
      handle: RwLock::new(Some(handle)),
    })
  }

  pub fn destroy(&self) {
    self.context.destroy()
  }

  pub fn is_open(&self) -> bool {
    self.context.is_open()
  }

  pub fn currently_providing(&self) -> Option<String> {
    self.context.currently_providing()
  }

  pub fn provide_next(&self) {
    self.context.provide_next()
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

unsafe impl Send for Context {}

unsafe impl Sync for Context {}

fn run(context: Arc<Context>) {
  let mut debounce = SelectionDebounce::new(context.provider.clone());

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

        debug!(
          "{} {} {} {} {} {}",
          event.time(),
          event.owner(),
          event.selection(),
          event.property(),
          event.target(),
          event.requestor()
        );
        debug!("Selection target: {}", target);

        if target == context.atoms.targets {
          xcb::change_property(
            &context.connection,
            xcb::PROP_MODE_REPLACE as u8,
            event.requestor(),
            property,
            xcb::ATOM_ATOM,
            32,
            &[context.atoms.targets, context.atoms.string, context.atoms.utf8_string],
          );
        } else if target == context.atoms.string || target == context.atoms.utf8_string {
          match debounce.get_selection() {
            Some(mut value) => {
              if let Some(property) = debounce.current_selection_name() {
                context.event_hub.send(Event::ClipboardProviding {
                  store_name: context.store_name.clone(),
                  block_id: context.block_id.clone(),
                  property,
                });
              }
              xcb::change_property(
                &context.connection,
                xcb::PROP_MODE_REPLACE as u8,
                event.requestor(),
                property,
                target,
                8,
                value.as_ref(),
              );
              value.zeroize();
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
  context.event_hub.send(Event::ClipboardDone);
  context.destroy();
}
