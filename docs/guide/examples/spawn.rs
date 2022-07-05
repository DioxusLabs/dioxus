#![allow(non_snake_case)]

use dioxus::prelude::*;

fn main() {
    dioxus::desktop::launch(App);
}

fn App(cx: Scope) -> Element {
    // ANCHOR: spawn
    let logged_in = use_state(&cx, || false);

    let log_in = move |_| {
        cx.spawn({
            let logged_in = logged_in.to_owned();

            async move {
                let resp = reqwest::Client::new()
                    .post("http://example.com/login")
                    .send()
                    .await;

                match resp {
                    Ok(_data) => {
                        println!("Login successful!");
                        logged_in.set(true);
                    }
                    Err(_err) => {
                        println!(
                            "Login failed - you need a login server running on localhost:8080."
                        )
                    }
                }
            }
        });
    };

    cx.render(rsx! {
        button {
            onclick: log_in,
            "Login",
        }
    })
    // ANCHOR_END: spawn
}

pub fn Tokio(cx: Scope) -> Element {
    let _ = || {
        // ANCHOR: tokio
        cx.spawn(async {
            let _ = tokio::spawn(async {}).await;

            let _ = tokio::task::spawn_local(async {
                // some !Send work
            })
            .await;
        });
        // ANCHOR_END: tokio
    };

    None
}

pub fn ToOwnedMacro(cx: Scope) -> Element {
    let _ = || {
        // ANCHOR: to_owned_macro
        use dioxus::core::to_owned;

        cx.spawn({
            to_owned![count, age, name, description, etc];
            async move {
                // ...
            }
        });
        // ANCHOR_END: to_owned_macro
    };

    cx.render(rsx!())
}
