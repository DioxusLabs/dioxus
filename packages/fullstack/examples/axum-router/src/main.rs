//! Run with:
//!
//! ```sh
//! dx serve --platform fullstack
//! ```

use dioxus::prelude::*;

fn main() {
    let cfg = server_only!(dioxus::fullstack::Config::new().incremental(
        IncrementalRendererConfig::default().invalidate_after(std::time::Duration::from_secs(120)),
    ));

    LaunchBuilder::fullstack().with_cfg(cfg).launch(app);
}

fn app() -> Element {
    rsx! {
        Router::<Route> {}
    }
}

#[derive(Clone, Routable, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
enum Route {
    #[route("/")]
    Home {},

    #[route("/blog/:id")]
    Blog { id: i32 },
}

#[component]
fn Blog(id: i32) -> Element {
    rsx! {
        Link { to: Route::Home {}, "Go to counter" }
        table {
            tbody {
                for _ in 0..id {
                    tr {
                        for _ in 0..id {
                            td { "hello world!" }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn Home() -> Element {
    let mut count = use_signal(|| 0);
    let text = use_signal(|| "...".to_string());

    rsx! {
        Link {
            to: Route::Blog {
                id: count()
            },
            "Go to blog"
        }
        div {
            h1 { "High-Five counter: {count}" }
            button { onclick: move |_| count += 1, "Up high!" }
            button { onclick: move |_| count -= 1, "Down low!" }
            button {
                onclick: move |_| {
                    to_owned![text];
                    async move {
                        if let Ok(data) = get_server_data().await {
                            println!("Client received: {}", data);
                            text.set(data.clone());
                            post_server_data(data).await.unwrap();
                        }
                    }
                },
                "Run server function!"
            }
            "Server said: {text}"
        }
    }
}

#[server(PostServerData)]
async fn post_server_data(data: String) -> Result<(), ServerFnError> {
    println!("Server received: {}", data);

    Ok(())
}

#[server(GetServerData)]
async fn get_server_data() -> Result<String, ServerFnError> {
    Ok("Hello from the server!".to_string())
}
