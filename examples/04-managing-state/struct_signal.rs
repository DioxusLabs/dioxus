//! A signal that holds a struct.
//!
//! Signals don't care what they hold — you can put any `'static` type inside. To mutate
//! a field on a struct signal, call `.write()` and the guard derefs to the inner value.
//! Any read of the signal in the UI will re-render when you drop the write guard.

use dioxus::prelude::*;

fn main() {
    dioxus::launch(app);
}

#[derive(Clone)]
struct User {
    name: String,
    age: u32,
    admin: bool,
}

fn app() -> Element {
    let mut user = use_signal(|| User {
        name: "Ada".to_string(),
        age: 36,
        admin: false,
    });

    rsx! {
        h1 { "User profile" }
        p { "Name: {user.read().name}" }
        p { "Age: {user.read().age}" }
        p { "Admin: {user.read().admin}" }

        button {
            // .write() gives a mutable guard — assign to a single field
            onclick: move |_| user.write().age += 1,
            "Happy birthday!"
        }
        button {
            onclick: move |_| {
                let mut guard = user.write();
                guard.admin = !guard.admin;
            },
            "Toggle admin"
        }

        input {
            value: "{user.read().name}",
            oninput: move |evt| user.write().name = evt.value(),
        }

        // Replacing the whole value works too
        button {
            onclick: move |_| user.set(User {
                name: "Grace".to_string(),
                age: 85,
                admin: true,
            }),
            "Reset to Grace"
        }
    }
}
