#![allow(non_snake_case)]

use dioxus::prelude::*;

fn main() {
    dioxus::tui::launch(app);
}

fn Button(cx: Scope) -> Element {
    let state = use_state(&cx, || false);
    let color = if *state.get() { "red" } else { "blue" };
    let text = if *state.get() { "☐" } else { "☒" };

    cx.render(rsx! {
        div {
            border_width: "1px",
            width: "50%",
            height: "100%",
            background_color: "{color}",
            justify_content: "center",
            align_items: "center",
            onkeydown: |_| state.modify(|s| !s),

            "{text}"
        }
    })
}

fn app(cx: Scope) -> Element {
    cx.render(rsx! {
        div {
            width: "100%",
            height: "100%",
            flex_direction: "column",

            div {
                width: "100%",
                height: "50%",
                flex_direction: "row",

                Button{},
                Button{},
                Button{},
                Button{},
            }

            div {
                width: "100%",
                height: "50%",
                flex_direction: "row",

                Button{},
                Button{},
                Button{},
                Button{},
            }
        }
    })
}
