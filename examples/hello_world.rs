use dioxus::prelude::*;

fn main() {
    dioxus_desktop::launch(app);
}

fn app(cx: Scope) -> Element {
    let a = 10;

    cx.render(rsx! (
        div { "Hello, world!" }
        format_args!("asdasdasd {a}")
    ))
}
