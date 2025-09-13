use dioxus::prelude::*;

fn main() {
    dioxus::launch(app);
}

fn app() -> Element {
    let fetch_data = move |_| async move {
        get_user(123).await?;
        Ok(())
    };

    let fetch_from_endpoint = move |_| async move {
        reqwest::get("http://localhost:8000/api/user/123")
            .await
            .unwrap()
            .json::<User>()
            .await
            .unwrap();
        Ok(())
    };

    rsx! {
        button { onclick: fetch_data, "Fetch Data" }
        button { onclick: fetch_from_endpoint, "Fetch From Endpoint" }
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
struct User {
    id: String,
    name: String,
}

#[get("/api/user/{id}")]
async fn get_user(id: i32) -> anyhow::Result<User> {
    Ok(User {
        id: id.to_string(),
        name: "John Doe".into(),
    })
}

#[post("/api/user/{id}")]
async fn update_user(id: i32, name: String) -> anyhow::Result<User> {
    Ok(User {
        id: id.to_string(),
        name,
    })
}

#[server]
async fn update_user_auto(id: i32, name: String) -> anyhow::Result<User> {
    Ok(User {
        id: id.to_string(),
        name,
    })
}
