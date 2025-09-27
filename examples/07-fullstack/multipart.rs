#![cfg_attr(not(feature = "server"), allow(dead_code))]

//! This example demonstrates how to use types like `Form`, `SetHeader`, and `TypedHeader`
//! to create a simple login form that sets a cookie in the browser and uses it for authentication
//! on a protected endpoint.

use dioxus::prelude::*;
use dioxus::{
    fullstack::{Cookie, Form, SetCookie, SetHeader},
    server::axum::extract::Multipart,
};
use serde::{Deserialize, Serialize};

use std::sync::LazyLock;

fn main() {
    dioxus::launch(app);
}

fn app() -> Element {
    let mut fetch_login = use_action(login);

    let onsubmit = move |evt: FormEvent| {
        for file in evt.files() {
            todo!()
        }
    };

    rsx! {
        h1 { "Login Form Demo" }
        form {
            onsubmit,
            input { r#type: "text", id: "name", name: "name" }
            label { r#for: "name", "Text" }
            button { "Submit" }
        }
    }
}

/// In our `login` form, we'll return a `SetCookie` header if the login is successful.
///
/// This will set a cookie in the user's browser that can be used for subsequent authenticated requests.
/// The `SetHeader::new()` method takes anything that can be converted into a `HeaderValue`.
///
/// We can set multiple headers by returning a tuple of `SetHeader` types, or passing in a tuple
/// of headers to `SetHeader::new()`.
#[post("/api/login")]
async fn login(mut form: Multipart) -> Result<()> {
    while let Some(mut field) = form.next_field().await.unwrap() {
        let name = field.name().unwrap().to_string();
        let data = field.bytes().await.unwrap();

        println!("Length of `{}` is {} bytes", name, data.len());
    }

    todo!()
}
