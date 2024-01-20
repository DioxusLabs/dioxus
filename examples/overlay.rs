use dioxus::desktop::{tao::dpi::PhysicalPosition, LogicalSize, WindowBuilder};
use dioxus::prelude::*;

fn main() {
    LaunchBuilder::desktop().with_cfg(make_config());
}

fn app() -> Element {
    rsx! {
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
