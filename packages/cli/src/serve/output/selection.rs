use crate::serve::MessageSource;
use clipboard_rs::{Clipboard, ClipboardContext};
use std::fmt::Write as _;
use std::sync::OnceLock;

const WAYLAND_ENV: &str = "WAYLAND_DISPLAY";
const USER_ERR_MSG: &str = "Failed to copy selection to clipboard.";

static IS_WAYLAND: OnceLock<bool> = OnceLock::new();
static CLIPBOARD_CTX: OnceLock<ClipboardContext> = OnceLock::new();

/// Perform minor text processing on user selection, compiling it into a single [`String`].
pub fn process_selection(selected_lines: &mut [String]) -> String {
    let mut final_text = String::new();

    // Go through each line, parse it, and append it to the final text.
    for line in selected_lines.iter_mut() {
        // Trim scroll bar and whitespace.
        *line = line.replace("▐", "").trim_end().to_string();
        if !line.is_empty() {
            writeln!(final_text, "{line}").unwrap();
        }
    }

    // Remove the last unescessary newline.
    final_text.trim_end_matches('\n').to_string()
}

/// Set the clipboard content.
///
/// This automatically routes to the correct clipboard provider
/// depending on the desktop environment.
pub fn set_clipboard(content: String) {
    let is_wayland = is_wayland();

    if is_wayland {
        #[cfg(target_family = "unix")]
        {
            use wl_clipboard_rs::copy::{MimeType, Options, Source};
            let opts = Options::new();
            if let Err(e) = opts.copy(Source::Bytes(content.as_bytes().into()), MimeType::Text) {
                tracing::error!(dx_src = ?MessageSource::Dev, err = ?e, "{USER_ERR_MSG}");
            }
        }
    } else {
        let ctx = get_generic_clipboard();

        if let Err(e) = ctx.set_text(content) {
            tracing::error!(dx_src = ?MessageSource::Dev, err = e, "{USER_ERR_MSG}");
        }
    }
}

/// Gets a context to the generic clipboard provider.
///
/// Creates a new clipboard context if it doesn't exist. This is required because
/// subsequent calls to `set_text` with a new clipboard context will cause
/// the underlying library to `println!` that it no longer owns the clipboard.
///
/// # Panics
/// This function will panic if the clipboard context fails to initialize.
fn get_generic_clipboard() -> &'static ClipboardContext {
    CLIPBOARD_CTX.get_or_init(|| match ClipboardContext::new() {
        Ok(c) => c,
        Err(e) => {
            tracing::error!(err = e, "Failed to init clipboard context.");
            panic!("Failed to initialize clipboard: {e}");
        }
    })
}

/// Check if the current desktop environment is Wayland.
///
/// This function simply checks if the `WAYLAND_DISPLAY` environment
/// variable exists.
fn is_wayland() -> bool {
    *IS_WAYLAND.get_or_init(|| std::env::var(WAYLAND_ENV).is_ok())
}
