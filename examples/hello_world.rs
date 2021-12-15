use dioxus::prelude::*;

fn main() {
    dioxus::desktop::launch(App);
}

fn App((cx, props): ScopeState<()>) -> Element {
    cx.render(rsx! (
        div { "Hello, world!" }
    ))
}
