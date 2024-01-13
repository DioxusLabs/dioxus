//! This example shows how to create a popup window and send data back to the parent window.

use dioxus::prelude::*;
use futures_util::StreamExt;

fn main() {
    dioxus_desktop::launch(app);
}

fn app(cx: Scope) -> Element {
    let emails_sent = use_ref(cx, Vec::new);

    let tx = use_coroutine(cx, |mut rx: UnboundedReceiver<String>| {
        to_owned![emails_sent];
        async move {
            while let Some(message) = rx.next().await {
                emails_sent.write().push(message);
            }
        }
    });

    cx.render(rsx! {
        div {
            h1 { "This is your email" }

            button {
                onclick: move |_| {
                    let dom = VirtualDom::new_with_props(compose, ComposeProps { app_tx: tx.clone() });
                    dioxus_desktop::window().new_window(dom, Default::default());
                },
                "Click to compose a new email"
            }

            ul {
                for message in emails_sent.read().iter() {
                    li {
                        h3 { "email" }
                        span {"{message}"}
                    }
                }
            }
        }
    })
}

struct ComposeProps {
    app_tx: Coroutine<String>,
}

fn compose(cx: Scope<ComposeProps>) -> Element {
    let user_input = use_state(cx, String::new);

    cx.render(rsx! {
        div {
            h1 { "Compose a new email" }

            button {
                onclick: move |_| {
                    cx.props.app_tx.send(user_input.get().clone());
                    dioxus_desktop::window().close();
                },
                "Click to send"
            }

            input { oninput: move |e| user_input.set(e.value()), value: "{user_input}" }
        }
    })
}
