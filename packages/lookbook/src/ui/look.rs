use crate::ui::pane::VerticalPane;
use dioxus::prelude::*;
use dioxus_markdown::Markdown;

#[component]
pub fn Look(
    name: &'static str,
    docs: &'static str,
    controls: Element,
    children: Element,
) -> Element {
    let top = rsx!(
        div { padding: "20px",
            h2 { "{name}" }
            Markdown { content: Signal::new(String::from(docs)) }
        }
        div {
            flex: 1,
            display: "flex",
            flex_direction: "column",
            display: "flex",
            justify_content: "center",
            align_items: "center",
            { children }
        }
    );

    let bottom = rsx!(
        div { flex: 1, display: "flex", flex_direction: "column", overflow_y: "auto", gap: "20px",
            table { text_align: "left", border_collapse: "collapse",
                tr { height: "60px", color: "#777", border_bottom: "2px solid #e7e7e7",
                    th { padding_left: "20px", "Name" }
                    th { "Type" }
                    th { "Description" }
                    th { "Default" }
                    th { padding_right: "20px", "Controls" }
                }
                { controls }
            }
        }
    );

    rsx!(
        div { flex: 1, display: "flex", flex_direction: "column", VerticalPane { top: top, bottom: bottom } }
    )
}
