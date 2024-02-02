use std::time::Duration;

use dioxus::prelude::*;

fn main() {
    launch(app);
}

fn app() -> Element {
    let mut count = use_signal(|| 0);

    let g: ReadOnlySignal<(), UnsyncStorage> =
        use_maybe_sync_selector_with_dependencies((&(count())), move |deps| {
            println!("High-Five counter: {}", deps);
        });

    rsx! {
        h1 { "High-Five counter: {count}" }
        button { onclick: move |_| count += 1, "Up high!" }
        button { onclick: move |_| count -= 1, "Down low!" }
    }
}
