//! This example shows how to create a popup window and send data back to the parent window.

use dioxus::prelude::*;

fn main() {
    dioxus::LaunchBuilder::desktop().launch(app);
}

fn app() -> Element {
    let mut emails_sent = use_signal(|| Vec::new() as Vec<String>);
    let mut compose_windows = use_signal(Vec::<usize>::new);
    let mut next_window_id = use_signal(|| 0usize);

    // Wait for responses to the compose channel, and then push them to the emails_sent signal.
    let handle = use_coroutine(move |mut rx: UnboundedReceiver<String>| async move {
        use futures_util::StreamExt;
        while let Some(message) = rx.next().await {
            emails_sent.push(message);
        }
    });

    let open_compose_window = move |_evt: MouseEvent| {
        let id = next_window_id();
        next_window_id.set(id + 1);
        compose_windows.write().push(id);
    };

    rsx! {
        h1 { "This is your email" }
        button { onclick: open_compose_window, "Click to compose a new email" }
        for id in compose_windows() {
            Window {
                key: "{id}",
                onclose: move |_| compose_windows.write().retain(|window_id| *window_id != id),
                Popup {
                    send: move |message| {
                        handle.tx().unbounded_send(message).unwrap();
                    }
                }
            }
        }
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

#[component]
fn Popup(send: EventHandler<String>) -> Element {
    let mut user_input = use_signal(String::new);
    let window = dioxus::desktop::use_window();

    let close_window = move |_| {
        println!("Attempting to close Window B");
        window.close();
    };

    rsx! {
        div {
            h1 { "Compose a new email" }
            button {
                onclick: close_window,
                "Close Window B (button)"
            }
            button {
                onclick: move |_| {
                    send.call(user_input.cloned());
                    dioxus::desktop::window().close();
                },
                "Send"
            }
            input { oninput: move |e| user_input.set(e.value()), value: "{user_input}" }
        }
    }
}
