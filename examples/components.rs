use dioxus::prelude::*;

fn main() {
    rink::launch(app);
}

#[derive(Props, PartialEq)]
struct QuadrentProps {
    color: String,
    text: String,
}

fn Quadrent(cx: Scope<QuadrentProps>) -> Element {
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
                Quadrent{
                    color: "red".to_string(),
                    text: "[A]".to_string()
                },
                Quadrent{
                    color: "black".to_string(),
                    text: "[B]".to_string()
                }
            }

            div {
                width: "100%",
                height: "50%",
                flex_direction: "row",
                Quadrent{
                    color: "green".to_string(),
                    text: "[C]".to_string()
                },
                Quadrent{
                    color: "blue".to_string(),
                    text: "[D]".to_string()
                }
            }
        }
    })
}
