use serde::de::DeserializeOwned;

use base64::engine::general_purpose::STANDARD;
use base64::Engine;

#[allow(unused)]
pub(crate) fn serde_from_bytes<T: DeserializeOwned>(string: &[u8]) -> Option<T> {
    let decompressed = STANDARD.decode(string).ok()?;

    postcard::from_bytes(&decompressed).ok()
}

#[cfg(not(feature = "ssr"))]
/// Get the props from the document. This is only available in the browser.
///
/// When dioxus-fullstack renders the page, it will serialize the root props and put them in the document. This function gets them from the document.
pub fn get_root_props_from_document<T: DeserializeOwned>() -> Option<T> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        None
    }
    #[cfg(target_arch = "wasm32")]
    {
        let attribute = web_sys::window()?
            .document()?
            .get_element_by_id("dioxus-storage")?
            .get_attribute("data-serialized")?;

        serde_from_string(&attribute)
    }
}
