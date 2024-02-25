// This test is used by playwright configured in the root of the repo

use axum::{extract::ws::WebSocketUpgrade, response::Html, routing::get, Router};
use dioxus::prelude::*;

fn app() -> Element {
    let mut num = use_signal(|| 0);

    rsx! {
        div {
            "hello axum! {num}"
            button { onclick: move |_| num += 1, "Increment" }
        }
        svg { circle { cx: 50, cy: 50, r: 40, stroke: "green", fill: "yellow" } }
        div { class: "raw-attribute-div", "raw-attribute": "raw-attribute-value" }
        div { class: "hidden-attribute-div", hidden: true }
        div {
            class: "dangerous-inner-html-div",
            dangerous_inner_html: "<p>hello dangerous inner html</p>"
        }
        input { value: "hello input" }
        div { class: "style-div", color: "red", "colored text" }
    }
}

#[tokio::main]
async fn main() {
    let addr: std::net::SocketAddr = ([127, 0, 0, 1], 3030).into();

    let view = dioxus_liveview::LiveViewPool::new();

    let app = Router::new()
        .route(
            "/",
            get(move || async move {
                Html(format!(
                    r#"
            <!DOCTYPE html>
            <html>
                <head> <title>Dioxus LiveView with axum</title>  </head>
                <body> <div id="main"></div> </body>
                {glue}
            </html>
            "#,
                    glue = dioxus_liveview::interpreter_glue(&format!("ws://{addr}/ws"))
                ))
            }),
        )
        .route(
            "/ws",
            get(move |ws: WebSocketUpgrade| async move {
                ws.on_upgrade(move |socket| async move {
                    _ = view.launch(dioxus_liveview::axum_socket(socket), app).await;
                })
            }),
        );

    println!("Listening on http://{addr}");

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}
