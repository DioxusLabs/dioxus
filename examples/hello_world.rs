use dioxus::prelude::*;

fn main() {
    dioxus::desktop::launch(App, |c| c);
}

fn App((cx, props): Component<()>) -> DomTree {
    cx.render(rsx! (
        div { "Hello, world!" }
    ))
}
