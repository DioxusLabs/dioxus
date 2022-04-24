#![cfg(feature = "warp")]

use dioxus_core::{Element, LazyNodes, Scope};
use dioxus_liveview as liveview;
use warp::ws::Ws;
use warp::Filter;

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    let addr = ([127, 0, 0, 1], 3030);

    // todo: compactify this routing under one liveview::app method
    let view = liveview::new(addr);
    let body = view.body("<title>Dioxus LiveView</title>");

    let routes = warp::path::end()
        .map(move || warp::reply::html(body.clone()))
        .or(warp::path("app")
            .and(warp::ws())
            .and(warp::any().map(move || view.clone()))
            .map(|ws: Ws, view: liveview::Liveview| {
                ws.on_upgrade(|socket| async move {
                    view.upgrade(socket, app).await;
                })
            }));

    warp::serve(routes).run(addr).await;
}

fn app(cx: Scope) -> Element {
    cx.render(LazyNodes::new(|f| f.text(format_args!("hello world!"))))
}
