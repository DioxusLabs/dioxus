//! Passing a read-only signal to a child component.
//!
//! `ReadSignal<T>` is a signal that can only be read, not written. Taking it as a prop is
//! the idiomatic way to tell a child "you can subscribe to this value, but you can't change
//! it." Any `Signal<T>` or `Memo<T>` coerces into a `ReadSignal<T>` automatically, so the
//! parent passes them in unchanged.

use dioxus::prelude::*;

fn main() {
    dioxus::launch(app);
}

fn app() -> Element {
    let mut count = use_signal(|| 0);
    let doubled = use_memo(move || count() * 2);

    rsx! {
        h1 { "Parent" }
        button { onclick: move |_| count += 1, "Increment" }

        // Both Signal and Memo implement Into<ReadSignal<T>>
        Display { label: "Count", value: count }
        Display { label: "Doubled", value: doubled }

        // You can also pass a plain value — it'll be wrapped automatically
        Display { label: "Constant", value: 42 }
    }
}

// Taking `ReadSignal<i32>` means the child can subscribe but can't mutate the source
#[component]
fn Display(label: String, value: ReadSignal<i32>) -> Element {
    rsx! {
        p { "{label}: {value}" }
    }
}
