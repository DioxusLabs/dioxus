//! Fullstack router intigration
#![allow(non_snake_case)]
use dioxus::prelude::*;

/// Used by the launch macro
#[doc(hidden)]
pub fn RouteWithCfg<R>(cx: Scope<FullstackRouterConfig<R>>) -> Element
where
    R: dioxus_router::prelude::Routable,
    <R as std::str::FromStr>::Err: std::fmt::Display,
{
    use dioxus_router::prelude::RouterConfig;

    #[cfg(feature = "ssr")]
    let context = crate::prelude::server_context();

    let cfg = *cx.props;
    render! {
        dioxus_router::prelude::Router::<R> {
            config: move || {
                RouterConfig::default()
                    .failure_external_navigation(cfg.failure_external_navigation)
                    .history({
                        #[cfg(feature = "ssr")]
                        let history = dioxus_router::prelude::MemoryHistory::with_initial_path(
                            context
                                .request_parts().unwrap()
                                .uri
                                .to_string()
                                .parse()
                                .unwrap_or_else(|err| {
                                    tracing::error!("Failed to parse uri: {}", err);
                                    "/"
                                        .parse()
                                        .unwrap_or_else(|err| {
                                            panic!("Failed to parse uri: {}", err);
                                        })
                                }),
                        );
                        #[cfg(not(feature = "ssr"))]
                        let history = dioxus_router::prelude::WebHistory::new(
                            None,
                            cfg.scroll_restoration,
                        );
                        history
                    })
            },
        }
    }
}

fn default_external_navigation_handler() -> fn(Scope) -> Element {
    dioxus_router::prelude::FailureExternalNavigation
}

/// The configeration for the router
#[derive(Props, serde::Serialize, serde::Deserialize)]
pub struct FullstackRouterConfig<R>
where
    R: dioxus_router::prelude::Routable,
    <R as std::str::FromStr>::Err: std::fmt::Display,
{
    #[serde(skip)]
    #[serde(default = "default_external_navigation_handler")]
    failure_external_navigation: fn(Scope) -> Element,
    scroll_restoration: bool,
    #[serde(skip)]
    phantom: std::marker::PhantomData<R>,
}

impl<R> Clone for FullstackRouterConfig<R>
where
    R: dioxus_router::prelude::Routable,
    <R as std::str::FromStr>::Err: std::fmt::Display,
{
    fn clone(&self) -> Self {
        *self
    }
}

impl<R> Copy for FullstackRouterConfig<R>
where
    R: dioxus_router::prelude::Routable,
    <R as std::str::FromStr>::Err: std::fmt::Display,
{
}

impl<R> Default for FullstackRouterConfig<R>
where
    R: dioxus_router::prelude::Routable,
    <R as std::str::FromStr>::Err: std::fmt::Display,
{
    fn default() -> Self {
        Self {
            failure_external_navigation: dioxus_router::prelude::FailureExternalNavigation,
            scroll_restoration: true,
            phantom: std::marker::PhantomData,
        }
    }
}
