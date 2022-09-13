use dioxus::prelude::*;

fn main() {
    dioxus_desktop::launch(app);
}

fn app(cx: Scope) -> Element {
    cx.render(rsx! (
        div { "Hello, world!" }
        (0..10).map(|f| cx.render(rsx! { "{f}" }))
    ))
}
