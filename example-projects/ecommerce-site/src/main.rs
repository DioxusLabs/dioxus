#![allow(non_snake_case)]

use axum::{
    extract::{Path, WebSocketUpgrade},
    response::Html,
    routing::get,
    Router,
};
use components::home::Home;
use dioxus::prelude::*;
use std::{future::Future, net::SocketAddr};
use tokio::runtime::Handle;
use tower_http::services::ServeDir;

mod components {
    pub mod error;
    pub mod home;
    pub mod nav;
    pub mod product_item;
    pub mod product_page;
}
mod api;

#[tokio::main]
async fn main() {
    // Create a liveview pool
    let view = dioxus_liveview::LiveViewPool::new();

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));

    // build our application router
    let app = Router::new()
        // serve the public directory
        .nest_service("/public", ServeDir::new("public"))
        // serve the SSR rendered homepage
        .route("/", get(root))
        // serve the liveview rendered details page
        .route(
            "/details/:id",
            get(move |Path(id): Path<usize>| async move {
                Html(format!(
                    r#"
            <!DOCTYPE html>
            <html>
                <head>
                    <title>Dioxus Ecomerse</title>
                    <link rel="stylesheet" href="/public/tailwind.css">
                </head>
                <body> <div id="main"></div> </body>
                {}
            </html>
            "#,
                    dioxus_liveview::interpreter_glue(&format!("ws://{addr}/details/{id}/ws"))
                ))
            }),
        )
        .route(
            "/details/:id/ws",
            get(
                move |Path(id): Path<usize>, ws: WebSocketUpgrade| async move {
                    ws.on_upgrade(move |socket| async move {
                        _ = view
                            .launch_with_props(dioxus_liveview::axum_socket(socket), details, id)
                            .await;
                    })
                },
            ),
        );

    // run it
    println!("listening on http://{}", addr);
    println!("- Route available on http://{}", addr);
    println!("- Route available on http://{}/details/1", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

// Just render a simple page directly from the request
async fn root() -> Html<String> {
    // The root page blocks on futures so we need to render it in a spawn_blocking task
    tokio::task::spawn_blocking(move || async move {
        let mut app = VirtualDom::new(Home);
        let _ = app.rebuild();
        Html(dioxus_ssr::render(&app))
    })
    .await
    .unwrap()
    .await
}

/// Render a more sophisticated page with ssr
fn details(cx: Scope<usize>) -> Element {
    cx.render(rsx!(
        div {
            components::nav::nav {}
            components::product_page::product_page {
                product_id: *cx.props
            }
        }
    ))
}

pub(crate) fn block_on<T: Send + Sync + 'static>(
    f: impl Future<Output = T> + Send + Sync + 'static,
) -> T {
    let handle = Handle::current();
    std::thread::spawn(move || {
        // Using Handle::block_on to run async code in the new thread.
        handle.block_on(f)
    })
    .join()
    .unwrap()
}
