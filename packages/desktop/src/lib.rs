#![doc = include_str!("readme.md")]
#![doc(html_logo_url = "https://avatars.githubusercontent.com/u/79236386")]
#![doc(html_favicon_url = "https://avatars.githubusercontent.com/u/79236386")]
#![deny(missing_docs)]
#![cfg_attr(docsrs, feature(doc_cfg))]

mod android_sync_lock;
mod app;
mod assets;
mod config;
mod default_icon;
mod desktop_context;
mod document;
mod edits;
mod element;
mod event_handlers;
mod events;
mod file_upload;
mod hooks;
mod ipc;
mod menubar;
mod mobile;
mod protocol;
mod query;
mod shortcut;
mod waker;
mod webview;

pub use default_icon::{icon_from_memory, icon_from_path}; // default_icon,

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
// Reexport muda only if we are on desktop platforms that support menus
#[cfg(not(any(target_os = "ios", target_os = "android")))]
pub use muda;

// Tray icon
#[cfg(any(target_os = "windows", target_os = "linux", target_os = "macos"))]
pub mod trayicon;

// Public exports
pub use assets::AssetRequest;
pub use config::{Config, WindowCloseBehaviour};
pub use desktop_context::{
    window, DesktopContext, DesktopService, PendingDesktopContext, WeakDesktopContext,
};
pub use event_handlers::WryEventHandler;
pub use hooks::*;
pub use shortcut::{HotKeyState, ShortcutHandle, ShortcutRegistryError};
pub use wry::RequestAsyncResponder;
