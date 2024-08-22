//! This example shows how to set the title of the page or window with the Title component

use dioxus::prelude::*;

fn main() {
    launch(app);
}

fn app() -> Element {
    let mut count = use_signal(|| 0);

    rsx! {
        div {
            // You can set the title of the page with the Title component
            // In web applications, this sets the title in the head. On desktop, it sets the window title
            document::Title { "My Application (Count {count})" }
            button { onclick: move |_| count += 1, "Up high!" }
            button { onclick: move |_| count -= 1, "Down low!" }
        }
    }
}
