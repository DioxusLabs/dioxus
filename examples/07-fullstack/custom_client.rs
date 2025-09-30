//! This example shows how to use a custom client with Dioxus.
//!
//! You can customize an outgoing request by using methods on the request future before `await`ing it.
//! This is useful for adding custom headers, authentication, or other request modifications.
//!
//! You can also use an entirely different http client

use std::any::TypeId;

use dioxus::prelude::*;

fn main() {
    dioxus::launch(app);
}

fn app() -> Element {
    // let mut fetch_with_custom_headers = use_action(|| async {
    //     get_user(123)
    //         .header("x-custom-header", "my-value")
    //         .header("x-custom-header", "my-value")
    //         .header("x-custom-header", "my-value")
    //         .await
    // });

    todo!()

    // rsx! {
    //     button { onclick: move |_| user_from_server_fn.call(123), "Fetch Data" }
    //     button { onclick: move |_| user_from_reqwest.call(456), "Fetch From Endpoint" }
    //     div { display: "flex", flex_direction: "column",
    //         pre { "User from server: {user_from_server_fn.value():?}", }
    //         pre { "User from server: {user_from_reqwest.value():?}", }
    //     }
    // }
}

#[post("/api/user/{id}")]
async fn get_user(id: i32) -> Result<()> {
    todo!()
}

// struct MyDb(dioxus::fullstack::Cient);

// impl MyDb {
//     #[get("/api/user/{id}")]
//     async fn get_user(&self, id: i32) -> Result<User> {
//         todo!()
//     }
// }

// MyDb.get_user(123).await;
// MyDebug::get_user(123).header("x-custom-header", "my-value").await;

// fn it_works(s: Self) {
//     TypeId::of::<Self>();
// }
