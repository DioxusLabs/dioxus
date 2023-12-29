// Run with:
// ```bash
// cargo run --bin server --features ssr
// ```

use axum_desktop::*;
use dioxus_fullstack::prelude::*;

#[tokio::main]
async fn main() {
    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], 8080));

    let _ = PostServerData::register_explicit();
    let _ = GetServerData::register_explicit();

    let app = axum::Router::new()
        .register_server_fns("")
        .into_make_service();

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
