#![allow(non_snake_case)]

use dioxus::prelude::*;
use dioxus_signals::Signal;

fn main() {
    launch(app);
}

#[derive(Clone, Copy, Default)]
struct ApplicationData {
    first_data: Signal<i32>,
    second_data: Signal<i32>,
    many_signals: Signal<Vec<Signal<i32>>>,
}

fn app() -> Element {
    use_context_provider(ApplicationData::default);

    rsx! {
        div { ReadsFirst {} }
        div { ReadsSecond {} }
        div { ReadsManySignals {} }
    }
}

#[component]
fn ReadsFirst() -> Element {
    println!("running first");
    let mut data = use_context::<ApplicationData>();

    rsx! {
        button {
            onclick: move |_| {
                *data.first_data.write() += 1;
            },
            "Increase"
        }
        button {
            onclick: move |_| {
                *data.first_data.write() -= 1;
            },
            "Decrease"
        }
        button {
            onclick: move |_| {
                *data.first_data.write() = 0;
            },
            "Reset"
        }
        "{data.first_data}"
    }
}

#[component]
fn ReadsSecond() -> Element {
    println!("running second");
    let mut data = use_context::<ApplicationData>();

    rsx! {
        button { onclick: move |_| data.second_data += 1, "Increase" }
        button { onclick: move |_| data.second_data -= 1, "Decrease" }
        button { onclick: move |_| data.second_data.set(0), "Reset" }
        "{data.second_data}"
    }
}

#[component]
fn ReadsManySignals() -> Element {
    println!("running many signals");
    let mut data = use_context::<ApplicationData>();

    rsx! {
        button {
            onclick: move |_| data.many_signals.write().push(Signal::new(0)),
            "Create"
        }
        button {
            onclick: move |_| { data.many_signals.write().pop(); },
            "Destroy"
        }
        button {
            onclick: move |_| {
                if let Some(mut first) = data.many_signals.read().first().cloned() {
                    first += 1;
                }
            },
            "Increase First Item"
        }
        for signal in data.many_signals.iter() {
            Child { count: *signal }
        }
    }
}

#[component]
fn Child(mut count: Signal<i32>) -> Element {
    println!("running child");

    rsx! {
        div {
            "Child: {count}"
            button { onclick: move |_| count += 1, "Increase" }
            button { onclick: move |_| count -= 1, "Decrease" }
        }
    }
}
