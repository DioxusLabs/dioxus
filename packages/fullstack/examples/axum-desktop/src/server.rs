// Run with:
// ```bash
// cargo run --bin server --features server
// ```

use axum_desktop::*;
use dioxus::prelude::*;
use server_fn::axum::register_explicit;

#[tokio::main]
async fn main() {
    let listener = tokio::net::TcpListener::bind("127.0.0.01:8080")
        .await
        .unwrap();

    register_explicit::<PostServerData>();
    register_explicit::<GetServerData>();

    axum::serve(
        listener,
        axum::Router::new()
            .register_server_fns()
            .into_make_service(),
    )
    .await
    .unwrap();
}
