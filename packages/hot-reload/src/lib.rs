use std::io::{BufRead, BufReader};

use dioxus_core::Template;
#[cfg(feature = "file_watcher")]
pub use dioxus_html::HtmlCtx;
use interprocess_docfix::local_socket::LocalSocketStream;
use serde::{Deserialize, Serialize};

#[cfg(feature = "file_watcher")]
mod file_watcher;
#[cfg(feature = "file_watcher")]
pub use file_watcher::*;

/// A message the hot reloading server sends to the client
#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub enum HotReloadMsg {
    /// A template has been updated
    #[serde(borrow = "'static")]
    UpdateTemplate(Template<'static>),
    /// The program needs to be recompiled, and the client should shut down
    Shutdown,
}

/// Connect to the hot reloading listener. The callback provided will be called every time a template change is detected
pub fn connect(mut f: impl FnMut(HotReloadMsg) + Send + 'static) {
    std::thread::spawn(move || {
        if let Ok(socket) = LocalSocketStream::connect("@dioxusin") {
            let mut buf_reader = BufReader::new(socket);
            loop {
                let mut buf = String::new();
                match buf_reader.read_line(&mut buf) {
                    Ok(_) => {
                        let template: HotReloadMsg =
                            serde_json::from_str(Box::leak(buf.into_boxed_str())).unwrap();
                        f(template);
                    }
                    Err(err) => {
                        if err.kind() != std::io::ErrorKind::WouldBlock {
                            break;
                        }
                    }
                }
            }
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
