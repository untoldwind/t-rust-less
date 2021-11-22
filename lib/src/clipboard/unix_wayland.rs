use std::{
  cell::RefCell,
  error::Error,
  fs::File,
  io::Write,
  os::unix::io::FromRawFd,
  rc::Rc,
  sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex, RwLock,
  },
  thread,
};

use log::{debug, error};
use wayland_client::{global_filter, protocol::wl_seat::WlSeat, Display, GlobalManager, Main};
use wayland_protocols::wlr::unstable::data_control::v1::client::{
  zwlr_data_control_manager_v1::ZwlrDataControlManagerV1, zwlr_data_control_source_v1::Event,
};
use zeroize::Zeroize;

use crate::api::{ClipboardProviding, EventData, EventHub};
use crate::clipboard::selection_provider_holder::SelectionProviderHolder;

use super::{ClipboardCommon, ClipboardError, ClipboardResult, SelectionProvider};

const TEXT_MIMES: &[&str] = &[
  "text/plain;charset=utf-8",
  "text/plain",
  "STRING",
  "UTF8_STRING",
  "TEXT",
];

struct Context {
  display: Display,
  open: AtomicBool,
  cancel: AtomicBool,
  provider_holder: RwLock<SelectionProviderHolder>,
}

impl Context {
  fn new<T>(display: Display, provider: T) -> Self
  where
    T: SelectionProvider + 'static,
  {
    Context {
      display,
      open: AtomicBool::new(false),
      cancel: AtomicBool::new(false),
      provider_holder: RwLock::new(SelectionProviderHolder::new(provider)),
    }
  }

  fn is_open(&self) -> bool {
    self.open.load(Ordering::Relaxed)
  }

  fn currently_providing(&self) -> Option<ClipboardProviding> {
    self.provider_holder.read().ok()?.current_selection()
  }

  fn provide_next(&self) {
    if let Ok(mut provider_holder) = self.provider_holder.write() {
      provider_holder.get_value();
    }
  }

  fn destroy(&self) {
    self.cancel.store(true, Ordering::Relaxed)
  }
}

pub struct Clipboard {
  context: Arc<Context>,
  handle: Mutex<Option<thread::JoinHandle<()>>>,
}

impl ClipboardCommon for Clipboard {
  fn new<T>(display_name: &str, selection_provider: T, event_hub: Arc<dyn EventHub>) -> ClipboardResult<Self>
  where
    T: SelectionProvider + Clone + 'static,
  {
    let display = Display::connect_to_name(display_name)?;
    match selection_provider.current_selection() {
      Some(providing) => event_hub.send(EventData::ClipboardProviding(providing)),
      None => return Err(ClipboardError::Other("Empty provider".to_string())),
    };

    let context = Arc::new(Context::new(display, selection_provider));

    let handle = thread::spawn({
      let cloned = context.clone();
      move || {
        if let Err(err) = try_run(cloned.clone()) {
          cloned.open.store(false, Ordering::Relaxed);
          error!("Wayland clipboard error: {}", err);
        }
      }
    });

    Ok(Clipboard {
      context,
      handle: Mutex::new(Some(handle)),
    })
  }

  fn is_open(&self) -> bool {
    self.context.is_open()
  }

  fn currently_providing(&self) -> Option<ClipboardProviding> {
    self.context.currently_providing()
  }

  fn provide_next(&self) {
    self.context.provide_next()
  }

  fn destroy(&self) {
    self.context.destroy()
  }

  fn wait(&self) -> ClipboardResult<()> {
    let mut maybe_handle = self.handle.lock()?;
    if let Some(handle) = maybe_handle.take() {
      handle
        .join()
        .map_err(|_| ClipboardError::Other("wait timeout".to_string()))?;
    }
    Ok(())
  }
}

fn try_run(context: Arc<Context>) -> Result<(), Box<dyn Error>> {
  let mut queue = context.display.create_event_queue();
  let display = context.display.attach(queue.token());
  let seats = Rc::new(RefCell::new(vec![]));
  let mut devices = vec![];

  let seats_cloned = seats.clone();

  let manager = GlobalManager::new_with_cb(
    &display,
    global_filter!([WlSeat, 2, move |seat: Main<WlSeat>, _: DispatchData| {
      seats_cloned.borrow_mut().push(seat);
    }]),
  );

  queue.sync_roundtrip(&mut (), |_, _, _| {})?;

  let clipboard_manager: Main<ZwlrDataControlManagerV1> = manager.instantiate_exact(1).unwrap();

  let data_source = clipboard_manager.create_data_source();
  let context_cloned = context.clone();

  data_source.quick_assign(move |_, event, _| match event {
    Event::Send { mime_type, fd } if TEXT_MIMES.contains(&mime_type.as_str()) => {
      match context_cloned.provider_holder.write() {
        Ok(mut selection_provider) => {
          if let Some(mut content) = selection_provider.get_value() {
            let mut f = unsafe { File::from_raw_fd(fd) };
            f.write_all(content.as_bytes()).ok();
            content.zeroize();
          } else {
            debug!("No more values");
            context_cloned.cancel.store(true, Ordering::Relaxed);
          }
        }
        Err(err) => {
          error!("Lock error: {}", err);
          context_cloned.cancel.store(true, Ordering::Relaxed);
        }
      }
    }
    Event::Cancelled => {
      debug!("Lost ownership");
      context_cloned.cancel.store(true, Ordering::Relaxed)
    }
    _ => (),
  });

  for &mime_type in TEXT_MIMES {
    data_source.offer(mime_type.to_string());
  }

  for seat in seats.borrow_mut().iter_mut() {
    let device = clipboard_manager.get_data_device(seat);
    device.quick_assign(|_, _, _| {});
    device.set_selection(Some(&data_source));
    devices.push(device);
  }

  debug!("Start event loop");
  context.open.store(true, Ordering::Relaxed);
  while !context.cancel.load(Ordering::Relaxed) {
    queue.dispatch(&mut (), |_, _, _| {})?;
  }
  context.open.store(false, Ordering::Relaxed);
  debug!("End event loop");

  for device in devices {
    device.destroy();
  }
  data_source.destroy();
  for seat in seats.borrow_mut().iter_mut() {
    seat.detach();
  }
  display.detach();

  Ok(())
}
