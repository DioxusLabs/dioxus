use dioxus::prelude::*;

fn main() {
    rink::launch(app);
}

fn app(cx: Scope) -> Element {
    cx.render(rsx! {
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
            div {
                font_weight: "bold",
                color: "#666666",
                p{
                    "bold"
                }
                p {
                    font_weight: "normal",
                    " normal"
                }
            }
            p {
                font_style: "italic",
                color: "red",
                "italic"
            }
            p {
                text_decoration: "underline",
                color: "rgb(50, 100, 255)",
                "underline"
            }
            p {
                text_decoration: "line-through",
                color: "hsl(10, 100%, 70%)",
                "line-through"
            }
        }
    })
}
