#[cfg(not(feature = "axum"))]
fn main() {}

#[cfg(feature = "axum")]
#[tokio::main]
async fn main() {
    use axum::{extract::ws::WebSocketUpgrade, response::Html, routing::get, Router};
    use dioxus_core::{Element, LazyNodes, Scope};
    pretty_env_logger::init();

    fn app(cx: Scope) -> Element {
        cx.render(LazyNodes::new(|f| f.text(format_args!("hello world!"))))
    }

    let addr: std::net::SocketAddr = ([127, 0, 0, 1], 3030).into();

    let view = dioxus_liveview::new(addr);
    let body = view.body("<title>Dioxus Liveview</title>");

    let app = Router::new()
        .route("/", get(move || async { Html(body) }))
        .route(
            "/app",
            get(move |ws: WebSocketUpgrade| async move {
                ws.on_upgrade(move |socket| async move {
                    view.upgrade_axum(socket, app).await;
                })
            }),
        );
    axum::Server::bind(&addr.to_string().parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}
