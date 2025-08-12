use js_sys::{
    wasm_bindgen::{JsCast, JsValue},
    ArrayBuffer, Uint8Array,
};
use wasm_bindgen_futures::JsFuture;
use web_sys::{Request, Response};

use crate::WebAssetResolveError;

impl From<js_sys::Error> for WebAssetResolveError {
    fn from(error: js_sys::Error) -> Self {
        WebAssetResolveError { error }
    }
}

impl WebAssetResolveError {
    fn from_js_value(value: JsValue) -> Self {
        if let Some(error) = value.dyn_ref::<js_sys::Error>() {
            WebAssetResolveError::from(error.clone())
        } else {
            unreachable!("Expected a js_sys::Error, got: {:?}", value)
        }
    }
}

pub(crate) async fn resolve_web_asset(path: &str) -> Result<Vec<u8>, WebAssetResolveError> {
    let url = if path.starts_with("/") {
        path.to_string()
    } else {
        format!("/{path}")
    };

    let request = Request::new_with_str(&url).map_err(WebAssetResolveError::from_js_value)?;

    let window = web_sys::window().unwrap();
    let response_promise = JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(WebAssetResolveError::from_js_value)?;
    let response = response_promise.unchecked_into::<Response>();

    let array_buffer_promise = response
        .array_buffer()
        .map_err(WebAssetResolveError::from_js_value)?;
    let array_buffer: ArrayBuffer = JsFuture::from(array_buffer_promise)
        .await
        .map_err(WebAssetResolveError::from_js_value)?
        .unchecked_into();
    let bytes = Uint8Array::new(&array_buffer);
    Ok(bytes.to_vec())
}
