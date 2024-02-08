use dioxus::prelude::*;
use dioxus_liveview::warp_adapter::warp_socket;
use dioxus_liveview::LiveViewPool;
use std::net::SocketAddr;
use warp::ws::Ws;
use warp::Filter;

fn app() -> Element {
    let mut num = use_signal(|| 0);

    rsx! {
        div {
            "hello warp! {num}"
            button {
                onclick: move |_| num += 1,
                "Increment"
            }
        }
    }
}

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    let addr: SocketAddr = ([127, 0, 0, 1], 3030).into();

    let index = warp::path::end().map(move || {
        warp::reply::html(format!(
            r#"
            <!DOCTYPE html>
            <html>
                <head> <title>Dioxus LiveView with Warp</title>  </head>
                <body> <div id="main"></div> </body>
                {glue}
            </html>
            "#,
            glue = dioxus_liveview::interpreter_glue(&format!("ws://{addr}/ws/"))
        ))
    });

    let pool = LiveViewPool::new();

    let ws = warp::path("ws")
        .and(warp::ws())
        .and(warp::any().map(move || pool.clone()))
        .map(move |ws: Ws, pool: LiveViewPool| {
            ws.on_upgrade(|ws| async move {
                let _ = pool.launch(warp_socket(ws), app).await;
            })
        });

    println!("Listening on http://{}", addr);

    warp::serve(index.or(ws)).run(addr).await;
}
