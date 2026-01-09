//! This example shows how to create a popup window and send data back to the parent window.
//! Currently Dioxus doesn't support nested renderers, hence the need to create popups as separate windows.
//!
//! Note: With the threaded VirtualDom model, we use global signals to communicate between windows.

use dioxus::prelude::*;

fn main() {
    dioxus::LaunchBuilder::desktop().launch(app);
}

/// Global signal for cross-window communication
static EMAIL_CHANNEL: GlobalSignal<Vec<String>> = Signal::global(Vec::new);

fn app() -> Element {
    // Read emails from the global channel
    let emails_sent = EMAIL_CHANNEL();

    let open_compose_window = move |_evt: MouseEvent| {
        dioxus::desktop::window().new_window(popup, Default::default());
    };

    rsx! {
        h1 { "This is your email" }
        button { onclick: open_compose_window, "Click to compose a new email" }
        ul {
            for message in emails_sent.iter() {
                li {
                    h3 { "email" }
                    span { "{message}" }
                }
            }
        }
    }
}

fn popup() -> Element {
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
                    // Send the message via global signal
                    let mut emails = EMAIL_CHANNEL.write();
                    emails.push(user_input.cloned());
                    dioxus::desktop::window().close();
                },
                "Send"
            }
            input { oninput: move |e| user_input.set(e.value()), value: "{user_input}" }
        }
    }
}
