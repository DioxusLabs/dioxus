//! A simple example using Dioxus Fullstack to call a server action.
//!
//! the `get`, `post`, `put`, `delete`, etc macros are used to define server actions that can be
//! called from the client. The action can take arguments and return a value, and the client
//! will automatically serialize and deserialize the data.

use dioxus::prelude::*;

fn main() {
    dioxus::launch(|| {
        let mut message = use_action(get_message);

        rsx! {
            h1 { "Server says: "}
            pre { "{message:?}"}
            button { onclick: move |_| message.call("world".into(), 30), "Click me!" }
        }
    });
}

#[get("/api/:name/?age")]
async fn get_message(name: String, age: i32) -> Result<String> {
    Ok(format!("Hello {}, you are {} years old!", name, age))
}
