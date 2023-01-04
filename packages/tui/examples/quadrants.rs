use dioxus::prelude::*;

fn main() {
    dioxus_tui::launch(app);
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
                div {
                    border_width: "1px",
                    width: "50%",
                    height: "100%",
                    background_color: "red",
                    justify_content: "center",
                    align_items: "center",
                    "[A]"
                }
                div {
                    width: "50%",
                    height: "100%",
                    background_color: "black",
                    justify_content: "center",
                    align_items: "center",
                    "[B]"
                }
            }

            div {
                width: "100%",
                height: "50%",
                flex_direction: "row",
                div {
                    width: "50%",
                    height: "100%",
                    background_color: "green",
                    justify_content: "center",
                    align_items: "center",
                    "[C]"
                }
                div {
                    width: "50%",
                    height: "100%",
                    background_color: "blue",
                    justify_content: "center",
                    align_items: "center",
                    "[D]"
                }
            }
        }
    })
}
