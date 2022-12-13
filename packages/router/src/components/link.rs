use dioxus::prelude::*;
use dioxus_router_core::{navigation::NavigationTarget, RouterMessage};
use log::error;

use crate::utils::use_router_internal::use_router_internal;

/// The properties for a [`Link`].
#[derive(Debug, Props)]
pub struct LinkProps<'a> {
    /// A class to apply to the generate HTML anchor tag if the `target` route is active.
    pub active_class: Option<&'a str>,
    /// The children to render within the generated HTML anchor tag.
    pub children: Element<'a>,
    /// The class attribute for the generated HTML anchor tag.
    ///
    /// If `active_class` is [`Some`] and the `target` route is active, `active_class` will be
    /// appended at the end of `class`.
    pub class: Option<&'a str>,
    /// Require the __exact__ `target` to be active, for the [`Link`] to be active.
    ///
    /// See [`RouterState::is_at`](dioxus_router_core::RouterState::is_at) for more details.
    #[props(default)]
    pub exact: bool,
    /// The id attribute for the generated HTML anchor tag.
    pub id: Option<&'a str>,
    /// When [`true`], the `target` route will be opened in a new tab.
    ///
    /// This does not change whether the [`Link`] is active or not.
    #[props(default)]
    pub new_tab: bool,
    /// The rel attribute for the generated HTML anchor tag.
    ///
    /// For external `target`s, this defaults to `noopener noreferrer`.
    pub rel: Option<&'a str>,
    /// The navigation target. Roughly equivalent to the href attribute of an HTML anchor tag.
    #[props(into)]
    pub target: NavigationTarget,
}

/// A link to navigate to another route.
///
/// Only works as descendant of a component calling [`use_router`], otherwise it will be inactive.
///
/// Unlike a regular HTML anchor, a [`Link`] allows the router to handle the navigation and doesn't
/// cause the browser to load a new page.
///
/// However, in the background a [`Link`] still generates an anchor, which you can use for styling
/// as normal.
///
/// [`use_router`]: crate::hooks::use_router
///
/// # External targets
/// When the [`Link`]s target is an [`External`] target, that is used as the `href` directly. This
/// means that a [`Link`] can always navigate to an [`External`] target.
///
/// This is different from a [`Navigator`], which can only navigate to external targets when the
/// routers [`HistoryProvider`] supports it.
///
/// [`External`]: dioxus_router_core::navigation::NavigationTarget::External
/// [`HistoryProvider`]: dioxus_router_core::history::HistoryProvider
/// [`Navigator`]: dioxus_router_core::Navigator
///
/// # Panic
/// - When the [`Link`] is not nested within another component calling the [`use_router`] hook, but
///   only in debug builds.
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
///         &|| Segment::empty()
///     );
///
///     render! {
///         Link {
///             active_class: "active",
///             class: "link_class",
///             exact: true,
///             id: "link_id",
///             new_tab: true,
///             rel: "link_rel",
///             target: "/",
///
///             "A fully configured link"
///         }
///     }
/// }
/// #
/// # let mut vdom = VirtualDom::new(App);
/// # let _ = vdom.rebuild();
/// # assert_eq!(
/// #     dioxus_ssr::render(&vdom),
/// #     r#"<a href="/" dioxus-prevent-default="" class="link_class active" id="link_id" rel="link_rel" target="_blank">A fully configured link</a>"#
/// # );
/// ```
#[allow(non_snake_case)]
pub fn Link<'a>(cx: Scope<'a, LinkProps<'a>>) -> Element {
    let LinkProps {
        active_class,
        children,
        class,
        exact,
        id,
        new_tab,
        rel,
        target,
    } = cx.props;

    // hook up to router
    let router = match use_router_internal(&cx) {
        Some(r) => r,
        #[allow(unreachable_code)]
        None => {
            let msg = "`Link` must have access to a parent router";
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
    let sender = router.sender.clone();

    let href = state.href(target);
    let ac = active_class
        .and_then(|active_class| {
            state
                .is_at(target, *exact)
                .then(|| format!(" {active_class}"))
        })
        .unwrap_or_default();

    let id = id.unwrap_or_default();
    let class = format!("{}{ac}", class.unwrap_or_default());
    let tag_target = new_tab.then_some("_blank").unwrap_or_default();

    let is_external = matches!(target, NavigationTarget::External(_));
    let is_router_nav = !is_external && !new_tab;
    let prevent_default = is_router_nav.then_some("onclick").unwrap_or_default();
    let rel = rel
        .or_else(|| is_external.then_some("noopener noreferrer"))
        .unwrap_or_default();

    render! {
        a {
            onclick: move |_| {
                if is_router_nav {
                    let _ = sender.unbounded_send(RouterMessage::Push(target.clone()));
                }
            },
            href: "{href}",
            prevent_default: "{prevent_default}",
            class: "{class}",
            id: "{id}",
            rel: "{rel}",
            target: "{tag_target}",
            children
        }
    }
}
