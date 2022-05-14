mod context;
mod event;
mod hooks;
mod plugin;
mod runner;
mod setting;
mod window;

pub use dioxus_desktop::cfg::DesktopConfig;

pub mod prelude {
    pub use crate::{hooks::*, plugin::DioxusDesktopPlugin, DesktopConfig};
}
