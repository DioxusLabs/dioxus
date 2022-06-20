#![allow(dead_code)]

pub(crate) mod events;
pub mod adapters {
    #[cfg(feature = "warp")]
    pub mod warp_adapter;

    #[cfg(feature = "axum")]
    pub mod axum_adapter;
}

use std::net::SocketAddr;

use tokio_util::task::LocalPoolHandle;

#[derive(Clone)]
pub struct Liveview {
    pool: LocalPoolHandle,
    addr: String,
}

impl Liveview {
    pub fn body(&self, header: &str) -> String {
        format!(
            r#"
<!DOCTYPE html>
<html>
  <head>
    {header}
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
