use dioxus::prelude::*;
use dioxus_fullstack::{ServerFnSugar, ServerFunction};

fn main() {
    // `/`
    dioxus::launch(app);
}

fn app() -> Element {
    rsx! {
        button {
            onclick: move |_| async move {
                let res = upload_user(123).await.unwrap();
            },
            "Fetch Data"
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
struct User {
    id: String,
    name: String,
}

#[post("/api/user/{id}")]
async fn upload_user(id: i32) -> anyhow::Result<User> {
    Ok(User {
        id: id.to_string(),
        name: "John Doe".into(),
    })
}
