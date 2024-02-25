//! Backgrounded futures example
//!
//! This showcases how use_future, use_memo, and use_effect will stop running if the component returns early.
//! Generally you should avoid using early returns around hooks since most hooks are not properly designed to
//! handle early returns. However, use_future *does* pause the future when the component returns early, and so
//! hooks that build on top of it like use_memo and use_effect will also pause.
//!
//! This example is more of a demonstration of the behavior than a practical use case, but it's still interesting to see.

use dioxus::prelude::*;

fn main() {
    launch_desktop(app);
}

fn app() -> Element {
    let mut show_child = use_signal(|| true);
    let mut count = use_signal(|| 0);

    let child = use_memo(move || {
        rsx! {
            Child { count }
        }
    });

    rsx! {
        // Some toggle/controls to show the child or increment the count
        button { onclick: move |_| show_child.toggle(), "Toggle child" }
        button { onclick: move |_| count += 1, "Increment count" }

        if show_child() {
            {child()}
        }
    }
}

#[component]
fn Child(count: Signal<i32>) -> Element {
    let mut early_return = use_signal(|| false);

    let early = rsx! {
        button { onclick: move |_| early_return.toggle(), "Toggle {early_return} early return" }
    };

    if early_return() {
        return early;
    }

    use_future(move || async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            println!("Child")
        }
    });

    use_effect(move || println!("Child count: {}", count()));

    rsx! {
        div {
            "Child component"
            {early}
        }
    }
}
