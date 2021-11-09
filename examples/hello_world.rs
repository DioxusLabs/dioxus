use dioxus::prelude::*;

fn main() {
    dioxus::desktop::launch(App, |c| c);
}

fn App((cx, props): Scope<()>) -> Element {
    cx.render(rsx! (
        div { "Hello, world!" }
    ))
}
