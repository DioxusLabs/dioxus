use dioxus::prelude::*;

fn main() {
    dioxus::desktop::launch_with_props(with_hot_reload, app, |c| c);
}

fn app(cx: Scope) -> Element {
    let rsx_code = use_state(&cx, || {
        r##"div {
            width: "100%",
            height: "500px",
            onclick: move |_| {
                count.modify(|count| *count + 10);
            },

            p {
                "High-Five counter: {count}",
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
        }"##
        .to_string()
    });
    let submitted_rsx_code = use_state(&cx, || None);

    cx.render(rsx! {
        div {
             display: "flex",
             flex_direction: "row",
             width: "100%",
             height: "50%",
             Editable{
                current_code: submitted_rsx_code.get().clone(),
             },

             textarea {
                 width: "90%",
                 value:
                    rsx_code
                 ,
                 oninput: move |evt| {
                     rsx_code.set(evt.value.clone());
                 },
             }

             button {
                 height: "100%",
                 width: "10%",
                 onclick: move |_|{
                         submitted_rsx_code.set(Some(rsx_code.get().clone()));
                 },
                 "submit"
             }
         }
    })
}

#[derive(PartialEq, Props)]
struct EditableProps {
    #[props(!optional)]
    current_code: Option<String>,
}

fn Editable(cx: Scope<EditableProps>) -> Element {
    let count = use_state(&cx, || 170);
    if let Some(code) = cx.props.current_code.as_ref() {
        let rsx_index: RsxTextIndex = cx.consume_context().unwrap();
        rsx_index.insert(
            CodeLocation {
                file: r"examples\hot_reload.rs".to_string(),
                line: 95,
                column: 15,
            },
            code.clone(),
        );
    }
    cx.render(rsx! {
         div {
            width: "100%",
            height: "500px",
            onclick: move |_| {
                count.modify(|count| *count + 10);
            },

            p {
                "High-Five counter: {count}",
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
