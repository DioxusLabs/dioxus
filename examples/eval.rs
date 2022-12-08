use dioxus::prelude::*;

fn main() {
    dioxus_desktop::launch(app);
}

fn app(cx: Scope) -> Element {
    let script = use_state(cx, String::new);
    let eval = dioxus_desktop::use_eval(cx);

    cx.render(rsx! {
        div {
            input {
                placeholder: "Enter an expression",
                value: "{script}",
                oninput: move |e| script.set(e.value.clone()),
            }
            button {
                onclick: move |_| eval(script.to_string()),
                "Execute"
            }
        }
    })
}
