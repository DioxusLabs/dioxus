use std::net::SocketAddr;

use tokio_util::task::LocalPoolHandle;

#[derive(Clone)]
pub struct LiveView {
    pub(crate) pool: LocalPoolHandle,
    pub(crate) addr: String,
}

impl LiveView {
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

    pub fn interpreter_code(&self) -> String {
        include_str!("../src/interpreter.js").to_string()
    }

    pub fn new(addr: impl Into<SocketAddr>) -> Self {
        let addr: SocketAddr = addr.into();

        LiveView {
            pool: LocalPoolHandle::new(16),
            addr: addr.to_string(),
        }
    }
}
