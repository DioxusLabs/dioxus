use axum::response::IntoResponse;
use axum::Json;
use dioxus_fullstack::{
    post,
    req_from::{DeSer, ExtractRequest, ExtractState},
    req_to::{EncodeRequest, EncodeState, ReqSer},
    ServerFnSugar, ServerFunction,
};

fn main() {}

#[derive(serde::Serialize, serde::Deserialize)]
struct User {
    id: String,
    name: String,
    age: i32,
}

#[post("/api/user/{id}")]
async fn upload_user(id: i32, name: String, age: i32) -> anyhow::Result<User> {
    Ok(User {
        id: id.to_string(),
        name: "John Doe".into(),
        age: 123,
    })
}
