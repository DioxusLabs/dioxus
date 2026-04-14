//! This example demonstrates that dioxus server functions can be called directly as a Rust
//! function or via an HTTP request using reqwest.
//!
//! Dioxus server functions generated a REST endpoint that can be called using any HTTP client.
//! By default, they also support different serialization formats like JSON and CBOR. Try changing
//! your `accept` header to see the different formats.

use dioxus::prelude::*;

fn main() {
    dioxus::launch(app);
}

fn app() -> Element {
    let mut user_from_server_fn = use_action(get_user);

    let mut user_from_reqwest = use_action(move |id: i32| async move {
        let port = dioxus::cli_config::server_port().unwrap_or(8080);
        reqwest::get(&format!("http://localhost:{}/api/user/{}", port, id))
            .await?
            .json::<User>()
            .await
    });

    rsx! {
        button { onclick: move |_| user_from_server_fn.call(123), "Fetch Data" }
        button { onclick: move |_| user_from_reqwest.call(456), "Fetch From Endpoint" }
        div { display: "flex", flex_direction: "column",
            pre { "User from server: {user_from_server_fn.value():?}", }
            pre { "User from server: {user_from_reqwest.value():?}", }
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
struct User {
    id: String,
    name: String,
}

#[get("/api/user/{id}")]
async fn get_user(id: i32) -> Result<User> {
    Ok(User {
        id: id.to_string(),
        name: "John Doe".into(),
    })
}
