//! Run with:
//!
//! ```sh
//! dioxus build --features web
//! cargo run --features ssr --no-default-features
//! ```

#![allow(non_snake_case)]
use dioxus::prelude::*;
use dioxus_fullstack::prelude::*;
use dioxus_router::prelude::*;
use serde::{Deserialize, Serialize};

fn main() {
    #[cfg(feature = "web")]
    dioxus_web::launch_with_props(
        App,
        AppProps { route: None },
        dioxus_web::Config::new().hydrate(true),
    );
    #[cfg(feature = "ssr")]
    {
        // Start hot reloading
        hot_reload_init!(dioxus_hot_reload::Config::new().with_rebuild_callback(|| {
            execute::shell("dioxus build --features web")
                .spawn()
                .unwrap()
                .wait()
                .unwrap();
            execute::shell("cargo run --features ssr --no-default-features")
                .spawn()
                .unwrap();
            true
        }));

        use axum::extract::State;
        use std::str::FromStr;

        PostServerData::register().unwrap();
        GetServerData::register().unwrap();
        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(async move {
                let addr = std::net::SocketAddr::from(([127, 0, 0, 1], 8080));

                axum::Server::bind(&addr)
                    .serve(
                        axum::Router::new()
                            // Serve the dist/assets folder with the javascript and WASM files created by the CLI
                            .serve_static_assets("./dist")
                            // Register server functions
                            .register_server_fns("")
                            // Connect to the hot reload server
                            .connect_hot_reload()
                            // If the path is unknown, render the application
                            .fallback(
                                move |uri: http::uri::Uri, State(ssr_state): State<SSRState>| {
                                    let rendered = ssr_state.render(
                                        &ServeConfigBuilder::new(
                                            App,
                                            AppProps {
                                                route: Route::from_str(&uri.to_string()).ok(),
                                            },
                                        )
                                        .build(),
                                    );
                                    async move { axum::body::Full::from(rendered) }
                                },
                            )
                            .with_state(SSRState::default())
                            .into_make_service(),
                    )
                    .await
                    .unwrap();
            });
    }
}

#[derive(Clone, Routable, Serialize, Deserialize, Debug, PartialEq)]
enum Route {
    #[route("/")]
    Home {},
    #[route("/blog")]
    Blog {},
}

#[inline_props]
fn Blog(cx: Scope) -> Element {
    render! {
        Link {
            target: Route::Home {},
            "Go to counter"
        }
        table {
            tbody {
                for _ in 0..100 {
                    tr {
                        for _ in 0..100 {
                            td { "hello world!" }
                        }
                    }
                }
            }
        }
    }
}

#[derive(Clone, Debug, Props, PartialEq, Serialize, Deserialize)]
struct AppProps {
    route: Option<Route>,
}

fn App(cx: Scope<AppProps>) -> Element {
    #[cfg(feature = "ssr")]
    let initial_route = cx.props.route.clone().unwrap_or(Route::Home {});
    cx.render(rsx! {
        Router {
            config: move || RouterConfig::default().history({
                #[cfg(feature = "ssr")]
                let history = MemoryHistory::with_initial_path(initial_route);
                #[cfg(feature = "web")]
                let history = WebHistory::default();
                history
            })
        }
    })
}

#[inline_props]
fn Home(cx: Scope) -> Element {
    let mut count = use_state(cx, || 0);
    let text = use_state(cx, || "...".to_string());

    cx.render(rsx! {
        Link {
            target: Route::Blog {}
            "Go to blog"
        }
        div{
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
                "Run a server function"
            }
            "Server said: {text}"
        }
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
