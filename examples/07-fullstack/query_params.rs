//! An example showcasing query parameters in Dioxus Fullstack server functions.
//!
//! The query parameter syntax mostly follows axum, but with a few extra conveniences.
//! - can rename parameters in the function signature with `?age=age_in_years` where `age_in_years` is Rust variable name
//! - can absorb all query params with `?{object}` directly into a struct implementing `Deserialize`

use dioxus::prelude::*;

fn main() {
    dioxus::launch(|| {
        let mut message = use_action(get_message);

        rsx! {
            h1 { "Server says: "}
            pre { "{message:?}"}
            button { onclick: move |_| message.call(Params { age: 30, name: "world".into() }), "Click me!" }
        }
    });
}

// #[cfg(feature = "server")]
// use {axum::extract::State, dioxus::server::axum, fullstack::DioxusServerState};

#[derive(serde::Deserialize, serde::Serialize, Debug)]
struct Params {
    age: i32,
    name: String,
}

#[get("/api/1/?:query")]
async fn get_message(query: Params) -> Result<()> {
    println!("Custom server state abc: {:?}", query);
    Ok(())
    // Ok(format!("Hello {}, you are {} years old!", name, age))
}

// async fn get_message(query: Params) -> Result<()> {

// #[get("/api/:name/?age", state: State<MyCustomServerState>)]
// #[derive(Debug)]
// struct MyReturnType;

// #[derive(Clone)]
// struct MyCustomServerState {
//     abc: i32,
// }

// // #[cfg(feature = "server")]
// use dioxus_fullstack::axum;
// impl axum::extract::FromRef<DioxusServerState> for MyCustomServerState {
//     fn from_ref(state: &DioxusServerState) -> Self {
//         MyCustomServerState { abc: 123 }
//     }
// }

// #[get("/api/")]
// async fn get_message2(item: MyThing) -> Result<()> {
//     todo!()
//     // Ok(format!("Hello {}, you are {} years old!", name, age))
// }

// struct MyThing;

#[get("/api/2/?age=age_in_years")]
async fn get_message2(age_in_years: i32) -> Result<String> {
    Ok(format!("You are {} years old!", age_in_years))
}

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
