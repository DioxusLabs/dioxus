// ANCHOR: all
use axum::{extract::ws::WebSocketUpgrade, response::Html, routing::get, Router};
use dioxus::prelude::*;

// ANCHOR: glue
#[tokio::main]
async fn main() {
    let addr: std::net::SocketAddr = ([127, 0, 0, 1], 3030).into();

    let view = dioxus_liveview::LiveViewPool::new();

    let app = Router::new()
        // The root route contains the glue code to connect to the WebSocket
        .route(
            "/",
            get(move || async move {
                Html(format!(
                    r#"
                <!DOCTYPE html>
                <html>
                <head> <title>Dioxus LiveView with Axum</title>  </head>
                <body> <div id="main"></div> </body>
                {glue}
                </html>
                "#,
                    // Create the glue code to connect to the WebSocket on the "/ws" route
                    glue = dioxus_liveview::interpreter_glue(&format!("ws://{addr}/ws"))
                ))
            }),
        )
        // The WebSocket route is what Dioxus uses to communicate with the browser
        .route(
            "/ws",
            get(move |ws: WebSocketUpgrade| async move {
                ws.on_upgrade(move |socket| async move {
                    // When the WebSocket is upgraded, launch the LiveView with the app component
                    _ = view.launch(dioxus_liveview::axum_socket(socket), app).await;
                })
            }),
        );

    println!("Listening on http://{}", addr);

    axum::Server::bind(&addr.to_string().parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}
// ANCHOR_END: glue

// ANCHOR: app
fn app(cx: Scope) -> Element {
    cx.render(rsx! {
        div {
            "Hello, world!"
        }
    })
}
// ANCHOR_END: app
// ANCHOR_END: all
