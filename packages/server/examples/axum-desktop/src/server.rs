// Run with:
// ```bash
// cargo run --bin server --features="ssr"
// ```

use axum_desktop::*;
use dioxus_server::prelude::*;

#[tokio::main]
async fn main() {
    PostServerData::register().unwrap();
    GetServerData::register().unwrap();

    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], 8080));
    axum::Server::bind(&addr)
        .serve(
            axum::Router::new()
                .register_server_fns("")
                .into_make_service(),
        )
        .await
        .unwrap();
}
