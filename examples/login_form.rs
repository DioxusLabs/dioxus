//! This example demonstrates the following:
//! Futures in a callback, Router, and Forms

use dioxus::events::*;
use dioxus::prelude::*;
use dioxus::router::{Link, Router, Route, RouterService};

fn main() {
    dioxus::desktop::launch(APP);
}

static APP: Component = |cx| {
    cx.render(rsx!{
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
    let username = use_state(&cx, String::new);
    let password = use_state(&cx, String::new);

    let service = cx.consume_context::<RouterService>()?;

    let onsubmit = move |_| {
        cx.push_future({
            let (username, password) = (username.get().clone(), password.get().clone());
            let service = service.clone();

            async move {
                let params = [
                    ("username", username.to_string()),
                    ("password", password.to_string())
                ];

                let resp = reqwest::Client::new()
                    .post("http://localhost/login")
                    .form(&params)
                    .send()
                    .await;

                match resp {
                    Ok(data) => {
                        // Parse data from here, such as storing a response token
                        service.push_route("/");
                    }
                    Err(err) => {} //Handle any errors from the fetch here
                }
            }
        });
    };

    cx.render(rsx!{
        h1 { "Login" }
        form {
            onsubmit: onsubmit,
            // Prevent the default behavior of <form> to post
            prevent_default: "onsubmit",
            input {
                oninput: move |evt| username.set(evt.value.clone())
            }
            label {
                "Username"
            }
            br {}
            input {
                oninput: move |evt| password.set(evt.value.clone()),
                r#type: "password"
            }
            label {
                "Password"
            }
            br {}
            button {
                "Login"
            }
        }
    })
}