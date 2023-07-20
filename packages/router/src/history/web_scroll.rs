use gloo::render::{request_animation_frame, AnimationFrame};
use web_sys::Window;

#[derive(Clone, Copy, Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
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

    pub(crate) fn scroll_to(&self, window: Window) -> AnimationFrame {
        let Self { x, y } = *self;
        request_animation_frame(move |_| window.scroll_to_with_x_and_y(x, y))
    }
}
