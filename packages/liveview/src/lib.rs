#![allow(dead_code)]

pub(crate) mod events;
pub mod adapters {
    #[cfg(feature = "warp")]
    pub mod warp_adapter;

    #[cfg(feature = "axum")]
    pub mod axum_adapter;

    #[cfg(feature = "actix")]
    pub mod actix_adapter;
}

use std::net::SocketAddr;

#[cfg(feature = "warp")]
pub use adapters::warp_adapter::connect;

#[cfg(feature = "axum")]
pub use adapters::axum_adapter::connect;

#[cfg(feature = "actix")]
pub use adapters::actix_adapter::connect;
use tokio_util::task::LocalPoolHandle;

#[derive(Clone)]
pub struct Liveview {
    pool: LocalPoolHandle,
    addr: String,
}

impl Liveview {
    pub fn body(&self) -> String {
        format!(
            r#"
<!DOCTYPE html>
<html>
  <head>
    <title>Dioxus app</title>
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
  </head>
  <body>
    <div id="main"></div>
    <script>
      var WS_ADDR = "ws://{addr}/app";
      {interpreter}
      main();
    </script>
  </body>
</html>"#,
            addr = self.addr,
            interpreter = include_str!("../src/interpreter.js")
        )
    }
}

pub fn new(addr: impl Into<SocketAddr>) -> Liveview {
    let addr: SocketAddr = addr.into();

    Liveview {
        pool: LocalPoolHandle::new(16),
        addr: addr.to_string(),
    }
}
