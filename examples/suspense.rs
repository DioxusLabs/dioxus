use dioxus::prelude::*;
use dioxus_signals::use_signal;

fn main() {
    dioxus_desktop::launch(app);
}

fn app(cx: Scope) -> Element {
    cx.render(rsx!(
        div {
            "Waiting for... "
            suspended_child {}
        }
    ))
}

fn suspended_child(cx: Scope) -> Element {
    let mut val = use_signal(cx, || 1);

    if val.value() % 10 == 0 {
        cx.spawn(async move {
            tokio::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            })
            .await;
            val += 1;
        });
        cx.suspend()?;
    }

    render! {
        button {
            onclick: move |_| {
                val += 1;
            },
            "{val}"
        }
    }
}
