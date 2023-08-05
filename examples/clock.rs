#![allow(non_snake_case)]

use dioxus::prelude::*;
use dioxus_signals::{use_signal, Effect, Signal};

fn main() {
    dioxus_desktop::launch(app);
}

fn app(cx: Scope) -> Element {
    println!("running app");

    let counts = use_signal(cx, || (0..100).map(Signal::new).collect::<Vec<_>>());

    cx.use_hook(|| {
        Effect::new(move || {
            println!("Counts: {:?}", counts);
        })
    });

    render! {
        for (i, count) in counts.into_iter().enumerate() {
            Child {
                id: i,
                count: count,
            }
        }
    }
}

#[derive(Props, PartialEq)]
struct ChildProps {
    id: usize,
    count: Signal<u64>,
}

fn Child(cx: Scope<ChildProps>) -> Element {
    println!("running child {}", cx.props.id);
    let count = cx.props.count;

    use_future!(cx, || async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(count.value())).await;
            *count.write() += 1;
        }
    });

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
