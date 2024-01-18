#![doc = include_str!("readme.md")]
#![doc(html_logo_url = "https://avatars.githubusercontent.com/u/79236386")]
#![doc(html_favicon_url = "https://avatars.githubusercontent.com/u/79236386")]
#![deny(missing_docs)]

mod app;
mod assets;
mod config;
mod desktop_context;
mod edits;
mod element;
mod eval;
mod event_handlers;
mod events;
mod file_upload;
mod hooks;
mod ipc;
mod menubar;
mod protocol;
mod query;
mod shortcut;
mod waker;
mod webview;

// mobile shortcut is only supported on mobile platforms
#[cfg(any(target_os = "ios", target_os = "android"))]
mod mobile_shortcut;

/// The main entrypoint for this crate
pub mod launch;

// Reexport tao and wry, might want to re-export other important things
pub use tao;
pub use tao::dpi::{LogicalPosition, LogicalSize};
pub use tao::event::WindowEvent;
pub use tao::window::WindowBuilder;
pub use wry;

// Public exports
pub use assets::AssetRequest;
pub use config::{Config, WindowCloseBehaviour};
pub use desktop_context::{window, DesktopContext, DesktopService};
pub use event_handlers::WryEventHandler;
pub use hooks::{use_asset_handler, use_global_shortcut, use_window, use_wry_event_handler};
pub use shortcut::{ShortcutHandle, ShortcutRegistryError};
pub use wry::RequestAsyncResponder;
