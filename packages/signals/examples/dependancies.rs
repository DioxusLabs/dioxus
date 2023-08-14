#![allow(non_snake_case)]

use dioxus::prelude::*;
use dioxus_signals::*;

fn main() {
    dioxus_desktop::launch(app);
}

fn app(cx: Scope) -> Element {
    let signal = use_signal(cx, || 0);

    use_future!(cx, || async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            *signal.write() += 1;
        }
    });

    let local_state = use_state(cx, || 0);
    let computed =
        use_selector_with_dependencies(cx, (local_state.get(),), move |(local_state,)| {
            local_state * 2 + signal.value()
        });
    println!("Running app");

    render! {
        button {
            onclick: move |_| {
                local_state.set(local_state.get() + 1);
            },
            "Add one"
        }
        div {
            "{computed}"
        }
    }
}
