//! Run with:
//!
//! ```sh
//! npm run serve
//! ```

use tracing_web::MakeWebConsoleWriter;
use cloudflare_workers_hello_world::app;
use tracing_subscriber::prelude::*;

#[cfg(feature = "web")]
fn main() {
    // let fmt_layer = tracing_subscriber::fmt::layer()
    //     .with_ansi(true)
    //     .without_time()
    //     .with_writer(MakeWebConsoleWriter::new().with_pretty_level());
    // tracing_subscriber::registry()
    //     .with(fmt_layer)
    //     .init();
    tracing_wasm::set_as_global_default();

    tracing::info!("Starting web");

    dioxus_web::launch::launch_cfg(
        app,
        dioxus_web::Config::default().hydrate(false),
    );
}

