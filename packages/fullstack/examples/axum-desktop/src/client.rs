// Run with:
// ```bash
// cargo run --bin client --features desktop
// ```

use axum_desktop::*;

fn main() {
    // Set the url of the server where server functions are hosted.
    #[cfg(not(feature = "server"))]
    dioxus::fullstack::prelude::server_fn::set_server_url("http://127.0.0.1:8080");

    #[cfg(feature = "desktop")]
    dioxus::prelude::launch_desktop(app)
}
