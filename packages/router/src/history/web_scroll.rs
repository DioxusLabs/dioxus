use gloo::render::{request_animation_frame, AnimationFrame};
use log::error;
use serde::{Deserialize, Serialize};
use wasm_bindgen::JsValue;
use web_sys::{History, Window};

#[derive(Debug, Default, Deserialize, Serialize)]
pub(crate) struct ScrollPosition {
    x: f64,
    y: f64,
}

pub(crate) fn top_left() -> JsValue {
    serde_wasm_bindgen::to_value(&ScrollPosition::default()).unwrap()
}

pub(crate) fn update_history(window: &Window, history: &History) {
    let position = serde_wasm_bindgen::to_value(&ScrollPosition {
        x: window.scroll_x().unwrap_or_default(),
        y: window.scroll_y().unwrap_or_default(),
    })
    .unwrap();

    if let Err(e) = history.replace_state(&position, "") {
        error!("failed to update scroll position: {e:?}");
    }
}

pub(crate) fn update_scroll(window: &Window, history: &History) -> AnimationFrame {
    let ScrollPosition { x, y } = history
        .state()
        .map(|state| serde_wasm_bindgen::from_value(state).unwrap_or_default())
        .unwrap_or_default();

    let w = window.clone();
    request_animation_frame(move |_| w.scroll_to_with_x_and_y(x, y))
}
