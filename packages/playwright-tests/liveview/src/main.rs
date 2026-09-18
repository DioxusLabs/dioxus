// This test is used by playwright configured in the root of the repo

use axum::{extract::ws::WebSocketUpgrade, response::Html, routing::get, Router};
use dioxus::{logger::tracing::Level, prelude::*};

fn app() -> Element {
    let mut num = use_signal(|| 0);
    let mut delayed_eval_result = use_signal(String::new);

    rsx! {
        div {
            "hello axum! {num}"
            button { onclick: move |_| num += 1, "Increment" }
        }
        button {
            onclick: move |_| async move {
                let mut eval = document::eval(
                    r#"
                    const reply = dioxus.recv();
                    setTimeout(() => dioxus.send("ready"), 100);
                    return await reply;
                    "#,
                );
                let ready: String = eval.recv().await.expect("browser should start receiving");
                assert_eq!(ready, "ready");

                eval.send("delivered").expect("query should accept a message");
                let result: String = eval.join().await.expect("browser should return the message");
                delayed_eval_result.set(result);
            },
            "Receive delayed message"
        }
        div { class: "delayed-eval-result", "{delayed_eval_result}" }
        svg { circle { cx: 50, cy: 50, r: 40, stroke: "green", fill: "yellow" } }
        div { class: "raw-attribute-div", "raw-attribute": "raw-attribute-value" }
        div { class: "hidden-attribute-div", hidden: true }
        div {
            class: "dangerous-inner-html-div",
            dangerous_inner_html: "<p>hello dangerous inner html</p>"
        }
        input { value: "hello input" }
        div { class: "style-div", color: "red", "colored text" }
        OnMounted {}
    }
}

#[component]
fn OnMounted() -> Element {
    let mut mounted_triggered_count = use_signal(|| 0);
    rsx! {
        div {
            class: "onmounted-div",
            onmounted: move |_| {
                mounted_triggered_count += 1;
            },
            "onmounted was called {mounted_triggered_count} times"
        }
    }
}

#[tokio::main]
async fn main() {
    _ = dioxus::logger::init(Level::DEBUG);

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
