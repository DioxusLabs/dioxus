//! This example demonstrates the following:
//! Futures in a callback, Router, and Forms

use dioxus::events::*;
use dioxus::prelude::*;
use dioxus::router::{use_router, Link, Route, Router};

fn main() {
    dioxus::desktop::launch(app);
}

fn app(cx: Scope) -> Element {
    cx.render(rsx! {
        Router {
            Route { to: "/", home() }
            Route { to: "/login", login() }
        }
    })
}

fn home(cx: Scope) -> Element {
    cx.render(rsx! {
        h1 { "Welcome Home" }
        Link { to: "/login", "Login" }
    })
}

fn login(cx: Scope) -> Element {
    let service = use_router(&cx);

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
                Ok(_data) => service.push_route("/"),

                //Handle any errors from the fetch here
                Err(_err) => {}
            }
        });
    };

    cx.render(rsx! {
        h1 { "Login" }
        form {
            onsubmit: onsubmit,
            prevent_default: "onsubmit", // Prevent the default behavior of <form> to post

            input { "type": "text" }
            label { "Username" }
            br {}
            input { "type": "password" }
            label { "Password" }
            br {}
            button { "Login" }
        }
    })
}
