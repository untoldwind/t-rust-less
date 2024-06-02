use std::{
  collections::HashMap,
  error::Error,
  fs::File,
  io::Write,
  os::{fd::AsRawFd, unix::io::FromRawFd},
  sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex, RwLock,
  },
  thread,
};

use log::{debug, error};
use wayland_client::{
  event_created_child,
  globals::{registry_queue_init, BindError, GlobalListContents},
  protocol::{
    wl_registry::WlRegistry,
    wl_seat::{self, WlSeat},
  },
  Connection, Dispatch, EventQueue, Proxy,
};
use wayland_protocols_wlr::data_control::v1::client::{
  zwlr_data_control_device_v1::{self, ZwlrDataControlDeviceV1},
  zwlr_data_control_manager_v1::ZwlrDataControlManagerV1,
  zwlr_data_control_offer_v1::ZwlrDataControlOfferV1,
  zwlr_data_control_source_v1::{self, ZwlrDataControlSourceV1},
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
  open: AtomicBool,
  cancel: AtomicBool,
  provider_holder: RwLock<SelectionProviderHolder>,
}

impl Context {
  fn new<T>(provider: T) -> Self
  where
    T: SelectionProvider + 'static,
  {
    Context {
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

struct State {
  context: Arc<Context>,
  clipboard_manager: ZwlrDataControlManagerV1,
  seats: HashMap<WlSeat, SeatData>,
}

impl Dispatch<WlRegistry, GlobalListContents> for State {
  fn event(
    _state: &mut Self,
    _proxy: &WlRegistry,
    _event: <WlRegistry as wayland_client::Proxy>::Event,
    _data: &GlobalListContents,
    _conn: &wayland_client::Connection,
    _qhandle: &wayland_client::QueueHandle<Self>,
  ) {
  }
}

impl Dispatch<WlSeat, ()> for State {
  fn event(
    _state: &mut Self,
    seat: &WlSeat,
    event: <WlSeat as wayland_client::Proxy>::Event,
    _data: &(),
    _conn: &wayland_client::Connection,
    _qh: &wayland_client::QueueHandle<Self>,
  ) {
    if let wl_seat::Event::Name { name } = event {
      _state.seats.get_mut(seat).unwrap().set_name(name);
    }
  }
}

impl Dispatch<ZwlrDataControlManagerV1, ()> for State {
  fn event(
    _state: &mut Self,
    _proxy: &ZwlrDataControlManagerV1,
    _event: <ZwlrDataControlManagerV1 as wayland_client::Proxy>::Event,
    _data: &(),
    _conn: &wayland_client::Connection,
    _qhandle: &wayland_client::QueueHandle<Self>,
  ) {
  }
}

impl Dispatch<ZwlrDataControlSourceV1, ()> for State {
  fn event(
    _state: &mut Self,
    _proxy: &ZwlrDataControlSourceV1,
    _event: <ZwlrDataControlSourceV1 as wayland_client::Proxy>::Event,
    _data: &(),
    _conn: &wayland_client::Connection,
    _qhandle: &wayland_client::QueueHandle<Self>,
  ) {
    match _event {
      zwlr_data_control_source_v1::Event::Send { mime_type, fd } if TEXT_MIMES.contains(&mime_type.as_str()) => {
        debug!("Event send: {} {:?}", mime_type, fd);
        match _state.context.provider_holder.write() {
          Ok(mut selection_provider) => {
            if let Some(mut content) = selection_provider.get_value() {
              let mut f = unsafe { File::from_raw_fd(fd.as_raw_fd()) };
              f.write_all(content.as_bytes()).ok();
              content.zeroize();
            } else {
              debug!("No more values");
              _state.context.cancel.store(true, Ordering::Relaxed);
            }
          }
          Err(err) => {
            error!("Lock error: {}", err);
            _state.context.cancel.store(true, Ordering::Relaxed);
          }
        }
      }
      zwlr_data_control_source_v1::Event::Cancelled => {
        debug!("Event cancel: Lost ownership");
        _state.context.cancel.store(true, Ordering::Relaxed);
      }
      _ => (),
    }
  }
}

impl Dispatch<ZwlrDataControlDeviceV1, WlSeat> for State {
  fn event(
    state: &mut Self,
    _device: &ZwlrDataControlDeviceV1,
    event: <ZwlrDataControlDeviceV1 as Proxy>::Event,
    seat: &WlSeat,
    _conn: &wayland_client::Connection,
    _qhandle: &wayland_client::QueueHandle<Self>,
  ) {
    match event {
      zwlr_data_control_device_v1::Event::DataOffer { id } => id.destroy(),
      zwlr_data_control_device_v1::Event::Finished => {
        state.seats.get_mut(seat).unwrap().set_device(None);
      }
      _ => (),
    }
  }

  event_created_child!(State, ZwlrDataControlDeviceV1, [
      zwlr_data_control_device_v1::EVT_DATA_OFFER_OPCODE => (ZwlrDataControlOfferV1, ()),
  ]);
}

impl Dispatch<ZwlrDataControlOfferV1, ()> for State {
  fn event(
    _state: &mut Self,
    _offer: &ZwlrDataControlOfferV1,
    _event: <ZwlrDataControlOfferV1 as wayland_client::Proxy>::Event,
    _data: &(),
    _conn: &wayland_client::Connection,
    _qhandle: &wayland_client::QueueHandle<Self>,
  ) {
  }
}

pub struct Clipboard {
  context: Arc<Context>,
  handle: Mutex<Option<thread::JoinHandle<()>>>,
}

impl ClipboardCommon for Clipboard {
  fn new<T>(selection_provider: T, event_hub: Arc<dyn EventHub>) -> ClipboardResult<Self>
  where
    T: SelectionProvider + Clone + 'static,
  {
    let conn = Connection::connect_to_env()?;
    let (globals, mut queue) = registry_queue_init::<State>(&conn)?;
    let qh = &queue.handle();
    let clipboard_manager = match globals.bind(qh, 2..=2, ()) {
      Ok(manager) => manager,
      Err(BindError::NotPresent | BindError::UnsupportedVersion) => globals.bind(qh, 1..=1, ())?,
    };
    let registry = globals.registry();
    let seats = globals.contents().with_list(|globals| {
      globals
        .iter()
        .filter(|global| global.interface == WlSeat::interface().name && global.version >= 2)
        .map(|global| {
          let seat = registry.bind(global.name, 2, qh, ());
          (seat, SeatData::default())
        })
        .collect()
    });

    match selection_provider.current_selection() {
      Some(providing) => event_hub.send(EventData::ClipboardProviding(providing)),
      None => return Err(ClipboardError::Other("Empty provider".to_string())),
    };

    let context = Arc::new(Context::new(selection_provider));
    let mut state = State {
      context: context.clone(),
      clipboard_manager,
      seats,
    };

    queue.roundtrip(&mut state)?;

    let handle = thread::spawn({
      let cloned = context.clone();
      move || {
        if let Err(err) = try_run(queue, state) {
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

fn try_run(mut queue: EventQueue<State>, mut state: State) -> Result<(), Box<dyn Error>> {
  let data_source = state.clipboard_manager.create_data_source(&queue.handle(), ());

  debug!("Seats: {:?}", &state.seats);

  for &mime_type in TEXT_MIMES {
    data_source.offer(mime_type.to_string());
  }

  for (seat, data) in &mut state.seats {
    let device = state
      .clipboard_manager
      .get_data_device(seat, &queue.handle(), seat.clone());
    device.set_selection(Some(&data_source));
    data.set_device(Some(device));
  }

  debug!("Start event loop");
  state.context.open.store(true, Ordering::Relaxed);
  while !state.context.cancel.load(Ordering::Relaxed) {
    queue.blocking_dispatch(&mut state)?;
  }
  state.context.open.store(false, Ordering::Relaxed);
  debug!("End event loop");

  for data in state.seats.values_mut() {
    data.set_device(None);
  }
  data_source.destroy();

  Ok(())
}

#[derive(Default, Debug)]
pub struct SeatData {
  pub name: Option<String>,

  pub device: Option<ZwlrDataControlDeviceV1>,
}

impl SeatData {
  pub fn set_name(&mut self, name: String) {
    self.name = Some(name)
  }

  pub fn set_device(&mut self, device: Option<ZwlrDataControlDeviceV1>) {
    let old_device = self.device.take();
    self.device = device;

    if let Some(device) = old_device {
      device.destroy();
    }
  }
}
