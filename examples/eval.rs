use dioxus::prelude::*;

fn main() {
    dioxus::desktop::launch(app);
}

fn app(cx: Scope) -> Element {
    let script = use_state(&cx, String::new);
    let eval = use_eval(&cx);

    cx.render(rsx! {
        div {
            input {
                placeholder: "Enter an expression",
                value: "{script}",
                oninput: move |e| script.set(e.value.clone()),
            }
            button {
                onclick: move |_| eval(script),

                "Execute"
            }
        }
    })
}
