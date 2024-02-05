use serde::de::DeserializeOwned;

use base64::engine::general_purpose::STANDARD;
use base64::Engine;

use super::HTMLDataCursor;

#[allow(unused)]
pub(crate) fn serde_from_bytes<T: DeserializeOwned>(string: &[u8]) -> Option<T> {
    let decompressed = match STANDARD.decode(string) {
        Ok(bytes) => bytes,
        Err(err) => {
            tracing::error!("Failed to decode base64: {}", err);
            return None;
        }
    };

    match ciborium::from_reader(std::io::Cursor::new(decompressed)) {
        Ok(data) => Some(data),
        Err(err) => {
            tracing::error!("Failed to deserialize: {}", err);
            None
        }
    }
}

static SERVER_DATA: once_cell::sync::Lazy<Option<HTMLDataCursor>> =
    once_cell::sync::Lazy::new(|| {
        #[cfg(all(feature = "web", target_arch = "wasm32"))]
        {
            let window = web_sys::window()?.document()?;
            let element = match window.get_element_by_id("dioxus-storage-data") {
                Some(element) => element,
                None => {
                    tracing::error!("Failed to get element by id: dioxus-storage-data");
                    return None;
                }
            };
            let attribute = match element.get_attribute("data-serialized") {
                Some(attribute) => attribute,
                None => {
                    tracing::error!("Failed to get attribute: data-serialized");
                    return None;
                }
            };

            let data: super::HTMLData = serde_from_bytes(attribute.as_bytes())?;

            Some(data.cursor())
        }
        #[cfg(not(all(feature = "web", target_arch = "wasm32")))]
        {
            None
        }
    });

pub(crate) fn take_server_data<T: DeserializeOwned>() -> Option<T> {
    SERVER_DATA.as_ref()?.take()
}

#[cfg(not(feature = "server"))]
/// Get the props from the document. This is only available in the browser.
///
/// When dioxus-fullstack renders the page, it will serialize the root props and put them in the document. This function gets them from the document.
pub fn get_root_props_from_document<T: DeserializeOwned>() -> Option<T> {
    #[cfg(all(feature = "web", target_arch = "wasm32"))]
    {
        let attribute = web_sys::window()?
            .document()?
            .get_element_by_id("dioxus-storage-props")?
            .get_attribute("data-serialized")?;

        serde_from_bytes(attribute.as_bytes())
    }
    #[cfg(not(all(feature = "web", target_arch = "wasm32")))]
    {
        None
    }
}
