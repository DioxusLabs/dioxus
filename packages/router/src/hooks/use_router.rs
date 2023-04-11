use async_lock::RwLockReadGuard;
use dioxus::{core::Component, prelude::*};
use dioxus_router_core::{
    history::{HistoryProvider, MemoryHistory},
    routes::{ContentAtom, Segment},
    Navigator, RouterService, RouterState, RoutingCallback,
};
use log::error;

use crate::{
    contexts::router::RouterContext,
    prelude::{
        comp,
        default_errors::{
            FailureExternalNavigation, FailureNamedNavigation, FailureRedirectionLimit,
        },
    },
};

/// The basic building block required for all other router components and hooks.
///
/// This manages a [`dioxus_router_core::RouterService`], which in turn is required for basically
/// all router functionality. All other components and hooks provided by [`dioxus_router`](crate)
/// will only work as/in components nested within a component calling [`use_router`].
///
/// Components calling [`use_router`] should not be nested within each other.
///
/// # Return values
/// This hook returns the current router state and a navigator. For more information about the
/// state, see the [`use_route`](crate::hooks::use_route) hook. For more information about the
/// [`Navigator`], see its own documentation and the [`use_navigate`](crate::hooks::use_navigate)
/// hook.
///
/// # Panic
/// - When used within a component, that is nested inside another component calling [`use_router`],
///   but only in debug builds.
///
/// # Example
/// ```rust
/// # use dioxus::prelude::*;
/// # use dioxus_router::prelude::*;
/// fn App(cx: Scope) -> Element {
///     let (_, _) = use_router(
///         &cx,
///         &|| RouterConfiguration {
///             synchronous: true, // asynchronicity not needed for doc test
///             ..Default::default()
///         },
///         &|| Segment::content(comp(Content))
///     );
///
///     render! {
///         h1 { "App" }
///         Outlet { }
///     }
/// }
///
/// fn Content(cx: Scope) -> Element {
///     render! {
///         p { "Some content" }
///     }
/// }
/// # let mut vdom = VirtualDom::new(App);
/// # let _ = vdom.rebuild();
/// # assert_eq!(dioxus_ssr::render(&vdom), "<h1>App</h1><p>Some content</p>");
/// ```
pub fn use_router<'a>(
    cx: &'a ScopeState,
    cfg: &dyn Fn() -> RouterConfiguration,
    content: &dyn Fn() -> Segment<Component>,
) -> (
    RwLockReadGuard<'a, RouterState<Component>>,
    Navigator<ScopeId>,
) {
    let (service, state, sender) = cx.use_hook(|| {
        #[allow(unreachable_code, unused_variables)]
        if let Some(outer) = cx.consume_context::<RouterContext>() {
            let msg = "components using `use_router` should not be nested within each other";
            error!("{msg}, inner will be inactive and transparent");
            #[cfg(debug_assertions)]
            panic!("{}", msg);
            return (None, outer.state, outer.sender);
        }

        let cfg = cfg();
        let content = content();

        let (mut service, sender, state) = RouterService::new(
            content,
            cfg.history,
            cx.schedule_update_any(),
            cfg.on_update,
            cfg.failure_external_navigation,
            cfg.failure_named_navigation,
            cfg.failure_redirection_limit,
        );

        cx.provide_context(RouterContext {
            state: state.clone(),
            sender: sender.clone(),
        });

        (
            if cfg.synchronous {
                service.init();
                Some(service)
            } else {
                cx.spawn(async move { service.run().await });
                None
            },
            state,
            sender,
        )
    });

    if let Some(service) = service {
        service.run_current();
    }

    (
        loop {
            if let Some(state) = state.try_read() {
                break state;
            }
        },
        sender.clone().into(),
    )
}

/// Global configuration options for the router.
///
/// This implements [`Default`], so you can use it like this:
/// ```rust,no_run
/// # use dioxus_router::prelude::RouterConfiguration;
/// let cfg = RouterConfiguration {
///     synchronous: false,
///     ..Default::default()
/// };
/// ```
pub struct RouterConfiguration {
    /// A component to render when an external navigation fails.
    ///
    /// Defaults to a router-internal component called `FailureExternalNavigation`. It is not part
    /// of the public API. Do not confuse it with
    /// [`dioxus_router_core::prelude::FailureExternalNavigation`].
    pub failure_external_navigation: ContentAtom<Component>,
    /// A component to render when a named navigation fails.
    ///
    /// Defaults to a router-internal component called `FailureNamedNavigation`. It is not part of
    /// the public API. Do not confuse it with
    /// [`dioxus_router_core::prelude::FailureNamedNavigation`].
    pub failure_named_navigation: ContentAtom<Component>,
    /// A component to render when the redirect limit is reached.
    ///
    /// Defaults to a router-internal component called `FailureRedirectionLimit`. It is not part of
    /// the public API. Do not confuse it with
    /// [`dioxus_router_core::prelude::FailureRedirectionLimit`].
    pub failure_redirection_limit: ContentAtom<Component>,
    /// The [`HistoryProvider`] the router should use.
    ///
    /// Defaults to a default [`MemoryHistory`].
    pub history: Box<dyn HistoryProvider>,
    /// A function to be called whenever the routing is updated.
    ///
    /// The callback is invoked after the routing is updated, but before components and hooks are
    /// updated.
    ///
    /// If the callback returns a [`NavigationTarget`] the router will replace the current location
    /// with it. If no navigation failure was triggered, the router will then updated dependent
    /// components and hooks.
    ///
    /// The callback is called no more than once per rerouting. It will not be called if a
    /// navigation failure occurs.
    ///
    /// Defaults to [`None`].
    ///
    /// [`NavigationTarget`]: dioxus_router_core::navigation::NavigationTarget
    pub on_update: Option<RoutingCallback<Component>>,
    /// Whether the router should run in synchronous mode.
    ///
    /// If [`true`], the router will only update its state whenever the component using the
    /// [`use_router`] hook rerenders. If [`false`], an asynchronous task is launched and the router
    /// will update whenever it receives new input.
    ///
    /// Defaults to [`false`].
    pub synchronous: bool,
}

impl Default for RouterConfiguration {
    fn default() -> Self {
        Self {
            failure_external_navigation: comp(FailureExternalNavigation),
            failure_named_navigation: comp(FailureNamedNavigation),
            failure_redirection_limit: comp(FailureRedirectionLimit),
            history: Box::<MemoryHistory>::default(),
            on_update: None,
            synchronous: false,
        }
    }
}
