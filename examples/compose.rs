//! This example shows how to create a popup window and send data back to the parent window.

use std::rc::Rc;

use dioxus::prelude::*;
use futures_util::StreamExt;

fn main() {
    launch_desktop(app);
}

fn app() -> Element {
    let mut emails_sent = use_signal(|| Vec::new() as Vec<String>);

    // Wait for responses to the compose channel, and then push them to the emails_sent signal.
    let handle = use_coroutine(|mut rx: UnboundedReceiver<String>| async move {
        while let Some(message) = rx.next().await {
            emails_sent.write().push(message);
        }
    });

    let open_compose_window = move |_evt: MouseEvent| {
        let tx = handle.tx();
        dioxus_desktop::window().new_window(
            VirtualDom::new_with_props(compose, Rc::new(move |s| tx.unbounded_send(s).unwrap())),
            Default::default(),
        );
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

fn compose(send: Rc<dyn Fn(String)>) -> Element {
    let mut user_input = use_signal(String::new);

    rsx! {
        div {
            h1 { "Compose a new email" }

            button {
                onclick: move |_| {
                    send(user_input.cloned());
                    dioxus_desktop::window().close();
                },
                "Click to send"
            }

            input { oninput: move |e| user_input.set(e.value()), value: "{user_input}" }
        }
    }
}
