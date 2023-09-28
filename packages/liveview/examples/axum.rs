use axum::{extract::ws::WebSocketUpgrade, response::Html, routing::get, Router};
use dioxus::prelude::*;

fn app(cx: Scope) -> Element {
    let mut num = use_state(cx, || 0);

    cx.render(rsx! {
        div {
            "hello axum! {num}"
            button { onclick: move |_| num += 1, "Increment" }
        }
    })
}

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    let addr: std::net::SocketAddr = ([127, 0, 0, 1], 3030).into();

    let view = dioxus_liveview::LiveViewPool::new();
    let index_page_with_glue = |glue: &str| {
        Html(format!(
            r#"
        <!DOCTYPE html>
        <html>
            <head> <title>Dioxus LiveView with axum</title>  </head>
            <body> <div id="main"></div> </body>
            {glue}
        </html>
        "#,
        ))
    };

    let app =
        Router::new()
            .route(
                "/",
                get(move || async move {
                    index_page_with_glue(&dioxus_liveview::interpreter_glue(&format!(
                        "ws://{addr}/ws"
                    )))
                }),
            )
            .route(
                "/as-path",
                get(move || async move {
                    index_page_with_glue(&dioxus_liveview::interpreter_glue("/ws"))
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

    axum::Server::bind(&addr.to_string().parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}
