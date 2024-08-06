// Run with:
// ```bash
// cargo run --bin server --features server
// ```

use dioxus::prelude::*;
use fullstack_desktop_example::*;
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
            .register_server_functions()
            .into_make_service(),
    )
    .await
    .unwrap();
}
