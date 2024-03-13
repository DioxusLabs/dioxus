use std::{
    io::{BufRead, BufReader},
    path::PathBuf,
};

use dioxus_core::Template;
#[cfg(feature = "file_watcher")]
pub use dioxus_html::HtmlCtx;
use interprocess::local_socket::LocalSocketStream;
use serde::{Deserialize, Serialize};

#[cfg(feature = "custom_file_watcher")]
mod file_watcher;
#[cfg(feature = "custom_file_watcher")]
pub use file_watcher::*;

/// A message the hot reloading server sends to the client
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(bound(deserialize = "'de: 'static"))]
pub enum HotReloadMsg {
    /// A template has been updated
    UpdateTemplate(Template),

    /// An asset discovered by rsx! has been updated
    UpdateAsset(PathBuf),

    /// The program needs to be recompiled, and the client should shut down
    Shutdown,
}

/// Connect to the hot reloading listener. The callback provided will be called every time a template change is detected
pub fn connect(mut callback: impl FnMut(HotReloadMsg) + Send + 'static) {
    std::thread::spawn(move || {
        let path = PathBuf::from("./").join("target").join("dioxusin");

        let socket =
            LocalSocketStream::connect(path).expect("Could not connect to hot reloading server.");

        let mut buf_reader = BufReader::new(socket);

        loop {
            let mut buf = String::new();

            if let Err(err) = buf_reader.read_line(&mut buf) {
                if err.kind() != std::io::ErrorKind::WouldBlock {
                    break;
                }
            }

            let template = serde_json::from_str(Box::leak(buf.into_boxed_str())).expect(
                "Could not parse hot reloading message - make sure your client is up to date",
            );

            callback(template);
        }
    });
}

/// Start the hot reloading server with the current directory as the root
#[macro_export]
macro_rules! hot_reload_init {
    () => {
        #[cfg(debug_assertions)]
        dioxus_hot_reload::init(dioxus_hot_reload::Config::new().root(env!("CARGO_MANIFEST_DIR")));
    };

    ($cfg: expr) => {
        #[cfg(debug_assertions)]
        dioxus_hot_reload::init($cfg.root(env!("CARGO_MANIFEST_DIR")));
    };
}
