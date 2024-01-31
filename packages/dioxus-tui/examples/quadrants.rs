#![allow(non_snake_case)]

use dioxus::prelude::*;

fn main() {
    dioxus_tui::launch_cfg(app, Default::default());
}

#[component]
fn Quadrant(color: String, text: String) -> Element {
    rsx! {
        div {
            border_width: "1px",
            width: "50%",
            height: "100%",
            justify_content: "center",
            align_items: "center",
            background_color: "{color}",
            "{text}"
        }
    }
}

fn app() -> Element {
    rsx! {
        div {
            width: "100%",
            height: "100%",
            flex_direction: "column",
            div {
                width: "100%",
                height: "50%",
                flex_direction: "row",
                Quadrant {
                    color: "red".to_string(),
                    text: "[A]".to_string()
                },
                Quadrant {
                    color: "black".to_string(),
                    text: "[B]".to_string()
                }
            }
            div {
                width: "100%",
                height: "50%",
                flex_direction: "row",
                Quadrant {
                    color: "green".to_string(),
                    text: "[C]".to_string()
                },
                Quadrant {
                    color: "blue".to_string(),
                    text: "[D]".to_string()
                }
            }
        }
    }
}
