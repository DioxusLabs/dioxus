use std::sync::OnceLock;

use clipboard_rs::Clipboard;

use crate::serve::MessageSource;

const WAYLAND_ENV: &str = "WAYLAND_DISPLAY";
const USER_ERR_MSG: &str = "Failed to copy selection to clipboard.";

static IS_WAYLAND: OnceLock<bool> = OnceLock::new();

pub fn set_content(content: String) {
    let is_wayland = is_wayland();

    if is_wayland {
        use wl_clipboard_rs::copy::{MimeType, Options, Source};
        let opts = Options::new();
        if let Err(e) = opts.copy(Source::Bytes(content.as_bytes().into()), MimeType::Text) {
            tracing::error!(dx_src = ?MessageSource::Dev, err = ?e, "{USER_ERR_MSG}");
        }
    } else {
        use clipboard_rs::ClipboardContext;
        let ctx = match ClipboardContext::new() {
            Ok(c) => c,
            Err(e) => {
                tracing::error!(dx_src = ?MessageSource::Dev, err = e, "{USER_ERR_MSG}");
                return;
            }
        };

        if let Err(e) = ctx.set_text(content) {
            tracing::error!(dx_src = ?MessageSource::Dev, err = e, "{USER_ERR_MSG}");
        }
    }
}

fn is_wayland() -> bool {
    *IS_WAYLAND.get_or_init(|| std::env::var(WAYLAND_ENV).is_ok())
}
