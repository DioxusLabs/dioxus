#![allow(clippy::new_without_default)]
#![allow(unused)]
use dioxus_config_macro::*;
use dioxus_core::{Element, LaunchConfig};
use std::any::Any;

use crate::prelude::*;

/// Launch your Dioxus application with the given root component, context and config.
/// The platform will be determined from cargo features.
///
/// For a builder API, see `LaunchBuilder` defined in the `dioxus` crate.
///
/// # Feature selection
///
/// - `web`: Enables the web platform.
/// - `desktop`: Enables the desktop platform.
/// - `mobile`: Enables the mobile (ios + android webview) platform.
/// - `server`: Enables the server (axum + server-side-rendering) platform.
/// - `liveview`: Enables the liveview (websocke) platform.
/// - `native`: Enables the native (wgpu + winit renderer) platform.
///
/// Currently `native` is its own platform that is not compatible with desktop or mobile since it
/// unifies both platforms into one. If "desktop" and "native" are enabled, then the native renderer
/// will be used.
///
/// # Feature priority
///
/// If multiple renderers are enabled, the order of priority goes:
///
/// 1. liveview
/// 2. server
/// 3. native
/// 4. desktop
/// 5. mobile
/// 6. web
///
/// However, we don't recommend enabling multiple renderers at the same time due to feature conflicts
/// and bloating of the binary size.
///
/// # Example
/// ```rust, no_run
/// use dioxus::prelude::*;
///
/// fn main() {
///     dioxus::launch(app);
/// }
///
/// fn app() -> Element {
///     rsx! {
///         div { "Hello, world!" }
///     }
/// }
/// ```
pub fn launch(app: fn() -> Element) {
    #[allow(deprecated)]
    LaunchBuilder::new().launch(app)
}

/// A builder for a fullstack app.
#[must_use]
pub struct LaunchBuilder {
    platform: KnownPlatform,
    contexts: Vec<ContextFn>,
    configs: Vec<Box<dyn Any>>,
}

pub type LaunchFn = fn(fn() -> Element, Vec<ContextFn>, Vec<Box<dyn Any>>);

/// A context function is a Send and Sync closure that returns a boxed trait object
pub type ContextFn = Box<dyn Fn() -> Box<dyn Any> + Send + Sync + 'static>;

enum KnownPlatform {
    Web,
    Desktop,
    Mobile,
    Server,
    Liveview,
    Native,
    Other(LaunchFn),
}

#[allow(clippy::redundant_closure)] // clippy doesn't understand our coercion to fn
impl LaunchBuilder {
    /// Create a new builder for your application. This will create a launch configuration for the current platform based on the features enabled on the `dioxus` crate.
    // If you aren't using a third party renderer and this is not a docs.rs build, generate a warning about no renderer being enabled
    #[cfg_attr(
        all(not(any(
            docsrs,
            feature = "third-party-renderer",
            feature = "liveview",
            feature = "desktop",
            feature = "mobile",
            feature = "web",
            feature = "fullstack",
        ))),
        deprecated(
            note = "No renderer is enabled. You must enable a renderer feature on the dioxus crate before calling the launch function.\nAdd `web`, `desktop`, `mobile`, or `fullstack` to the `features` of dioxus field in your Cargo.toml.\n# Example\n```toml\n# ...\n[dependencies]\ndioxus = { version = \"0.5.0\", features = [\"web\"] }\n# ...\n```"
        )
    )]
    pub fn new() -> LaunchBuilder {
        let platform = if cfg!(feature = "native") {
            KnownPlatform::Native
        } else if cfg!(feature = "desktop") {
            KnownPlatform::Desktop
        } else if cfg!(feature = "mobile") {
            KnownPlatform::Mobile
        } else if cfg!(feature = "web") {
            KnownPlatform::Web
        } else if cfg!(feature = "server") {
            KnownPlatform::Server
        } else if cfg!(feature = "liveview") {
            KnownPlatform::Liveview
        } else {
            panic!("No platform feature enabled. Please enable one of the following features: liveview, desktop, mobile, web, tui, fullstack to use the launch API.")
        };

        LaunchBuilder {
            platform,
            contexts: Vec::new(),
            configs: Vec::new(),
        }
    }

    /// Launch your web application.
    #[cfg(feature = "web")]
    #[cfg_attr(docsrs, doc(cfg(feature = "web")))]
    pub fn web() -> LaunchBuilder {
        LaunchBuilder {
            platform: KnownPlatform::Web,
            contexts: Vec::new(),
            configs: Vec::new(),
        }
    }

    /// Launch your desktop application.
    #[cfg(feature = "desktop")]
    #[cfg_attr(docsrs, doc(cfg(feature = "desktop")))]
    pub fn desktop() -> LaunchBuilder {
        LaunchBuilder {
            platform: KnownPlatform::Desktop,
            contexts: Vec::new(),
            configs: Vec::new(),
        }
    }

    /// Launch your fullstack axum server.
    #[cfg(all(feature = "fullstack", feature = "server"))]
    #[cfg_attr(docsrs, doc(cfg(all(feature = "fullstack", feature = "server"))))]
    pub fn server() -> LaunchBuilder {
        LaunchBuilder {
            platform: KnownPlatform::Server,
            contexts: Vec::new(),
            configs: Vec::new(),
        }
    }

    /// Launch your fullstack application.
    #[cfg(feature = "mobile")]
    #[cfg_attr(docsrs, doc(cfg(feature = "mobile")))]
    pub fn mobile() -> LaunchBuilder {
        LaunchBuilder {
            platform: KnownPlatform::Mobile,
            contexts: Vec::new(),
            configs: Vec::new(),
        }
    }

    /// Provide a custom launch function for your application.
    ///
    /// Useful for third party renderers to tap into the launch builder API without having to reimplement it.
    ///
    /// # Example
    /// ```rust, no_run
    /// use dioxus::prelude::*;
    /// use std::any::Any;
    ///
    /// #[derive(Default)]
    /// struct Config;
    ///
    /// fn my_custom_launcher(root: fn() -> Element, contexts: Vec<Box<dyn Fn() -> Box<dyn Any> + Send + Sync>>, cfg: Vec<Box<dyn Any>>) {
    ///     println!("launching with root: {:?}", root());
    ///     loop {
    ///         println!("running...");
    ///     }
    /// }
    ///
    /// fn app() -> Element {
    ///     rsx! {
    ///         div { "Hello, world!" }
    ///     }
    /// }
    ///
    /// dioxus::LaunchBuilder::custom(my_custom_launcher).launch(app);
    /// ```
    pub fn custom(launch_fn: LaunchFn) -> LaunchBuilder {
        LaunchBuilder {
            platform: KnownPlatform::Other(launch_fn),
            contexts: vec![],
            configs: Vec::new(),
        }
    }

    /// Switch to an embedded-friendly desktop launcher.
    ///
    /// This is intended for applications that embed Dioxus Desktop inside an existing host process
    /// (for example, as a plugin/add-in). On Windows, `tao::EventLoop::run` will call
    /// `std::process::exit` when the event loop exits, which can terminate the host process.
    ///
    /// When the current platform is Desktop, this switches the launcher to
    /// `dioxus_desktop::launch::launch_return` which uses `run_return` so the event loop can exit
    /// without calling `std::process::exit`.
    #[cfg(feature = "desktop")]
    #[cfg_attr(docsrs, doc(cfg(feature = "desktop")))]
    pub fn embedded(mut self) -> Self {
        if matches!(self.platform, KnownPlatform::Desktop) {
            self.platform = KnownPlatform::Other(dioxus_desktop::launch::launch_return);
        }
        self
    }

    /// Inject state into the root component's context that is created on the thread that the app is launched on.
    ///
    /// # Example
    /// ```rust, no_run
    /// use dioxus::prelude::*;
    /// use std::any::Any;
    ///
    /// #[derive(Default)]
    /// struct MyState {
    ///     value: i32,
    /// }
    ///
    /// fn app() -> Element {
    ///     rsx! {
    ///         div { "Hello, world!" }
    ///     }
    /// }
    ///
    /// dioxus::LaunchBuilder::new()
    ///     .with_context_provider(|| Box::new(MyState { value: 42 }))
    ///     .launch(app);
    /// ```
    pub fn with_context_provider(
        mut self,
        state: impl Fn() -> Box<dyn Any> + Send + Sync + 'static,
    ) -> Self {
        self.contexts.push(Box::new(state));
        self
    }

    /// Inject state into the root component's context.
    ///
    /// # Example
    /// ```rust, no_run
    /// use dioxus::prelude::*;
    /// use std::any::Any;
    ///
    /// #[derive(Clone)]
    /// struct MyState {
    ///     value: i32,
    /// }
    ///
    /// fn app() -> Element {
    ///     rsx! {
    ///         div { "Hello, world!" }
    ///     }
    /// }
    ///
    /// dioxus::LaunchBuilder::new()
    ///     .with_context(MyState { value: 42 })
    ///     .launch(app);
    /// ```
    pub fn with_context(mut self, state: impl Any + Clone + Send + Sync + 'static) -> Self {
        self.contexts
            .push(Box::new(move || Box::new(state.clone())));
        self
    }

    /// Provide a platform-specific config to the builder.
    ///
    /// # Example
    /// ```rust, no_run
    /// use dioxus::prelude::*;
    /// use dioxus_desktop::{Config, WindowBuilder};
    ///
    /// fn app() -> Element {
    ///     rsx! {
    ///         div { "Hello, world!" }
    ///     }
    /// }
    ///
    /// dioxus::LaunchBuilder::new()
    ///    .with_cfg(desktop! {
    ///        Config::new().with_window(
    ///            WindowBuilder::new()
    ///                .with_title("My App")
    ///        )
    ///     })
    ///     .launch(app);
    /// ```
    pub fn with_cfg(mut self, config: impl LaunchConfig) -> Self {
        self.configs.push(Box::new(config));
        self
    }

    /// Launch your application.
    #[allow(clippy::diverging_sub_expression)]
    pub fn launch(self, app: fn() -> Element) {
        let Self {
            platform,
            contexts,
            configs,
        } = self;

        // Make sure to turn on the logger if the user specified the logger feaature
        #[cfg(feature = "logger")]
        dioxus_logger::initialize_default();

        // Set any flags if we're running under fullstack
        #[cfg(feature = "fullstack")]
        {
            use dioxus_fullstack::{get_server_url, set_server_url};

            // Make sure to set the server_fn endpoint if the user specified the fullstack feature
            // We only set this on native targets
            #[cfg(any(feature = "desktop", feature = "mobile", feature = "native"))]
            if get_server_url().is_empty() {
                let serverurl = format!(
                    "http://{}:{}",
                    std::env::var("DIOXUS_DEVSERVER_IP")
                        .unwrap_or_else(|_| "127.0.0.1".to_string()),
                    std::env::var("DIOXUS_DEVSERVER_PORT").unwrap_or_else(|_| "8080".to_string())
                )
                .leak();

                set_server_url(serverurl);
            }

            // If there is a base path set, call server functions from that base path
            #[cfg(feature = "web")]
            if let Some(base_path) = dioxus_cli_config::base_path() {
                let base_path = base_path.trim_matches('/');
                set_server_url(format!("{}/{}", get_server_url(), base_path).leak());
            }
        }

        // If native is specified, we override the webview launcher
        #[cfg(feature = "native")]
        if matches!(platform, KnownPlatform::Native) {
            return dioxus_native::launch_cfg(app, contexts, configs);
        }

        #[cfg(feature = "mobile")]
        if matches!(platform, KnownPlatform::Mobile) {
            return dioxus_desktop::launch::launch(app, contexts, configs);
        }

        #[cfg(feature = "desktop")]
        if matches!(platform, KnownPlatform::Desktop) {
            return dioxus_desktop::launch::launch(app, contexts, configs);
        }

        #[cfg(feature = "server")]
        if matches!(platform, KnownPlatform::Server) {
            return dioxus_server::launch_cfg(app, contexts, configs);
        }

        #[cfg(feature = "web")]
        if matches!(platform, KnownPlatform::Web) {
            return dioxus_web::launch::launch(app, contexts, configs);
        }

        #[cfg(feature = "liveview")]
        if matches!(platform, KnownPlatform::Liveview) {
            return dioxus_liveview::launch::launch(app, contexts, configs);
        }

        // If the platform is not one of the above, then we assume it's a custom platform
        if let KnownPlatform::Other(launch_fn) = platform {
            return launch_fn(app, contexts, configs);
        }

        // If we're here, then we have no platform feature enabled and third-party-renderer is enabled
        if cfg!(feature = "third-party-renderer") {
            panic!("No first party renderer feature enabled. It looks like you are trying to use a third party renderer. You will need to use the launch function from the third party renderer crate.");
        }

        panic!("No platform feature enabled. Please enable one of the following features: liveview, desktop, mobile, web, tui, fullstack to use the launch API.")
    }
}
