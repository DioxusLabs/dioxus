//! This example shows how to add custom middleware to the server client that Dioxus uses to make
//! server function requests.
//!
//! In this example, we add a JWT token to the headers of every request

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
