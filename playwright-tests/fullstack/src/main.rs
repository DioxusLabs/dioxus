// This test is used by playwright configured in the root of the repo
// Tests:
// - Server functions
// - SSR
// - Hydration

#![allow(non_snake_case)]
use dioxus::prelude::*;
use dioxus_fullstack::prelude::*;
use serde::{Deserialize, Serialize};

fn main() {
    #[cfg(feature = "web")]
    dioxus_web::launch_with_props(
        app,
        get_root_props_from_document().unwrap_or_default(),
        dioxus_web::Config::new().hydrate(true),
    );
    #[cfg(feature = "ssr")]
    {
        // Start hot reloading
        hot_reload_init!(dioxus_hot_reload::Config::new().with_rebuild_callback(|| {
            execute::shell("dx build --features web")
                .spawn()
                .unwrap()
                .wait()
                .unwrap();
            execute::shell("cargo run --features ssr").spawn().unwrap();
            true
        }));

        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(async move {
                let addr = std::net::SocketAddr::from(([127, 0, 0, 1], 3333));
                axum::Server::bind(&addr)
                    .serve(
                        axum::Router::new()
                            .serve_dioxus_application(
                                "",
                                ServeConfigBuilder::new(app, AppProps { count: 12345 }).build(),
                            )
                            .into_make_service(),
                    )
                    .await
                    .unwrap();
            });
    }
}

#[derive(Props, PartialEq, Debug, Default, Serialize, Deserialize, Clone)]
struct AppProps {
    count: i32,
}

#[allow(unused)]
fn app(cx: Scope<AppProps>) -> Element {
    let mut count = use_state(cx, || cx.props.count);
    let text = use_state(cx, || "...".to_string());

    cx.render(rsx! {
        h1 { "hello axum! {count}" }
        button {
            class: "increment-button",
            onclick: move |_| count += 1,
            "Increment"
        }
        button {
            class: "server-button",
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
            "Run a server function!"
        }
        "Server said: {text}"
    })
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
