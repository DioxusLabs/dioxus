use dioxus::prelude::*;

fn main() {
    dioxus_tui::launch(app);
}

fn app() -> Element {
    rsx! {
        div {
            width: "100%",
            height: "100%",
            flex_direction: "column",
            background_color: "black",
            margin_right: "10px",

            div {
                width: "70%",
                height: "70%",
                background_color: "green",
                margin_left: "4px",

                div {
                    width: "100%",
                    height: "100%",

                    margin_top: "2px",
                    margin_bottom: "2px",
                    margin_left: "2px",
                    margin_right: "2px",
                    flex_shrink: "0",

                    background_color: "red",
                    justify_content: "center",
                    align_items: "center",
                    flex_direction: "column",

                    padding_top: "2px",
                    padding_bottom: "2px",
                    padding_left: "4px",
                    padding_right: "4px",

                    "[A]"
                    "[A]"
                    "[A]"
                    "[A]"
                }
            }
        }
    }
}
