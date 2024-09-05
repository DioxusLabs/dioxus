mod android;
mod ios;
mod mac;
mod web;
mod win;

mod app;
pub(crate) use app::*;

use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Debug)]
pub(crate) enum BundleFormat {
    // Apple
    Macos,
    Ios,

    // wasm
    Web,

    // Android
    Android,

    // Linux
    AppImage,
    Deb,
    Rpm,

    // Windows
    Msi,
    Wix,
}
