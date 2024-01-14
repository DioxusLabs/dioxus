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
            // justify_content: "center",
            // align_items: "center",
            // flex_direction: "row",
            // background_color: "red",

            p {
                background_color: "black",
                flex_direction: "column",
                justify_content: "center",
                align_items: "center",
                // height: "10%",
                "hi"
                "hi"
                "hi"
            }

            li {
                background_color: "red",
                flex_direction: "column",
                justify_content: "center",
                align_items: "center",
                // height: "10%",
                "bib"
                "bib"
                "bib"
                "bib"
                "bib"
                "bib"
                "bib"
                "bib"
            }
            li {
                background_color: "blue",
                flex_direction: "column",
                justify_content: "center",
                align_items: "center",
                // height: "10%",
                "zib"
                "zib"
                "zib"
                "zib"
                "zib"
                "zib"
                "zib"
                "zib"
                "zib"
                "zib"
                "zib"
                "zib"
                "zib"
            }
            p {
                background_color: "yellow",
                "asd"
            }
            p {
                background_color: "green",
                "asd"
            }
            p {
                background_color: "white",
                "asd"
            }
            p {
                background_color: "cyan",
                "asd"
            }
        }
    }
}
