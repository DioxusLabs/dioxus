//! Example: Global signals and memos
//!
//! This example demonstrates how to use global signals and memos to share state across your app.
//! Global signals are simply signals that live on the root of your app and are accessible from anywhere. To access a
//! global signal, simply use its methods like a regular signal. Calls to `read` and `write` will be forwarded to the
//! signal at the root of your app using the `static`'s address.

use dioxus::prelude::*;

const STYLE: Asset = asset!("/examples/assets/counter.css");

static COUNT: GlobalSignal<i32> = Signal::global(|| 0);
static DOUBLED_COUNT: GlobalMemo<i32> = Memo::global(|| COUNT() * 2);

fn main() {
    launch(app);
}

fn app() -> Element {
    rsx! {
        document::Stylesheet { href: STYLE }
        Increment {}
        Decrement {}
        Reset {}
        Display {}
    }
}

#[component]
fn Increment() -> Element {
    rsx! {
        button { onclick: move |_| *COUNT.write() += 1, "Up high!" }
    }
}

#[component]
fn Decrement() -> Element {
    rsx! {
        button { onclick: move |_| *COUNT.write() -= 1, "Down low!" }
    }
}

#[component]
fn Display() -> Element {
    rsx! {
        p { "Count: {COUNT}" }
        p { "Doubled: {DOUBLED_COUNT}" }
    }
}

#[component]
fn Reset() -> Element {
    // Not all write methods are available on global signals since `write` requires a mutable reference. In these cases,
    // We can simply pull out the actual signal using the signal() method.
    let mut as_signal = use_hook(|| COUNT.resolve());

    rsx! {
        button { onclick: move |_| as_signal.set(0), "Reset" }
    }
}
