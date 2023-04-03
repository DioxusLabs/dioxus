// Run with:
// ```bash
// cargo run --bin client --features="desktop"
// ```

use axum_desktop::*;
use dioxus_server::prelude::server_fn::set_server_url;

fn main() {
    // Set the url of the server where server functions are hosted.
    set_server_url("http://localhost:8080");
    dioxus_desktop::launch(app)
}
