use dioxus::prelude::*;

fn main() {
    dioxus::desktop::launch(app);
}

fn app(cx: Scope) -> Element {
    let count = use_state(&cx, || 170);
    cx.render(rsx! {
         div {
            width: "100%",
            height: "500px",
            onclick: move |_| {
                count.modify(|count| *count + 10);
            },
            p {
                "High-Five counter: {count.to_string():?}",
            }

            div {
                width: "{count}px",
                height: "10px",
                background_color: "red",
            }

            Comp {
                color: "#083289"
            }

            Comp {
                color: "green"
            }

            {
                (0..10).map(|i| {
                    cx.render(rsx!{p {"{i}"}})
                })
            }
        }
    })
}

#[derive(PartialEq, Props)]
struct CompProps {
    color: &'static str,
}

fn Comp(cx: Scope<CompProps>) -> Element {
    cx.render(rsx! {
        h1 {
            color: "{cx.props.color}",
            "Hello, from a component!"
        }
    })
}
