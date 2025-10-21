use tracing::{
    subscriber::{set_global_default, SetGlobalDefaultError},
    Level,
};

pub use tracing;

/// Attempt to initialize the subscriber if it doesn't already exist, with default settings.
///
/// See [`crate::init`] for more info.
///
/// If you're doing setup before your `dioxus::launch` function that requires lots of logging, then
/// it might be worth calling this earlier than launch.
///
/// `dioxus::launch` calls this for you automatically and won't replace any facade you've already set.
///
/// # Example
///
/// ```rust, ignore
/// use dioxus::prelude::*;
/// use tracing::info;
///
/// fn main() {
///     dioxus::logger::initialize_default();
///
///     info!("Doing some work before launching...");
///
///     dioxus::launch(App);
/// }
///
/// #[component]
/// fn App() -> Element {
///     info!("App rendered");
///     rsx! {
///         p { "hi" }
///     }
/// }
/// ```
pub fn initialize_default() {
    if tracing::dispatcher::has_been_set() {
        return;
    }

    if cfg!(debug_assertions) {
        _ = init(Level::DEBUG);
    } else {
        _ = init(Level::INFO);
    }
}

/// Initialize `dioxus-logger` with a specified max filter.
///
/// Generally it is best to initialize the logger before launching your Dioxus app.
/// Works on Web, Desktop, Fullstack, and Liveview.
///
/// # Example
///
/// ```rust, ignore
/// use dioxus::prelude::*;
/// use dioxus::logger::tracing::{Level, info};
///
/// fn main() {
///     dioxus::logger::init(Level::INFO).expect("logger failed to init");
///     dioxus::launch(App);
/// }
///
/// #[component]
/// fn App() -> Element {
///     info!("App rendered");
///     rsx! {
///         p { "hi" }
///     }
/// }
/// ```
pub fn init(level: Level) -> Result<(), SetGlobalDefaultError> {
    /*
    The default logger is currently set to log in fmt mode (meaning print directly to stdout)

    Eventually we want to change the output mode to be `json` when running under `dx`. This would let
    use re-format the tracing spans to be better integrated with `dx`
    */

    #[cfg(target_arch = "wasm32")]
    {
        use tracing_subscriber::layer::SubscriberExt;
        use tracing_subscriber::Registry;

        let layer_config = tracing_wasm::WASMLayerConfigBuilder::new()
            .set_max_level(level)
            .build();
        let layer = tracing_wasm::WASMLayer::new(layer_config);
        let reg = Registry::default().with(layer);

        set_global_default(reg)
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        let sub = tracing_subscriber::FmtSubscriber::builder()
            .with_max_level(level)
            .with_env_filter(
                tracing_subscriber::EnvFilter::builder()
                    .with_default_directive(level.into())
                    .from_env_lossy()
                    .add_directive("hyper_util=warn".parse().unwrap()), // hyper has `debug!` sitting around in some places that are spammy
            );

        if !dioxus_cli_config::is_cli_enabled() {
            return set_global_default(sub.finish());
        }

        // todo(jon): this is a small hack to clean up logging when running under the CLI
        // eventually we want to emit everything as json and let the CLI manage the parsing + display
        set_global_default(sub.without_time().with_target(false).finish())
    }
}
