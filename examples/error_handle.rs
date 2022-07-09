use dioxus::prelude::*;

fn main() {
    dioxus_desktop::launch(app);
}

fn app(cx: Scope) -> Element {
    let val = use_state(&cx, || "0.0001");

    let num = match val.parse::<f32>() {
        Err(_) => return cx.render(rsx!("Parsing failed")),
        Ok(num) => num,
    };

    cx.render(rsx! {
        h1 { "The parsed value is {num}" }
        button {
            onclick: move |_| val.set("invalid"),
            "Set an invalid number"
        }
    })
}
