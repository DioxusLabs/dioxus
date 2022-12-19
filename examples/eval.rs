use dioxus::prelude::*;
use dioxus_desktop::EvalResult;

fn main() {
    dioxus_desktop::launch(app);
}

fn app(cx: Scope) -> Element {
    let script = use_state(cx, String::new);
    let eval = dioxus_desktop::use_eval(cx);
    let future: &UseRef<Option<EvalResult>> = use_ref(cx, || None);
    if future.read().is_some() {
        let future_clone = future.clone();
        cx.spawn(async move {
            if let Some(fut) = future_clone.with_mut(|o| o.take()) {
                println!("{:?}", fut.await)
            }
        });
    }

    cx.render(rsx! {
        div {
            input {
                placeholder: "Enter an expression",
                value: "{script}",
                oninput: move |e| script.set(e.value.clone()),
            }
            button {
                onclick: move |_| {
                    let fut = eval(script);
                    future.set(Some(fut));
                },
                "Execute"
            }
        }
    })
}
