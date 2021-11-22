use wasm_bindgen::JsCast;
use web_sys::window;

pub(crate) fn strip_slash_suffix(path: &str) -> &str {
    path.strip_suffix('/').unwrap_or(path)
}
