//! Setup with:
//!
//! ```sh
//! bun install
//! bun run postinstall
//! ```
//!
//! Run with:
//!
//! ```sh
//! bun run serve
//! ```

#[cfg(feature = "web")]
fn main() {
    use cloudflare_hello_world::app;
    use tracing_subscriber::prelude::*;
    use tracing_web::MakeWebConsoleWriter;

    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_ansi(true)
        .without_time()
        .with_level(false)
        .with_writer(MakeWebConsoleWriter::new().with_pretty_level());
    tracing_subscriber::registry().with(fmt_layer).init();

    dioxus_web::launch::launch_cfg(app, dioxus_web::Config::default().hydrate(true));
}

#[cfg(not(feature = "web"))]
fn main() {}
