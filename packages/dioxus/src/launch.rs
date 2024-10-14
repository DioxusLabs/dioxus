//! Launch helper macros for fullstack apps
#![allow(clippy::new_without_default)]
#![allow(unused)]
use dioxus_config_macro::*;
use dioxus_core::LaunchConfig;
use std::any::Any;

use crate::prelude::*;

/// A builder for a fullstack app.
#[must_use]
pub struct LaunchBuilder {
    launch_fn: LaunchFn,
    contexts: Vec<ContextFn>,
    configs: Vec<Box<dyn Any>>,
}

pub type LaunchFn = fn(fn() -> Element, Vec<ContextFn>, Vec<Box<dyn Any>>);
/// A context function is a Send and Sync closure that returns a boxed trait object
pub type ContextFn = Box<dyn Fn() -> Box<dyn Any> + Send + Sync + 'static>;

#[cfg(any(
    feature = "fullstack",
    feature = "static-generation",
    feature = "liveview"
))]
type ValidContext = SendContext;

#[cfg(not(any(
    feature = "fullstack",
    feature = "static-generation",
    feature = "liveview"
)))]
type ValidContext = UnsendContext;

type SendContext = dyn Fn() -> Box<dyn Any + Send + Sync> + Send + Sync + 'static;

type UnsendContext = dyn Fn() -> Box<dyn Any> + 'static;

#[allow(clippy::redundant_closure)] // clippy doesnt doesn't understand our coercion to fn
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
            feature = "static-generation"
        ))),
        deprecated(
            note = "No renderer is enabled. You must enable a renderer feature on the dioxus crate before calling the launch function.\nAdd `web`, `desktop`, `mobile`, `fullstack`, or `static-generation` to the `features` of dioxus field in your Cargo.toml.\n# Example\n```toml\n# ...\n[dependencies]\ndioxus = { version = \"0.5.0\", features = [\"web\"] }\n# ...\n```"
        )
    )]
    pub fn new() -> LaunchBuilder {
        LaunchBuilder {
            // We can't use the `current_platform::launch` function directly because it may return ! or ()
            launch_fn: |root, contexts, cfg| current_platform::launch(root, contexts, cfg),
            contexts: Vec::new(),
            configs: Vec::new(),
        }
    }

    /// Launch your web application.
    #[cfg(feature = "web")]
    #[cfg_attr(docsrs, doc(cfg(feature = "web")))]
    pub fn web() -> LaunchBuilder {
        LaunchBuilder {
            launch_fn: web_launch,
            contexts: Vec::new(),
            configs: Vec::new(),
        }
    }

    /// Launch your desktop application.
    #[cfg(feature = "desktop")]
    #[cfg_attr(docsrs, doc(cfg(feature = "desktop")))]
    pub fn desktop() -> LaunchBuilder {
        LaunchBuilder {
            launch_fn: |root, contexts, cfg| dioxus_desktop::launch::launch(root, contexts, cfg),
            contexts: Vec::new(),
            configs: Vec::new(),
        }
    }

    /// Launch your fullstack axum server.
    #[cfg(all(feature = "fullstack", feature = "server"))]
    #[cfg_attr(docsrs, doc(cfg(all(feature = "fullstack", feature = "server"))))]
    pub fn server() -> LaunchBuilder {
        LaunchBuilder {
            launch_fn: |root, contexts, cfg| {
                dioxus_fullstack::server::launch::launch(root, contexts, cfg)
            },
            contexts: Vec::new(),
            configs: Vec::new(),
        }
    }

    /// Launch your static site generation application.
    #[cfg(all(feature = "static-generation", feature = "server"))]
    #[cfg_attr(
        docsrs,
        doc(cfg(all(feature = "static-generation", feature = "server")))
    )]
    pub fn static_generation() -> LaunchBuilder {
        LaunchBuilder {
            launch_fn: |root, contexts, cfg| {
                dioxus_static_site_generation::launch::launch(root, contexts, cfg)
            },
            contexts: Vec::new(),
            configs: Vec::new(),
        }
    }

    /// Launch your fullstack application.
    #[cfg(feature = "mobile")]
    #[cfg_attr(docsrs, doc(cfg(feature = "mobile")))]
    pub fn mobile() -> LaunchBuilder {
        LaunchBuilder {
            launch_fn: |root, contexts, cfg| dioxus_mobile::launch::launch(root, contexts, cfg),
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
            launch_fn,
            contexts: vec![],
            configs: Vec::new(),
        }
    }
}

impl LaunchBuilder {
    /// Inject state into the root component's context that is created on the thread that the app is launched on.
    pub fn with_context_provider(
        mut self,
        state: impl Fn() -> Box<dyn Any> + Send + Sync + 'static,
    ) -> Self {
        self.contexts.push(Box::new(state));
        self
    }

    /// Inject state into the root component's context.
    pub fn with_context(mut self, state: impl Any + Clone + Send + Sync + 'static) -> Self {
        self.contexts
            .push(Box::new(move || Box::new(state.clone())));
        self
    }
}

impl LaunchBuilder {
    /// Provide a platform-specific config to the builder.
    pub fn with_cfg(mut self, config: impl LaunchConfig) -> Self {
        self.configs.push(Box::new(config));
        self
    }

    // Static generation is the only platform that may exit. We can't use the `!` type here
    #[cfg(any(feature = "static-generation", feature = "web"))]
    /// Launch your application.
    pub fn launch(self, app: fn() -> Element) {
        let cfg = self.configs;

        (self.launch_fn)(app, self.contexts, cfg)
    }

    #[cfg(not(any(feature = "static-generation", feature = "web")))]
    /// Launch your application.
    pub fn launch(self, app: fn() -> Element) -> ! {
        let cfg = self.configs;

        (self.launch_fn)(app, self.contexts, cfg);
        unreachable!("Launching an application will never exit")
    }
}

/// Re-export the platform we expect the user wants
///
/// If multiple platforms are enabled, we use this priority (from highest to lowest):
/// - `fullstack`
/// - `desktop`
/// - `mobile`
/// - `static-generation`
/// - `web`
/// - `liveview`
mod current_platform {
    #[cfg(all(feature = "fullstack", feature = "server"))]
    pub use dioxus_fullstack::server::launch::*;

    #[cfg(all(
        feature = "desktop",
        not(all(feature = "fullstack", feature = "server"))
    ))]
    pub use dioxus_desktop::launch::*;

    #[cfg(all(
        feature = "mobile",
        not(feature = "desktop"),
        not(all(feature = "fullstack", feature = "server"))
    ))]
    pub use dioxus_mobile::launch::*;

    #[cfg(all(
        all(feature = "static-generation", feature = "server"),
        not(all(feature = "fullstack", feature = "server")),
        not(feature = "desktop"),
        not(feature = "mobile")
    ))]
    pub use dioxus_static_site_generation::launch::*;

    #[cfg(all(
        feature = "web",
        not(all(feature = "fullstack", feature = "server")),
        not(all(feature = "static-generation", feature = "server")),
        not(feature = "desktop"),
        not(feature = "mobile"),
    ))]
    pub fn launch(
        root: fn() -> dioxus_core::Element,
        contexts: Vec<super::ContextFn>,
        platform_config: Vec<Box<dyn std::any::Any>>,
    ) {
        super::web_launch(root, contexts, platform_config);
    }

    #[cfg(all(
        feature = "liveview",
        not(all(feature = "fullstack", feature = "server")),
        not(all(feature = "static-generation", feature = "server")),
        not(feature = "desktop"),
        not(feature = "mobile"),
        not(feature = "web"),
    ))]
    pub use dioxus_liveview::launch::*;

    #[cfg(not(any(
        feature = "liveview",
        all(feature = "fullstack", feature = "server"),
        all(feature = "static-generation", feature = "server"),
        feature = "desktop",
        feature = "mobile",
        feature = "web",
    )))]
    pub fn launch(
        root: fn() -> dioxus_core::Element,
        contexts: Vec<super::ContextFn>,
        platform_config: Vec<Box<dyn std::any::Any>>,
    ) -> ! {
        #[cfg(feature = "third-party-renderer")]
        panic!("No first party renderer feature enabled. It looks like you are trying to use a third party renderer. You will need to use the launch function from the third party renderer crate.");

        panic!("No platform feature enabled. Please enable one of the following features: liveview, desktop, mobile, web, tui, fullstack to use the launch API.")
    }
}

// ! is unstable, so we can't name the type with an alias. Instead we need to generate different variants of items with macros
macro_rules! impl_launch {
    ($($return_type:tt),*) => {
        /// Launch your application without any additional configuration. See [`LaunchBuilder`] for more options.
        pub fn launch(app: fn() -> Element) -> $($return_type)* {
            #[allow(deprecated)]
            LaunchBuilder::new().launch(app)
        }
    };
}

// Static generation is the only platform that may exit. We can't use the `!` type here
#[cfg(any(feature = "static-generation", feature = "web"))]
impl_launch!(());
#[cfg(not(any(feature = "static-generation", feature = "web")))]
impl_launch!(!);

#[cfg(feature = "web")]
fn web_launch(
    root: fn() -> dioxus_core::Element,
    contexts: Vec<super::ContextFn>,
    platform_config: Vec<Box<dyn std::any::Any>>,
) {
    // If the server feature is enabled, launch the client with hydration enabled
    #[cfg(any(feature = "static-generation", feature = "fullstack"))]
    {
        let platform_config = platform_config
            .into_iter()
            .find_map(|cfg| cfg.downcast::<dioxus_web::Config>().ok())
            .unwrap_or_default()
            .hydrate(true);

        let factory = move || {
            let mut vdom = dioxus_core::VirtualDom::new(root);
            for context in contexts {
                vdom.insert_any_root_context(context());
            }
            #[cfg(feature = "document")]
            {
                #[cfg(feature = "fullstack")]
                use dioxus_fullstack::document;
                #[cfg(all(feature = "static-generation", not(feature = "fullstack")))]
                use dioxus_static_site_generation::document;
                let document = std::rc::Rc::new(document::web::FullstackWebDocument)
                    as std::rc::Rc<dyn crate::prelude::document::Document>;
                vdom.provide_root_context(document);
            }
            vdom
        };

        dioxus_web::launch::launch_virtual_dom(factory(), platform_config)
    }
    #[cfg(not(any(feature = "static-generation", feature = "fullstack")))]
    dioxus_web::launch::launch(root, contexts, platform_config);
}
