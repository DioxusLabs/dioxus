use dioxus::prelude::*;
use dioxus_liveview::LiveView;
use std::net::SocketAddr;
use warp::ws::Ws;
use warp::Filter;

fn app(cx: Scope) -> Element {
    let mut num = use_state(cx, || 0);

    cx.render(rsx! {
        div {
            "hello world! {num}"
            button {
                onclick: move |_| num += 1,
                "Increment"
            }
        }
    })
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

    let view = LiveView::new();

    let ws = warp::path("ws")
        .and(warp::ws())
        .and(warp::any().map(move || view.clone()))
        .map(move |ws: Ws, view: LiveView| {
            println!("Got a connection!");
            ws.on_upgrade(|ws| view.upgrade_warp(ws, app))
        });

    println!("Listening on http://{}", addr);

    warp::serve(index.or(ws)).run(addr).await;
}
