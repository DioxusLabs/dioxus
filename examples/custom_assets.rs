use dioxus::prelude::*;

fn main() {
    dioxus::desktop::launch(app);
}

fn app(cx: Scope) -> Element {
    let (disabled, set_disabled) = use_state(&cx, || false);

    cx.render(rsx! {
        div {
            "hi"
            img {
                src: "examples/../../../assets/logo.png",
            }
        }
    })
}
