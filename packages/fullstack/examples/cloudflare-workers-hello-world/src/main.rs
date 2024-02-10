//! Run with:
//!
//! ```sh
//! npm run serve
//! ```

use cloudflare_workers_hello_world::app;

#[cfg(feature = "web")]
fn main() {
    dioxus_web::launch::launch_cfg(
        app,
        dioxus_web::Config::default().hydrate(false),
    );
}

