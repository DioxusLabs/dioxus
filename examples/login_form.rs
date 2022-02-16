//! This example demonstrates the following:
//! Futures in a callback, Router, and Forms

use dioxus::events::*;
use dioxus::prelude::*;
use dioxus::router::{Link, Route, Router, RouterService};

fn main() {
    dioxus::desktop::launch(APP);
}

static APP: Component = |cx| {
    cx.render(rsx! {
        Router {
            Route { to: "/", home() }
            Route { to: "/login", login() }
        }
    })
};

fn home(cx: Scope) -> Element {
    cx.render(rsx! {
        h1 { "Welcome Home" }
        Link { to: "/login", "Login" }
    })
}

fn login(cx: Scope) -> Element {
    let service = cx.consume_context::<RouterService>()?;

    let onsubmit = move |evt: FormEvent| {
        to_owned![service];
        let username = evt.values["username"].clone();
        let password = evt.values["password"].clone();

        cx.spawn(async move {
            let resp = reqwest::Client::new()
                .post("http://localhost/login")
                .form(&[("username", username), ("password", password)])
                .send()
                .await;

            match resp {
                // Parse data from here, such as storing a response token
                Ok(data) => service.push_route("/"),

                //Handle any errors from the fetch here
                Err(err) => {}
            }
        });
    };

    cx.render(rsx! {
        h1 { "Login" }
        form {
            onsubmit: onsubmit,
            prevent_default: "onsubmit", // Prevent the default behavior of <form> to post
            input { r#type: "text" }
            label { "Username" }
            br {}
            input { r#type: "password" }
            label {
                "Password"
            }
            br {}
            button { "Login" }
        }
    })
}
