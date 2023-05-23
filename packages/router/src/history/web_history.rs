use gloo::console::error;
use gloo_utils::format::JsValueSerdeExt;
use serde::{de::DeserializeOwned, Serialize};
use wasm_bindgen::JsValue;
use web_sys::History;

pub(crate) fn replace_state_with_url<V: Serialize>(
    history: &History,
    value: &V,
    url: Option<&str>,
) -> Result<(), JsValue> {
    let position = JsValue::from_serde(value).unwrap();

    history.replace_state_with_url(&position, "", url)
}

pub(crate) fn push_state_and_url<V: Serialize>(
    history: &History,
    value: &V,
    url: String,
) -> Result<(), JsValue> {
    let position = JsValue::from_serde(value).unwrap();

    history.push_state_with_url(&position, "", Some(&url))
}

pub(crate) fn get_current<V: DeserializeOwned>(history: &History) -> Option<V> {
    let state = history.state();
    if let Err(err) = &state {
        error!(err);
    }
    state.ok().and_then(|state| {
        let deserialized = state.into_serde();
        if let Err(err) = &deserialized {
            error!(format!("{}", err));
        }
        deserialized.ok()
    })
}
