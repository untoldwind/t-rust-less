#![cfg(all(unix, feature = "with_x11"))]

use crate::clipboard::{ClipboardError, ClipboardResult, SelectionProvider};
use log::debug;
use std::ffi::CString;
use std::mem;
use std::ptr;
use std::sync::{Arc, RwLock};
use std::thread;
use x11::xlib;

#[derive(Debug)]
struct Atoms {
  pub primary: xlib::Atom,
  pub clipboard: xlib::Atom,
  pub targets: xlib::Atom,
  pub string: xlib::Atom,
  pub utf8_string: xlib::Atom,
  pub text_plain: xlib::Atom,
  pub text_plain_utf8: xlib::Atom,
}

struct Context {
  display: *mut xlib::Display,
  window: xlib::Window,
  atoms: Atoms,
}

impl Context {
  fn new() -> ClipboardResult<Self> {
    unsafe {
      let display = xlib::XOpenDisplay(ptr::null());

      if display.is_null() {
        return Err(ClipboardError("Cannot open display".to_string()));
      }
      let root = xlib::XDefaultRootWindow(display);
      let black = xlib::XBlackPixel(display, xlib::XDefaultScreen(display));
      let window = xlib::XCreateSimpleWindow(display, root, 0, 0, 1, 1, 0, black, black);

      debug!("Window id: {}", window);

      xlib::XSelectInput(display, window, xlib::StructureNotifyMask | xlib::PropertyChangeMask);

      let primary = Self::get_atom(display, "PRIMARY");
      if primary != xlib::XA_PRIMARY {
        debug!("XA_PRIMARY is not named PRIMARY");
      }
      let clipboard = Self::get_atom(display, "CLIPBOARD");
      let targets = Self::get_atom(display, "TARGETS");
      let string = Self::get_atom(display, "STRING");
      if string != xlib::XA_STRING {
        debug!("XA_STRING is not named STRING");
      }
      let utf8_string = Self::get_atom(display, "UTF8_STRING");
      let text_plain = Self::get_atom(display, "text/plain");
      let text_plain_utf8 = Self::get_atom(display, "text/plain;charset=utf-8");

      let atoms = Atoms {
        primary,
        clipboard,
        targets,
        string,
        utf8_string,
        text_plain,
        text_plain_utf8,
      };

      debug!("{:?}", atoms);

      Ok(Context { display, window, atoms })
    }
  }

  fn get_atom(display: *mut xlib::Display, name: &str) -> xlib::Atom {
    unsafe {
      let c_name = CString::new(name).unwrap();
      xlib::XInternAtom(display, c_name.as_ptr(), xlib::False)
    }
  }

  fn destroy(&self) {
    unsafe {
      xlib::XDestroyWindow(self.display, self.window);
      xlib::XFlush(self.display);
    }
  }

  fn own_selection(&self) -> bool {
    unsafe {
      for selection in &[self.atoms.primary, self.atoms.clipboard] {
        xlib::XSetSelectionOwner(self.display, *selection, self.window, xlib::CurrentTime);

        let owner = xlib::XGetSelectionOwner(self.display, *selection);
        if owner != self.window {
          debug!("Failed taking ownership of {}", *selection);
          return false;
        }
      }
    }

    true
  }

  fn clear_selection(&self) {
    unsafe {
      for selection in &[self.atoms.primary, self.atoms.clipboard] {
        xlib::XSetSelectionOwner(self.display, *selection, 0, xlib::CurrentTime);
      }
      xlib::XFlush(self.display);
    }
  }
}

impl Drop for Context {
  fn drop(&mut self) {
    unsafe {
      xlib::XCloseDisplay(self.display);
    }
  }
}

unsafe impl Send for Context {}

unsafe impl Sync for Context {}

pub struct Clipboard {
  context: Arc<Context>,
  handle: RwLock<Option<thread::JoinHandle<()>>>,
}

impl Clipboard {
  pub fn new<T>(selection_provider: T) -> ClipboardResult<Clipboard>
  where
    T: SelectionProvider + 'static,
  {
    let context = Arc::new(Context::new()?);

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

fn run<T>(context: Arc<Context>, mut selection_provider: T)
where
  T: SelectionProvider,
{
  unsafe {
    if !context.own_selection() {
      return;
    }

    let mut event: xlib::XEvent = mem::uninitialized();

    loop {
      xlib::XFlush(context.display);
      debug!("Wating for event");
      xlib::XNextEvent(context.display, &mut event);

      debug!("Got event: {}", event.get_type());

      match event.get_type() {
        xlib::SelectionRequest => {
          let mut selection: xlib::XSelectionEvent = mem::uninitialized();
          selection.type_ = xlib::SelectionNotify;
          selection.display = event.selection_request.display;
          selection.requestor = event.selection_request.requestor;
          selection.selection = event.selection_request.selection;
          selection.time = event.selection_request.time;
          selection.target = event.selection_request.target;
          selection.property = event.selection_request.property;

          debug!("Selection target: {}", selection.target);

          if selection.target == context.atoms.targets {
            let atoms = [context.atoms.targets, context.atoms.utf8_string];
            xlib::XChangeProperty(
              context.display,
              selection.requestor,
              selection.property,
              xlib::XA_ATOM,
              32,
              xlib::PropModeReplace,
              &atoms as *const xlib::Atom as *const u8,
              2,
            );
          } else if selection.target == context.atoms.string
            || selection.target == context.atoms.utf8_string
            || selection.target == context.atoms.text_plain
            || selection.target == context.atoms.text_plain_utf8
          {
            match selection_provider.get_selection() {
              Some(value) => {
                let c_str = CString::new(value).unwrap();
                let c_str_bytes = c_str.as_bytes_with_nul();

                xlib::XChangeProperty(
                  context.display,
                  selection.requestor,
                  selection.property,
                  selection.target,
                  8,
                  xlib::PropModeReplace,
                  c_str_bytes.as_ptr(),
                  c_str_bytes.len() as i32,
                );
              }
              None => {
                context.clear_selection();
                break;
              }
            }
          } else {
            debug!("Reply with NONE");
            selection.property = 0;
          }

          xlib::XSendEvent(
            context.display,
            selection.requestor,
            xlib::False,
            xlib::NoEventMask,
            &mut xlib::XEvent { selection } as *mut xlib::XEvent,
          );

          xlib::XSync(context.display, xlib::False);
        }
        xlib::SelectionClear => {
          debug!("Lost ownership");
          break;
        }
        xlib::DestroyNotify => {
          debug!("Window destroyed");

          break;
        }
        ignored => debug!("Ignoring event: {}", ignored),
      }
    }
  }
}
