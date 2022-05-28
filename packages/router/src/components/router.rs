use std::sync::Arc;

use dioxus_core::{self as dioxus, prelude::*};
use dioxus_core_macro::*;
use dioxus_html as dioxus_elements;
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
    /// - [`BrowserPathHistoryProvider`] when the `web` feature is enabled
    /// - [`MemoryHistoryProvider`] when it isn't
    ///
    /// [`BrowserPathHistoryProvider`]: crate::history::BrowserPathHistoryProvider
    /// [`MemoryHistoryProvider`]: crate::history::MemoryHistoryProvider
    pub history: Option<&'a dyn Fn() -> Box<dyn HistoryProvider>>,
    /// When [`true`], the router will route __only once__.
    ///
    /// Useful for server-side rendering, as the router will not rely on an async task.
    #[props(default)]
    pub init_only: bool,
    /// The routes of the application.
    pub routes: Arc<Segment>,
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

    cx.use_hook(|_| {
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

        // run service
        match init_only {
            true => service.single_routing(),
            false => cx.spawn(async move { service.run().await }),
        }
    });

    cx.render(rsx!(children))
}
