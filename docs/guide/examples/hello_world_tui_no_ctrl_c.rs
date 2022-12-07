// todo remove deprecated
#![allow(non_snake_case, deprecated)]

use dioxus::events::{KeyCode, KeyboardEvent};
use dioxus::prelude::*;
use dioxus_tui::TuiContext;

fn main() {
    dioxus_tui::launch_cfg(
        App,
        dioxus_tui::Config::new()
            .without_ctrl_c_quit()
            // Some older terminals only support 16 colors or ANSI colors
            // If your terminal is one of these, change this to BaseColors or ANSI
            .with_rendering_mode(dioxus_tui::RenderingMode::Rgb),
    );
}

fn App(cx: Scope) -> Element {
    let tui_ctx: TuiContext = cx.consume_context().unwrap();

    cx.render(rsx! {
        div {
            width: "100%",
            height: "10px",
            background_color: "red",
            justify_content: "center",
            align_items: "center",
            onkeydown: move |k: KeyboardEvent| if let KeyCode::Q = k.key_code {
                tui_ctx.quit();
            },

            "Hello world!"
        }
    })
}
