use gloo::console::error;
#[cfg(feature = "serde")]
use gloo_utils::format::JsValueSerdeExt;
use wasm_bindgen::JsValue;
use web_sys::History;

#[cfg(not(feature = "serde"))]
pub(crate) fn replace_state_with_url(
    history: &History,
    value: &[f64; 2],
    url: Option<&str>,
) -> Result<(), JsValue> {
    let position = js_sys::Array::new();
    position.push(&JsValue::from(value[0]));
    position.push(&JsValue::from(value[1]));

    history.replace_state_with_url(&position, "", url)
}

#[cfg(feature = "serde")]
pub(crate) fn replace_state_with_url<V: serde::Serialize>(
    history: &History,
    value: &V,
    url: Option<&str>,
) -> Result<(), JsValue> {
    let position = JsValue::from_serde(value).unwrap();

    history.replace_state_with_url(&position, "", url)
}

#[cfg(not(feature = "serde"))]
pub(crate) fn push_state_and_url(
    history: &History,
    value: &[f64; 2],
    url: String,
) -> Result<(), JsValue> {
    let position = js_sys::Array::new();
    position.push(&JsValue::from(value[0]));
    position.push(&JsValue::from(value[1]));

    history.push_state_with_url(&position, "", Some(&url))
}

#[cfg(feature = "serde")]
pub(crate) fn push_state_and_url<V: serde::Serialize>(
    history: &History,
    value: &V,
    url: String,
) -> Result<(), JsValue> {
    let position = JsValue::from_serde(value).unwrap();

    history.push_state_with_url(&position, "", Some(&url))
}

#[cfg(feature = "serde")]
pub(crate) fn get_current<V: serde::de::DeserializeOwned>(history: &History) -> Option<V> {
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

#[cfg(not(feature = "serde"))]
pub(crate) fn get_current(history: &History) -> Option<[f64; 2]> {
    use wasm_bindgen::JsCast;

    let state = history.state();
    if let Err(err) = &state {
        error!(err);
    }
    state.ok().and_then(|state| {
        let state = state.dyn_into::<js_sys::Array>().ok()?;
        let x = state.get(0).as_f64()?;
        let y = state.get(1).as_f64()?;
        Some([x, y])
    })
}
