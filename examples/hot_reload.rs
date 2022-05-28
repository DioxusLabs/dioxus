use dioxus::prelude::*;

fn main() {
    dioxus::web::launch_with_props(with_hot_reload, app, |c| c);
}

fn app(cx: Scope) -> Element {
    let count = use_state(&cx, || 170);
    let rsx_code = use_state(&cx, || None);
    let re_render = cx.schedule_update();

    cx.render(rsx! {
        div {
            width: "100%",
            height: "500px",
            onclick: move |_| {
                count.modify(|count| *count + 10);
            },

            div {
                display: "flex",
                flex_direction: "row",
                width: "100%",
                height: "50%",
                textarea {
                    width: "90%",
                    value: {
                        if rsx_code.get().is_none() {
                            let rsx_text_index: RsxTextIndex = cx.consume_context().unwrap();
                            let read = rsx_text_index.read();
                            rsx_code.set(Some(read.get(&__line_num).unwrap().clone()));
                        }
                        (*rsx_code.current()).clone().unwrap()
                    },
                    oninput: move |evt| {
                        rsx_code.set(Some(evt.value.clone()));
                    },
                }

                button {
                    height: "100%",
                    width: "10%",
                    onclick: move |_|{
                        if let Some(code) = rsx_code.get(){
                            let rsx_text_index: RsxTextIndex = cx.consume_context().unwrap();
                            rsx_text_index.insert(__line_num.clone(), code.clone());
                            re_render();
                        }
                    },
                    "submit"
                }
            }

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
