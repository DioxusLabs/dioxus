//! This example demonstrates how to create an overlay window with dioxus.
//!
//! Basically, we just create a new window with a transparent background and no decorations, size it to the screen, and
//! then we can draw whatever we want on it. In this case, we're drawing a simple overlay with a draggable header.
//!
//! We also add a global shortcut to toggle the overlay on and off, so you could build a raycast-type app with this.

use dioxus::desktop::{
    tao::dpi::PhysicalPosition, use_global_shortcut, LogicalSize, WindowBuilder,
};
use dioxus::prelude::*;

fn main() {
    LaunchBuilder::desktop().with_cfg(make_config()).launch(app);
}

fn app() -> Element {
    let mut show_overlay = use_signal(|| true);

    _ = use_global_shortcut("cmd+g", move || show_overlay.toggle());

    rsx! {
        if show_overlay() {
            div {
                width: "100%",
                height: "100%",
                background_color: "red",
                border: "1px solid black",

                div {
                    width: "100%",
                    height: "10px",
                    background_color: "black",
                    onmousedown: move |_| dioxus::desktop::window().drag(),
                }

                "This is an overlay!"
            }
        }
    }
}

fn make_config() -> dioxus::desktop::Config {
    dioxus::desktop::Config::default()
        .with_window(make_window())
        .with_custom_head(
            r#"
<style type="text/css">
    html, body {
        height: 100px;
        margin: 0;
        overscroll-behavior-y: none;
        overscroll-behavior-x: none;
        overflow: hidden;
    }
    #main, #bodywrap {
        height: 100%;
        margin: 0;
        overscroll-behavior-x: none;
        overscroll-behavior-y: none;
    }
</style>
"#
            .to_owned(),
        )
}

fn make_window() -> WindowBuilder {
    WindowBuilder::new()
        .with_transparent(true)
        .with_decorations(false)
        .with_resizable(false)
        .with_always_on_top(true)
        .with_position(PhysicalPosition::new(0, 0))
        .with_max_inner_size(LogicalSize::new(100000, 50))
}
