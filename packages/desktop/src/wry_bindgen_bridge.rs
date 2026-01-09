//! wry-bindgen integration for dioxus desktop.
//!
//! This module provides the bridge between dioxus desktop and wry-bindgen,
//! enabling typed Rust<->JS communication through wry-bindgen's binary protocol.
//!
//! Currently this module is a placeholder. As features migrate from the existing
//! communication mechanisms (WebSocket for mutations, HTTP POST for events, IPC for
//! queries) to wry-bindgen, the typed bindings will be defined here.

// Example: typed event handler (for future migration from HTTP POST)
// #[wasm_bindgen(crate = wry_bindgen, inline_js = r#"
// export function setEventHandler(handler) {
//     window.rustEventHandler = handler;
// }
// "#)]
// extern "C" {
//     fn set_event_handler(handler: &Closure<dyn FnMut(String) -> String>);
// }

// Example: typed query callback (for future migration from IPC)
// #[wasm_bindgen(crate = wry_bindgen, inline_js = r#"
// export function setQueryCallback(handler) {
//     window.rustQueryCallback = handler;
// }
// "#)]
// extern "C" {
//     fn set_query_callback(handler: &Closure<dyn FnMut(u32, String)>);
// }
