use dioxus::prelude::*;
use dioxus_router_core::{Name, OutletData};
use log::error;

use crate::utils::use_router_internal::use_router_internal;

/// The properties for an [`Outlet`].
#[derive(Debug, Eq, PartialEq, Props)]
pub struct OutletProps {
    /// Override the [`Outlet`]s nesting depth.
    ///
    /// By default the [`Outlet`] will find its own depth. This property overrides that depth.
    /// Nested [`Outlet`]s will respect this override and calculate their depth based on it.
    pub depth: Option<usize>,
    /// The content name.
    ///
    /// By default, the outlet will render unnamed content. If this is set to a name, the outlet
    /// will render content for that name, defined via [`RouteContent::MultiContent`].
    ///
    /// [`RouteContent::MultiContent`]: dioxus_router_core::routes::RouteContent::MultiContent
    pub name: Option<Name>,
}

/// An outlet for the current content.
///
/// Only works as descendant of a component calling [`use_router`], otherwise it will be inactive.
///
/// The [`Outlet`] is aware of how many [`Outlet`]s it is nested within. It will render the content
/// of the active route that is __exactly as deep__.
///
/// [`use_router`]: crate::hooks::use_router
///
/// # Panic
/// - When the [`Outlet`] is not nested within another component calling the [`use_router`] hook,
///   but only in debug builds.
///
/// # Example
/// ```rust
/// # use dioxus::prelude::*;
/// # use dioxus_router::prelude::*;
/// fn App(cx: Scope) -> Element {
///     use_router(
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
///         Outlet { } // The content component will be rendered here
///     }
/// }
///
/// fn Content(cx: Scope) -> Element {
///     render! {
///         p { "Content" }
///     }
/// }
/// #
/// # let mut vdom = VirtualDom::new(App);
/// # let _ = vdom.rebuild();
/// # assert_eq!(dioxus_ssr::render(&vdom), "<h1>App</h1><p>Content</p>");
/// ```
#[allow(non_snake_case)]
pub fn Outlet(cx: Scope<OutletProps>) -> Element {
    let OutletProps { depth, name } = cx.props;

    // hook up to router
    let router = match use_router_internal(cx) {
        Some(r) => r,
        #[allow(unreachable_code)]
        None => {
            let msg = "`Outlet` must have access to a parent router";
            error!("{msg}, will be inactive");
            #[cfg(debug_assertions)]
            panic!("{}", msg);
            anyhow::bail!("{msg}");
        }
    };
    let state = loop {
        if let Some(state) = router.state.try_read() {
            break state;
        }
    };

    // do depth calculation and propagation
    let depth = cx.use_hook(|| {
        let mut context = cx.consume_context::<OutletData>().unwrap_or_default();
        let depth = depth
            .or_else(|| context.depth(name))
            .map(|d| d + 1)
            .unwrap_or_default();
        context.set_depth(name, depth);
        cx.provide_context(context);
        depth
    });

    // get content
    let content = match name {
        None => state.content.get(*depth),
        Some(n) => state.named_content.get(n).and_then(|n| n.get(*depth)),
    }
    .cloned();

    cx.render(match content {
        Some(content) => {
            let X = content.0;
            rsx!(X {})
        }
        None => rsx!(()),
    })
}
