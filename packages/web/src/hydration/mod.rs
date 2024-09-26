#[cfg(feature = "hydrate")]
mod deserialize;
#[cfg(feature = "hydrate")]
mod hydrate;

#[cfg(feature = "hydrate")]
pub use deserialize::*;

#[cfg(feature = "hydrate")]
#[allow(unused)]
pub use hydrate::*;

/// The message sent from the server to the client to hydrate a suspense boundary
#[derive(Debug)]
pub(crate) struct SuspenseMessage {
    #[cfg(feature = "hydrate")]
    /// The path to the suspense boundary. Each element in the path is an index into the children of the suspense boundary (or the root node) in the order they are first created
    suspense_path: Vec<u32>,
    #[cfg(feature = "hydrate")]
    /// The data to hydrate the suspense boundary with
    data: Vec<u8>,
}

#[cfg(feature = "hydrate")]
// Get the initial hydration data from the client
#[wasm_bindgen::prelude::wasm_bindgen(inline_js = r#"
    export function get_initial_hydration_data() {
        if (window.initial_dioxus_hydration_data === undefined) {
            return new Uint8Array();
        }

        const decoded = atob(window.initial_dioxus_hydration_data);
        return Uint8Array.from(decoded, (c) => c.charCodeAt(0))
    }
"#)]
extern "C" {
    /// Get the initial hydration data from the client
    pub fn get_initial_hydration_data() -> js_sys::Uint8Array;
}
