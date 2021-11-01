use std::{sync::mpsc, thread, time::Duration};

use dropbox_sdk::{
  default_client::NoauthDefaultClient,
  oauth2::{Authorization, AuthorizeUrlBuilder, Oauth2Type, PkceCode},
};
use log::error;
use tiny_http::{Header, Response, Server};
use url::Url;

use crate::block_store::{StoreError, StoreResult};

const APP_KEY: &str = "3q0sff542l6r3ly";
const REDIRECT_URL: &str = "http://127.0.0.1:9898";

const AUTHCODE_RESPONSE_BODY: &str = r#"
<!DOCTYPE html>
<html>
<head>
<title>t-rust-less</title>
</head>
<body">
<p style="text-align: center;">&nbsp;</p>
<p style="text-align: center;">T-Rust-Less is now authenticated to Dropbox!</p>
<p style="text-align: center;">You may now close this window...</p>
</body>
</html>
"#;

pub struct DropboxInitializer {
  oauth2_flow: Oauth2Type,
  pub auth_url: Url,
  auth_code_receiver: mpsc::Receiver<Result<String, String>>,
  server_shutdown: mpsc::Sender<()>,
}

impl DropboxInitializer {
  pub fn wait_for_authentication(&self) -> StoreResult<()> {
    match self.auth_code_receiver.recv()? {
      Ok(authcode_url) => {
        println!("{}", authcode_url);
        let auth_code = Url::parse(&authcode_url)?
          .query_pairs()
          .find_map(|(key, value)| if key == "code" { Some(value.to_string()) } else { None })
          .ok_or_else(|| StoreError::IO("auth url does not contain code".to_string()))?;
        println!("{}", auth_code);
        let mut authorization = Authorization::from_auth_code(
          APP_KEY.to_string(),
          self.oauth2_flow.clone(),
          auth_code,
          Some(REDIRECT_URL.to_string()),
        );
        authorization.obtain_access_token(NoauthDefaultClient::default())?;
        println!("{:?}", authorization.save());
      }
      Err(err) => {
        error!("Failed receiving dropbox authcode {}", err);
        return Err(StoreError::IO(err));
      }
    }
    Ok(())
  }
}

impl Drop for DropboxInitializer {
  fn drop(&mut self) {
    self.server_shutdown.send(()).ok();
  }
}

pub fn initialize_store() -> StoreResult<DropboxInitializer> {
  let oauth2_flow = Oauth2Type::PKCE(PkceCode::new());
  let auth_url = AuthorizeUrlBuilder::new(APP_KEY, &oauth2_flow)
    .redirect_uri(REDIRECT_URL)
    .build();
  let (auth_code_receiver, server_shutdown) = start_authcode_server();

  Ok(DropboxInitializer {
    oauth2_flow,
    auth_url,
    auth_code_receiver,
    server_shutdown,
  })
}

pub fn start_authcode_server() -> (mpsc::Receiver<Result<String, String>>, mpsc::Sender<()>) {
  let (tx_shutdown, rx_shutdown) = mpsc::channel::<()>();
  let (tx, rx) = mpsc::channel::<Result<String, String>>();

  thread::spawn(move || {
    let server = match Server::http("127.0.0.1:9898") {
      Ok(server) => server,
      Err(err) => {
        tx.send(Err(format!("{}", err))).ok();
        return;
      }
    };
    let poll_duration = Duration::from_millis(100);

    loop {
      if rx_shutdown.recv_timeout(poll_duration).is_ok() {
        tx.send(Err("Shutdown requested".to_string())).ok();
        return;
      }
      match server.try_recv() {
        Ok(Some(request)) => {
          tx.send(Ok(format!("{}{}", REDIRECT_URL, request.url()))).ok();
          request
            .respond(
              Response::from_data(AUTHCODE_RESPONSE_BODY)
                .with_header(Header::from_bytes(&b"Content-Type"[..], &b"text/html; charset=UTF-8"[..]).unwrap()),
            )
            .ok();
          return;
        }
        Ok(None) => (),
        Err(err) => {
          tx.send(Err(format!("{}", err))).ok();
          return;
        }
      }
    }
  });

  (rx, tx_shutdown)
}
