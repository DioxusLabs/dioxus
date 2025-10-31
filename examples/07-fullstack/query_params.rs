//! An example showcasing query parameters in Dioxus Fullstack server functions.
//!
//! The query parameter syntax mostly follows axum, but with a few extra conveniences.
//! - can rename parameters in the function signature with `?age=age_in_years` where `age_in_years` is Rust variable name
//! - can absorb all query params with `?{object}` directly into a struct implementing `Deserialize`

use dioxus::prelude::*;

fn main() {
    dioxus::launch(|| {
        let mut message = use_action(get_message);
        let mut message_rebind = use_action(get_message_rebind);
        let mut message_all = use_action(get_message_all);

        rsx! {
            h1 { "Server says: "}
            div {
                button { onclick: move |_| message.call(22), "Single" }
                pre { "{message:?}"}
            }
            div {
                button { onclick: move |_| message_rebind.call(25), "Rebind" }
                pre { "{message_rebind:?}"}
            }
            div {
                button { onclick: move |_| message_all.call(Params { age: 30, name: "world".into() }), "Bind all" }
                pre { "{message_all:?}"}
            }
        }
    });
}

#[get("/api/message/?age")]
async fn get_message(age: i32) -> Result<String> {
    Ok(format!("You are {} years old!", age))
}

#[get("/api/rebind/?age=age_in_years")]
async fn get_message_rebind(age_in_years: i32) -> Result<String> {
    Ok(format!("You are {} years old!", age_in_years))
}

#[derive(serde::Deserialize, serde::Serialize, Debug)]
struct Params {
    age: i32,
    name: String,
}

#[get("/api/all/?{query}")]
async fn get_message_all(query: Params) -> Result<String> {
    Ok(format!(
        "Hello {}, you are {} years old!",
        query.name, query.age
    ))
}
