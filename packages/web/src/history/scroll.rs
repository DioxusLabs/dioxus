use wasm_bindgen::{prelude::Closure, JsCast};
use web_sys::Window;

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct ScrollPosition {
    pub x: f64,
    pub y: f64,
}

impl ScrollPosition {
    pub(crate) fn of_window(window: &Window) -> Self {
        Self {
            x: window.scroll_x().unwrap_or_default(),
            y: window.scroll_y().unwrap_or_default(),
        }
    }

    pub(crate) fn scroll_to(&self, window: Window) {
        let Self { x, y } = *self;
        let f = Closure::wrap(
            Box::new(move || window.scroll_to_with_x_and_y(x, y)) as Box<dyn FnMut()>
        );
        web_sys::window()
            .expect("should be run in a context with a `Window` object (dioxus cannot be run from a web worker)")
            .request_animation_frame(&f.into_js_value().unchecked_into())
            .expect("should register `requestAnimationFrame` OK");
    }
}
