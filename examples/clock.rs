#![allow(non_snake_case)]

use dioxus::prelude::*;
use dioxus_signals::{use_signal, Effect, Signal};

fn main() {
    dioxus_desktop::launch(app);
}

fn app(cx: Scope) -> Element {
    let counts = use_signal(cx, || (0..100).map(Signal::new).collect::<Vec<_>>());

    cx.use_hook(|| {
        Effect::new(move || {
            println!("Counts: {:?}", counts);
        })
    });

    render! {
        for count in counts {
            Child {
                count: count,
            }
        }
    }
}

#[derive(Props, PartialEq)]
struct ChildProps {
    count: Signal<u64>,
}

fn Child(cx: Scope<ChildProps>) -> Element {
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
