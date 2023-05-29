#![allow(unused)]
#![allow(non_snake_case)]
// ANCHOR: all

// ANCHOR: main
#![allow(non_snake_case)]
use axum::{response::Html, routing::get, Router};
// import the prelude to get access to the `rsx!` macro and the `Scope` and `Element` types
use dioxus::prelude::*;

#[tokio::main]
async fn main() {
    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("listening on http://{}", addr);

    axum::Server::bind(&addr)
        .serve(
            Router::new()
                .route("/", get(app_endpoint))
                .into_make_service(),
        )
        .await
        .unwrap();
}

// ANCHOR_END: main

// ANCHOR: endpoint
async fn app_endpoint() -> Html<String> {
    // render the rsx! macro to HTML
    Html(dioxus_ssr::render_lazy(rsx! {
        div { "hello world!" }
    }))
}
// ANCHOR_END: endpoint

// ANCHOR: second_endpoint
async fn second_app_endpoint() -> Html<String> {
    // create a component that renders a div with the text "hello world"
    fn app(cx: Scope) -> Element {
        cx.render(rsx!(div { "hello world" }))
    }
    // create a VirtualDom with the app component
    let mut app = VirtualDom::new(app);
    // rebuild the VirtualDom before rendering
    let _ = app.rebuild();

    // render the VirtualDom to HTML
    Html(dioxus_ssr::render(&app))
}
// ANCHOR_END: second_endpoint

// ANCHOR: component
// define a component that renders a div with the text "Hello, world!"
fn App(cx: Scope) -> Element {
    cx.render(rsx! {
        div {
            "Hello, world!"
        }
    })
}
// ANCHOR_END: component
// ANCHOR_END: all
