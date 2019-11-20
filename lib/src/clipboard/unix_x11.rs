#![cfg(all(unix, feature = "with_x11"))]

use crate::clipboard::debounce::SelectionDebounce;
use crate::clipboard::{ClipboardError, ClipboardResult, SelectionProvider};
use log::debug;
use std::ffi::CString;
use std::mem::MaybeUninit;
use std::sync::atomic::{AtomicBool, Ordering};
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
}

struct Context {
  display: *mut xlib::Display,
  window: xlib::Window,
  atoms: Atoms,
  open: AtomicBool,
  providing: RwLock<Option<String>>,
}

impl Context {
  fn new(display_name: &str) -> ClipboardResult<Self> {
    unsafe {
      let c_display_name = CString::new(display_name)?;
      let display = xlib::XOpenDisplay(c_display_name.as_ptr());

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

      let atoms = Atoms {
        primary,
        clipboard,
        targets,
        string,
        utf8_string,
      };

      debug!("{:?}", atoms);

      Ok(Context {
        display,
        window,
        atoms,
        open: AtomicBool::new(true),
        providing: RwLock::new(None),
      })
    }
  }

  fn get_atom(display: *mut xlib::Display, name: &str) -> xlib::Atom {
    unsafe {
      let c_name = CString::new(name).unwrap();
      xlib::XInternAtom(display, c_name.as_ptr(), xlib::False)
    }
  }

  fn destroy(&self) {
    if self.open.swap(false, Ordering::Relaxed) {
      unsafe {
        xlib::XDestroyWindow(self.display, self.window);
        xlib::XFlush(self.display);
      }
    }
  }

  fn own_selection(&self) -> bool {
    unsafe {
      xlib::XSetSelectionOwner(self.display, self.atoms.clipboard, self.window, xlib::CurrentTime);

      let owner = xlib::XGetSelectionOwner(self.display, self.atoms.clipboard);
      if owner != self.window {
        debug!("Failed taking ownership of {}", self.atoms.clipboard);
        return false;
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

  fn is_open(&self) -> bool {
    self.open.load(Ordering::Relaxed)
  }

  fn currently_providing(&self) -> Option<String> {
    self.providing.read().unwrap().clone()
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
  pub fn new<T>(display_name: &str, selection_provider: T) -> ClipboardResult<Clipboard>
  where
    T: SelectionProvider + 'static,
  {
    let context = Arc::new(Context::new(display_name)?);

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

  pub fn is_open(&self) -> bool {
    self.context.is_open()
  }

  pub fn currently_providing(&self) -> Option<String> {
    self.context.currently_providing()
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

fn run<T>(context: Arc<Context>, selection_provider: T)
where
  T: SelectionProvider,
{
  let mut debounce = SelectionDebounce::new(selection_provider);

  unsafe {
    if !context.own_selection() {
      return;
    }

    let mut event: xlib::XEvent = MaybeUninit::zeroed().assume_init();

    loop {
      *context.providing.write().unwrap() = debounce.current_selection_name();

      xlib::XFlush(context.display);
      debug!("Wating for event");
      xlib::XNextEvent(context.display, &mut event);

      debug!("Got event: {}", event.get_type());

      match event.get_type() {
        xlib::SelectionRequest => {
          let mut selection: xlib::XSelectionEvent = MaybeUninit::zeroed().assume_init();
          selection.type_ = xlib::SelectionNotify;
          selection.display = event.selection_request.display;
          selection.requestor = event.selection_request.requestor;
          selection.selection = event.selection_request.selection;
          selection.time = event.selection_request.time;
          selection.target = event.selection_request.target;
          selection.property = event.selection_request.property;

          debug!("Selection target: {}", selection.target);

          if selection.target == context.atoms.targets {
            let atoms = [context.atoms.targets, context.atoms.string, context.atoms.utf8_string];
            xlib::XChangeProperty(
              context.display,
              selection.requestor,
              selection.property,
              xlib::XA_ATOM,
              32,
              xlib::PropModeReplace,
              &atoms as *const xlib::Atom as *const u8,
              atoms.len() as i32,
            );
          } else if selection.target == context.atoms.string || selection.target == context.atoms.utf8_string {
            match debounce.get_selection() {
              Some(value) => {
                let content: &[u8] = value.as_ref();

                xlib::XChangeProperty(
                  context.display,
                  selection.requestor,
                  selection.property,
                  selection.target,
                  8,
                  xlib::PropModeReplace,
                  content.as_ptr(),
                  content.len() as i32,
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

    debug!("Ending event loop");
    context.destroy();
  }
}
