#[macro_use]
extern crate rocket;

use dioxus::prelude::*;
use dioxus_liveview::LiveViewPool;
use rocket::response::content::RawHtml;
use rocket::{Config, Rocket, State};
use rocket_ws::{Channel, WebSocket};

fn app(cx: Scope) -> Element {
    let mut num = use_state(cx, || 0);

    cx.render(rsx! {
        div {
            "hello Rocket! {num}"
            button { onclick: move |_| num += 1, "Increment" }
        }
    })
}

fn index_page_with_glue(glue: &str) -> RawHtml<String> {
    RawHtml(format!(
        r#"
        <!DOCTYPE html>
        <html>
            <head> <title>Dioxus LiveView with Rocket</title>  </head>
            <body> <div id="main"></div> </body>
            {glue}
        </html>
        "#,
        glue = glue
    ))
}

#[get("/")]
async fn index(config: &Config) -> RawHtml<String> {
    index_page_with_glue(&dioxus_liveview::interpreter_glue(&format!(
        "ws://{addr}:{port}/ws",
        addr = config.address,
        port = config.port,
    )))
}

#[get("/as-path")]
async fn as_path() -> RawHtml<String> {
    index_page_with_glue(&dioxus_liveview::interpreter_glue("/ws"))
}

#[get("/ws")]
fn ws(ws: WebSocket, pool: &State<LiveViewPool>) -> Channel<'static> {
    let pool = pool.inner().to_owned();

    ws.channel(move |stream| {
        Box::pin(async move {
            let _ = pool
                .launch(dioxus_liveview::rocket_socket(stream), app)
                .await;
            Ok(())
        })
    })
}

#[tokio::main]
async fn main() {
    let view = dioxus_liveview::LiveViewPool::new();

    Rocket::build()
        .manage(view)
        .mount("/", routes![index, as_path, ws])
        .ignite()
        .await
        .expect("Failed to ignite rocket")
        .launch()
        .await
        .expect("Failed to launch rocket");
}
