//! An example showcasing query parameters in Dioxus Fullstack server functions.
//!
//! The query parameter syntax mostly follows axum, but with a few extra conveniences.
//! - can rename parameters in the function signature with `?age=age_in_years` where `age_in_years` is Rust variable name
//! - can absorb all query params with `?{object}` directly into a struct implementing `Deserialize`

use dioxus::{fullstack::DioxusServerState, prelude::*};

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

// #[cfg(feature = "server")]
// use {axum::extract::State, dioxus::server::axum, fullstack::DioxusServerState};

#[get("/api/:name/?age", state: dioxus_fullstack::extract::State<MyCustomServerState>)]
// #[get("/api/:name/?age", state: State<MyCustomServerState>)]
async fn get_message(name: String, age: i32) -> Result<()> {
    todo!()
    // Ok(format!("Hello {}, you are {} years old!", name, age))
}

#[derive(Debug)]
struct MyReturnType;

#[derive(Clone)]
struct MyCustomServerState {
    abc: i32,
}

// #[cfg(feature = "server")]
use dioxus_fullstack::axum;
impl axum::extract::FromRef<DioxusServerState> for MyCustomServerState {
    fn from_ref(state: &DioxusServerState) -> Self {
        MyCustomServerState { abc: 123 }
    }
}

// #[get("/api/")]
// async fn get_message2(item: MyThing) -> Result<()> {
//     todo!()
//     // Ok(format!("Hello {}, you are {} years old!", name, age))
// }

// struct MyThing;

// #[get("/api/:name/?age={age_in_years}")]
// async fn get_message2(name: String, age_in_years: i32) -> Result<String> {
//     Ok(format!(
//         "Hello {}, you are {} years old!",
//         name, age_in_years
//     ))
// }

// #[derive(serde::Deserialize)]
// struct Params {
//     age: i32,
// }

// #[get("/api/:name/?{params}")]
// async fn get_message3(name: String, params: Params) -> Result<String> {
//     Ok(format!("Hello {}, you are {} years old!", name, params.age))
// }

// // Absorb both path and query parameters into a struct
// #[get("/api/?name&{params}")]
// async fn get_message4(name: String, params: Params) -> Result<String> {
//     Ok(format!("Hello {}, you are {} years old!", name, params.age))
// }
