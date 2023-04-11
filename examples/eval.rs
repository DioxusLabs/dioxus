use dioxus::prelude::*;

fn main() {
    dioxus_desktop::launch(app);
}

fn app(cx: Scope) -> Element {
    let eval = dioxus_desktop::use_eval(cx);
    let script = use_state(cx, String::new);
    let output = use_state(cx, String::new);

    cx.render(rsx! {
        div {
            p { "Output: {output}" }
            input {
                placeholder: "Enter an expression",
                value: "{script}",
                oninput: move |e| script.set(e.value.clone()),
            }
            button {
                onclick: move |_| {
                    to_owned![script, eval, output];
                    cx.spawn(async move {
                        if let Ok(res) = eval(script.to_string()).await {
                            output.set(res.to_string());
                        }
                    });
                },
                "Execute"
            }
        }
    })
}
