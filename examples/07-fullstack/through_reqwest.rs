//! We can call server functions directly from reqwest as well as through Dioxus's built-in
//! server function support. This example shows both methods of calling the same server function.

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
