use dioxus::prelude::*;

fn main() {
    dioxus::launch(app);
}

fn app() -> Element {
    let mut user_from_server_fn = use_action(get_user);
    let mut user_from_reqwest = use_action(move |id: i32| async move {
        reqwest::get(&format!("http://localhost:8000/api/user/{}", id))
            .await?
            .json::<User>()
            .await
    });

    rsx! {
        button { onclick: move |_| user_from_server_fn.dispatch(123), "Fetch Data" }
        div { "User from server: {user_from_server_fn.value():?}", }

        button { onclick: move |_| user_from_reqwest.dispatch(456), "Fetch From Endpoint" }
        div { "User from server: {user_from_reqwest.value():?}", }
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
