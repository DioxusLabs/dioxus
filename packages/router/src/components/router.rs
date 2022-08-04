use std::{fmt::Debug, sync::Arc};

use dioxus::prelude::*;
use log::error;

use crate::{
    contexts::RouterContext,
    history::HistoryProvider,
    route_definition::{RouteContent, Segment},
    service::RouterService,
};

/// The props for a [`Router`].
#[derive(Props)]
pub struct RouterProps<'a> {
    /// A class to apply to active [`Link`]s.
    ///
    /// Individual [`Link`]s can override this via a prop with the same name.
    ///
    /// [`Link`]: crate::components::Link
    pub active_class: Option<&'a str>,
    /// The components to render where the [`Router`] itself is.
    ///
    /// Usually contains at least one [`Outlet`](crate::components::Outlet).
    pub children: Element<'a>,
    /// Fallback content.
    ///
    /// The router will use this content when no other content is found. It can be used to implement
    /// a 404 page.
    #[props(default)]
    pub fallback: RouteContent,
    /// A function that constructs a history provider.
    ///
    /// When [`None`], a default is used:
    /// - [`WebHistory`](crate::history::WebHistory) when the `web` feature is enabled and the
    ///   target family is `wasm`.
    /// - Otherwise [`MemoryHistory`](crate::history::MemoryHistory).
    pub history: Option<&'a dyn Fn() -> Box<dyn HistoryProvider>>,
    /// When [`true`], the router will route __only once__.
    ///
    /// Useful for server-side rendering, as the router will not rely on an async task.
    #[props(default)]
    pub init_only: bool,
    /// The routes of the application.
    pub routes: Arc<Segment>,
}

// [`Fn() -> Box<dyn HistoryProvider>`] (in `history`) doesn't implement [`Debug`]
impl Debug for RouterProps<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RouterProps")
            .field("active_class", &self.active_class)
            .field("children", &self.children)
            .field("fallback", &self.fallback)
            .field("history", &self.history.is_some())
            .field("init_only", &self.init_only)
            .field("routes", &self.routes)
            .finish()
    }
}

/// The base component that provides core functionality for the rest of the router.
///
/// All other components and hooks the router provides can only work as descendants of a [`Router`]
/// component.
///
/// The [`Router`] component cannot be nested within itself. Inner instances will be inactive and
/// ignored.
///
/// # Panic
/// - When nested within itself, but only in debug builds.
///
/// # Example
/// ```rust,no_run
/// # use dioxus::prelude::*;
/// # use dioxus_router::prelude::*;
/// fn App(cx: Scope) -> Element {
///     let routes = use_segment(&cx, Segment::new);
///
///     cx.render(rsx! {
///         Router {
///             routes: routes.clone(),
///             // other props
///
///             // content, at least one
///             Outlet { }
///         }
///     })
/// }
/// ```
#[allow(non_snake_case)]
pub fn Router<'a>(cx: Scope<'a, RouterProps<'a>>) -> Element {
    let RouterProps {
        active_class,
        children,
        fallback,
        history,
        init_only,
        routes,
    } = cx.props;

    let service = cx.use_hook(|| {
        // make sure no router context exists
        if cx.consume_context::<RouterContext>().is_some() {
            error!("`Router` can not be used as a descendent of a `Router`, inner will be ignored");
            #[cfg(debug_assertions)]
            panic!("`Router` can not be used as a descendent of a `Router`");
            #[cfg(not(debug_assertions))]
            return;
        };

        // create custom history provider
        let history = history.map(|x| x());

        // create router service and inject context
        let (mut service, context) = RouterService::new(
            routes.clone(),
            cx.schedule_update_any(),
            active_class.map(|ac| ac.to_string()),
            fallback.clone(),
            history,
        );
        cx.provide_context(context);

        match init_only {
            true => return Some(service),
            false => {
                // run service
                cx.spawn(async move { service.run().await });
                None
            }
        }
    });

    // update routing when `init_only`
    if let Some(service) = service {
        service.single_routing();
    }

    cx.render(rsx!(children))
}
