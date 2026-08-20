//! This example shows how to create a popup window and send data back to the parent window.
//! The app root is headless and owns each native window explicitly.

use dioxus::desktop::{Config, WindowBuilder, WindowConfig};
use dioxus::prelude::*;

#[derive(Store, PartialEq, Clone, Debug, Default)]
struct MailState {
    emails_sent: Vec<String>,
}

fn main() {
    dioxus::LaunchBuilder::desktop()
        .with_cfg(Config::new().with_headless_root(true))
        .launch(app);
}

fn app() -> Element {
    let mail = use_store(MailState::default);
    let mut inbox_open = use_signal(|| true);
    let mut compose_windows = use_signal(Vec::<usize>::new);
    let mut next_window_id = use_signal(|| 0usize);

    let mut open_compose_window = move || {
        let id = next_window_id();
        next_window_id.set(id + 1);
        compose_windows.write().push(id);
    };

    rsx! {
        if inbox_open() {
            Window {
                config: WindowConfig::new().with_window(
                    WindowBuilder::new().with_title("Inbox")
                ),
                onclose: move |_| inbox_open.set(false),
                Inbox {
                    mail,
                    open_compose_window: move |_| open_compose_window()
                }
            }
        }
        for id in compose_windows() {
            Window {
                key: "{id}",
                config: WindowConfig::new().with_window(
                    WindowBuilder::new().with_title(format!("Compose {id}"))
                ),
                onclose: move |_| compose_windows.write().retain(|window_id| *window_id != id),
                Popup { mail }
            }
        }
    }
}

#[component]
fn Inbox(mail: Store<MailState>, open_compose_window: EventHandler<()>) -> Element {
    rsx! {
        h1 { "This is your email" }
        button {
            onclick: move |_| open_compose_window.call(()),
            "Click to compose a new email"
        }
        ul {
            for message in mail.emails_sent().iter() {
                li {
                    h3 { "email" }
                    span { "{message}" }
                }
            }
        }
    }
}

#[component]
fn Popup(mut mail: Store<MailState>) -> Element {
    let mut user_input = use_signal(String::new);

    rsx! {
        form {
            onsubmit: move |event| {
                event.prevent_default();
                mail.emails_sent().push(user_input.cloned());
                dioxus::desktop::window().close();
            },
            h1 { "Compose a new email" }
            input {
                name: "message",
                oninput: move |e| user_input.set(e.value()),
                value: "{user_input}"
            }
            button { r#type: "submit", "Send" }
        }
    }
}
