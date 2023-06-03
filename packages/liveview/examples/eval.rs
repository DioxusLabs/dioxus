use axum::{extract::ws::WebSocketUpgrade, response::Html, routing::get, Router};
use dioxus::prelude::*;
use dioxus_liveview::use_eval;

fn app(cx: Scope) -> Element {
    let eval = use_eval(cx);

    cx.render(rsx! {
        div {
            button {
                onclick: move |_| {
                    let fut = eval("console.log(1)".to_string());
                    async move {
                        println!("{:?}", fut.await);
                    }
                },
                "log 1"
            }
            button {
                onclick: move |_| {
                    let fut = eval("return 1;".to_string());
                    async move {
                        println!("{:?}", fut.await);
                    }
                },
                "return 1"
            }
        }
    })
}

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

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

    axum::Server::bind(&addr.to_string().parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}
