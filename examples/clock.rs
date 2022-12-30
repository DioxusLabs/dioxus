//! Example: README.md showcase
//!
//! The example from the README.md.

use dioxus::prelude::*;

fn main() {
    dioxus_desktop::launch(app);
}

fn app(cx: Scope) -> Element {
    let count = use_ref(cx, || 0);

    let ct = count.to_owned();
    use_coroutine(cx, |_: UnboundedReceiver<()>| async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;

            *ct.write() += 1;

            let current = *ct.read();

            println!("current: {}", current);
        }
    });

    let count = count.read();

    cx.render(rsx! {
        div { "High-Five counter: {count}" }
    })
}
