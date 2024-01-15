//! This example shows how to create a popup window and send data back to the parent window.

use dioxus::prelude::*;
use futures_util::StreamExt;
use tokio::sync::mpsc::UnboundedSender;

fn main() {
    dioxus_desktop::launch(app);
}

fn app() -> Element {
    let emails_sent = use_signal(|| Vec::new() as Vec<String>);

    // Wait for responses to the compose channel, and then push them to the emails_sent signal.
    let tx = use_coroutine(|mut rx: UnboundedReceiver<String>| async move {
        while let Some(message) = rx.next().await {
            emails_sent.write().push(message);
        }
    });

    let open_compose_window = move |evt: MouseEvent| {
        dioxus_desktop::window().new_window(
            VirtualDom::new_with_props(compose, tx.clone()),
            Default::default(),
        )
    };

    rsx! {
        h1 { "This is your email" }
        button { onclick: open_compose_window, "Click to compose a new email" }
        ul {
            for message in emails_sent.read().iter() {
                li {
                    h3 { "email" }
                    span { "{message}" }
                }
            }
        }
    }
}

fn compose(receiver: UnboundedSender<String>) -> Element {
    let user_input = use_signal(String::new);

    rsx! {
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
    }
}
