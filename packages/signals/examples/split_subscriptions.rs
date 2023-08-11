#![allow(non_snake_case)]

use dioxus::prelude::*;
use dioxus_signals::Signal;

fn main() {
    dioxus_desktop::launch(app);
}

#[derive(Clone, Copy, Default)]
struct ApplicationData {
    first_data: Signal<i32>,
    second_data: Signal<i32>,
    many_signals: Signal<Vec<Signal<i32>>>,
}

fn use_app_data(cx: Scope) -> ApplicationData {
    *use_context(cx).unwrap()
}

fn app(cx: Scope) -> Element {
    use_context_provider(cx, ApplicationData::default);

    render! {
        div {
            ReadsFirst {}
        }
        div {
            ReadsSecond {}
        }
        div {
            ReadsManySignals {}
        }
    }
}

fn ReadsFirst(cx: Scope) -> Element {
    println!("running first");
    let data = use_app_data(cx);

    render! {
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

fn ReadsSecond(cx: Scope) -> Element {
    println!("running second");
    let data = use_app_data(cx);

    render! {
        button {
            onclick: move |_| {
                *data.second_data.write() += 1;
            },
            "Increase"
        }
        button {
            onclick: move |_| {
                *data.second_data.write() -= 1;
            },
            "Decrease"
        }
        button {
            onclick: move |_| {
                *data.second_data.write() = 0;
            },
            "Reset"
        }
        "{data.second_data}"
    }
}

fn ReadsManySignals(cx: Scope) -> Element {
    println!("running many signals");
    let data = use_app_data(cx);

    render! {
        button {
            onclick: move |_| {
                data.many_signals.write().push(Signal::new(0));
            },
            "Create"
        }
        button {
            onclick: move |_| {
                data.many_signals.write().pop();
            },
            "Destroy"
        }
        button {
            onclick: move |_| {
                if let Some(first) = data.many_signals.read().get(0) {
                    *first.write() += 1;
                }
            },
            "Increase First Item"
        }
        for signal in data.many_signals {
            Child {
                count: signal,
            }
        }
    }
}

#[derive(Props, PartialEq)]
struct ChildProps {
    count: Signal<i32>,
}

fn Child(cx: Scope<ChildProps>) -> Element {
    println!("running child");
    let count = cx.props.count;

    render! {
        div {
            "Child: {count}"
            button {
                onclick: move |_| {
                    *count.write() += 1;
                },
                "Increase"
            }
            button {
                onclick: move |_| {
                    *count.write() -= 1;
                },
                "Decrease"
            }
        }
    }
}
