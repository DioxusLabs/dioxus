use std::env;
use tracing_subscriber::fmt::MakeWriter;
use tracing_subscriber::{prelude::*, EnvFilter, Layer};

pub fn build_tracing(writer: Option<Box<dyn MakeWriter<'static> + Send + Sync>>) {
    // If {LOG_ENV} is set, default to env, otherwise filter to cli
    // and manganis warnings and errors from other crates
    let mut filter = EnvFilter::new("error,dx=info,dioxus-cli=info,manganis-cli-support=info");
    if env::var(LOG_ENV).is_ok() {
        filter = EnvFilter::from_env(LOG_ENV);
    }

    let fmt_layer = tracing_subscriber::fmt::layer().with_filter(filter);
    let sub = tracing_subscriber::registry().with(fmt_layer);

    #[cfg(feature = "tokio-console")]
    sub.with(console_subscriber::spawn()).init();

    #[cfg(not(feature = "tokio-console"))]
    sub.init();
}
