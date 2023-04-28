use serde::de::DeserializeOwned;

use super::u16_from_char;

#[allow(unused)]
pub(crate) fn serde_from_string<T: DeserializeOwned>(string: &str) -> Option<T> {
    let decompressed = string
        .chars()
        .flat_map(|c| {
            let u = u16_from_char(c);
            let u1 = (u >> 8) as u8;
            let u2 = (u & 0xFF) as u8;
            [u1, u2].into_iter()
        })
        .collect::<Vec<u8>>();
    let (decompressed, _) = yazi::decompress(&decompressed, yazi::Format::Zlib).unwrap();

    postcard::from_bytes(&decompressed).ok()
}

#[cfg(not(feature = "ssr"))]
/// Get the props from the document. This is only available in the browser.
///
/// When dioxus-server renders the page, it will serialize the root props and put them in the document. This function gets them from the document.
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
