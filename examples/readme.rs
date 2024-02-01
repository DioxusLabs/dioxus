use std::time::Duration;

use dioxus::prelude::*;

fn main() {
    launch(app);
}

fn app() -> Element {
    let mut count = use_signal(|| 0);

    use_effect(move || {
        println!("The count is now: {}", count);
    });

    rsx! {
        h1 { "High-Five counter: {count}" }
        Child { sig: count }
        button { onclick: move |_| count += 1, "Up high!" }
        button { onclick: move |_| count -= 1, "Down low!" }
    }
}

#[component]
fn Child(sig: Signal<i32>) -> Element {
    let doubled = use_memo(move || sig() * 2);

    let tripled = use_async_memo(move || async move {
        tokio::time::sleep(Duration::from_millis(200)).await;
        sig() * 3
    });

    let trippled = use_memo(move || match tripled.value() {
        Some(v) => v.cloned(),
        None => 1338,
    });

    rsx! {
        "The count is: {sig}, doubled: {doubled}, tripled: {trippled}"
    }
}
