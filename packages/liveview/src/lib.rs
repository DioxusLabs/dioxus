#![allow(dead_code)]

pub static INTERPRETER: &str = include_str!("interpreter.js");

pub fn interpreter_glue(url: &str) -> String {
    format!(
        r#"
<script>
    var WS_ADDR = "{url}";
    {INTERPRETER}
    main();
</script>
    "#
    )
}

pub(crate) mod events;

pub mod adapters {
    #[cfg(feature = "warp")]
    pub mod warp_adapter;

    #[cfg(feature = "axum")]
    pub mod axum_adapter;

    #[cfg(feature = "salvo")]
    pub mod salvo_adapter;
}

use std::net::SocketAddr;

use tokio_util::task::LocalPoolHandle;

pub mod pool;
pub use pool::*;

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum LiveViewError {}
