#![cfg_attr(docsrs, feature(doc_cfg))]

//! A native renderer for Dioxus.
//!
//! ## Feature flags
//!  - `default`: Enables the features listed below.
//!  - `accessibility`: Enables [`accesskit`](https://docs.rs/accesskit/latest/accesskit/) accessibility support.
//!  - `hot-reload`: Enables hot-reloading of Dioxus RSX.
//!  - `menu`: Enables the [`muda`](https://docs.rs/muda/latest/muda/) menubar.
//!  - `tracing`: Enables tracing support.
//!  - `shell`: Enables window and event-loop integration with blitz-shell and winit.

mod config;
mod dioxus_renderer;
#[cfg(feature = "shell")]
mod shell;

#[cfg(feature = "prelude")]
pub mod prelude;

#[doc(inline)]
pub use dioxus_native_dom::*;

pub use dioxus_renderer::DioxusNativeWindowRenderer;

#[cfg(any(feature = "vello", feature = "vello-hybrid"))]
pub use {
    dioxus_renderer::{Features, Limits},
    wgpu_context::DeviceHandle,
};

pub use blitz_dom::{FontContext, Widget, build_single_font_ctx};
pub use config::Config;

#[cfg(feature = "shell")]
pub use shell::*;
