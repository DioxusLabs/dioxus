use dioxus::prelude::*;

fn main() {
    dioxus_tui::launch(app);
}

fn app() -> Element {
    let mut alpha = use_signal(|| 100);

    rsx! {
        div {
            width: "100%",
            height: "100%",
            flex_direction: "column",
            onwheel: move |evt| alpha.set((alpha() + evt.delta().strip_units().y as i64).clamp(0, 100)),

            p {
                background_color: "black",
                flex_direction: "column",
                justify_content: "center",
                align_items: "center",
                color: "green",
                "hi"
                "hi"
                "hi"
            }

            li {
                background_color: "red",
                flex_direction: "column",
                justify_content: "center",
                align_items: "center",
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
                p {
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
                color: "rgba(255, 255, 255)",
                "underline"
            }
            p {
                text_decoration: "line-through",
                color: "hsla(10, 100%, 70%)",
                "line-through"
            }
            div{
                position: "absolute",
                top: "1px",
                background_color: "rgba(255, 0, 0, 50%)",
                width: "100%",
                p {
                    color: "rgba(255, 255, 255, {alpha}%)",
                    background_color: "rgba(100, 100, 100, {alpha}%)",
                    "rgba(255, 255, 255, {alpha}%)"
                }
                p {
                    color: "rgba(255, 255, 255, 100%)",
                    "rgba(255, 255, 255, 100%)"
                }
            }
        }
    }
}
