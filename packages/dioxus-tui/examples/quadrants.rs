#![allow(non_snake_case)]

use dioxus::prelude::*;
use dioxus_tui::Config;

fn main() {
    dioxus_tui::launch_cfg(app, Config::default());
}

#[derive(Props, PartialEq)]
struct QuadrentProps {
    color: String,
    text: String,
}

fn Quadrant(cx: Scope<QuadrentProps>) -> Element {
    cx.render(rsx! {
        div {
            border_width: "1px",
            width: "50%",
            height: "100%",
            background_color: "{cx.props.color}",
            justify_content: "center",
            align_items: "center",

            "{cx.props.text}"
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
                Quadrant{
                    color: "red".to_string(),
                    text: "[A]".to_string()
                },
                Quadrant{
                    color: "black".to_string(),
                    text: "[B]".to_string()
                }
            }

            div {
                width: "100%",
                height: "50%",
                flex_direction: "row",
                Quadrant{
                    color: "green".to_string(),
                    text: "[C]".to_string()
                },
                Quadrant{
                    color: "blue".to_string(),
                    text: "[D]".to_string()
                }
            }
        }
    })
}
