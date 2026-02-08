use std::{
  sync::Arc,
  thread::{self, JoinHandle},
};

use dropbox_sdk::{
  default_client::NoauthDefaultClient,
  oauth2::{Authorization, AuthorizeUrlBuilder, Oauth2Type, PkceCode},
};
use log::error;
use tiny_http::{Header, Response, Server};
use url::Url;

use super::APP_KEY;
use crate::error::{SyncError, SyncResult};

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

pub struct ServerHandle {
  server: Arc<Server>,
  join_handle: Option<JoinHandle<Result<String, String>>>,
}

impl ServerHandle {
  fn wait_for_auth_code(&mut self) -> SyncResult<String> {
    let join_handle = match self.join_handle.take() {
      Some(join_handle) => join_handle,
      None => return Err(SyncError::Generic("Already waiting".to_string())),
    };
    match join_handle.join() {
      Ok(Ok(authcode_url)) => Ok(
        Url::parse(&authcode_url)?
          .query_pairs()
          .find_map(|(key, value)| if key == "code" { Some(value.to_string()) } else { None })
          .ok_or_else(|| SyncError::Generic("auth url does not contain code".to_string()))?,
      ),
      Ok(Err(err)) => {
        error!("Failed receiving dropbox authcode {err}");
        Err(SyncError::Generic(err))
      }
      Err(err) => {
        error!("Failed receiving dropbox authcode {err:?}");
        Err(SyncError::Generic(format!("{err:?}")))
      }
    }
  }
}

impl Drop for ServerHandle {
  fn drop(&mut self) {
    self.server.unblock();
  }
}

pub struct DropboxInitializer {
  oauth2_flow: Oauth2Type,
  pub auth_url: Url,
  server_handle: ServerHandle,
}

impl DropboxInitializer {
  pub fn new() -> SyncResult<Self> {
    let oauth2_flow = Oauth2Type::PKCE(PkceCode::new());
    let auth_url = AuthorizeUrlBuilder::new(APP_KEY, &oauth2_flow)
      .redirect_uri(REDIRECT_URL)
      .build();
    let server_handle = start_authcode_server()?;

    Ok(DropboxInitializer {
      oauth2_flow,
      auth_url,
      server_handle,
    })
  }
  pub fn wait_for_authentication(mut self) -> SyncResult<String> {
    let auth_code = self.server_handle.wait_for_auth_code()?;

    let mut authorization = Authorization::from_auth_code(
      APP_KEY.to_string(),
      self.oauth2_flow.clone(),
      auth_code,
      Some(REDIRECT_URL.to_string()),
    );
    authorization.obtain_access_token(NoauthDefaultClient::default())?;
    let token = authorization
      .save()
      .ok_or_else(|| SyncError::Generic("Failed to obtain dropbox token".to_string()))?;

    Ok(token)
  }
}

pub fn start_authcode_server() -> SyncResult<ServerHandle> {
  let server = Arc::new(Server::http("127.0.0.1:9898").map_err(|e| SyncError::Generic(format!("{e}")))?);
  let server_cloned = server.clone();

  let join_handle = thread::spawn(move || {
    let request = server_cloned.recv().map_err(|e| format!("{e}"))?;
    let url = format!("{}{}", REDIRECT_URL, request.url());
    request
      .respond(
        Response::from_data(AUTHCODE_RESPONSE_BODY)
          .with_header(Header::from_bytes(&b"Content-Type"[..], &b"text/html; charset=UTF-8"[..]).unwrap()),
      )
      .ok();

    Ok(url)
  });

  Ok(ServerHandle {
    server,
    join_handle: Some(join_handle),
  })
}
