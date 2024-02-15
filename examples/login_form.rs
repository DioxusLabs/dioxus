//! Implementing a login form
//!
//! This example demonstrates how to implement a login form using Dioxus desktop. Since forms typically navigate the
//! page on submit, we need to intercept the onsubmit event and send a request to a server. On the web, we could
//! just leave the submit action` as is, but on desktop, we need to handle the form submission ourselves.
//!
//! Todo: actually spin up a server and run the login flow. Login is way more complex than a form override :)

use dioxus::prelude::*;

fn main() {
    launch_desktop(app);
}

fn app() -> Element {
    let onsubmit = move |evt: FormEvent| async move {
        let resp = reqwest::Client::new()
            .post("http://localhost:8080/login")
            .form(&[
                ("username", &evt.values()["username"]),
                ("password", &evt.values()["password"]),
            ])
            .send()
            .await;

        match resp {
            // Parse data from here, such as storing a response token
            Ok(_data) => println!("Login successful!"),

            //Handle any errors from the fetch here
            Err(_err) => {
                println!("Login failed - you need a login server running on localhost:8080.")
            }
        }
    };

    rsx! {
        h1 { "Login" }
        form { onsubmit,
            input { r#type: "text", id: "username", name: "username" }
            label { "Username" }
            br {}
            input { r#type: "password", id: "password", name: "password" }
            label { "Password" }
            br {}
            button { "Login" }
        }
    }
}
