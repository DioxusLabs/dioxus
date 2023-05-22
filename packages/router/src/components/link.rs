use std::fmt::Debug;

use dioxus::prelude::*;
use log::error;

use crate::navigation::NavigationTarget;
use crate::routable::Routable;
use crate::utils::use_router_internal::use_router_internal;

/// The properties for a [`Link`].
#[derive(Props)]
pub struct LinkProps<'a, R: Routable> {
    /// A class to apply to the generate HTML anchor tag if the `target` route is active.
    pub active_class: Option<&'a str>,
    /// The children to render within the generated HTML anchor tag.
    pub children: Element<'a>,
    /// The class attribute for the generated HTML anchor tag.
    ///
    /// If `active_class` is [`Some`] and the `target` route is active, `active_class` will be
    /// appended at the end of `class`.
    pub class: Option<&'a str>,
    /// The id attribute for the generated HTML anchor tag.
    pub id: Option<&'a str>,
    /// When [`true`], the `target` route will be opened in a new tab.
    ///
    /// This does not change whether the [`Link`] is active or not.
    #[props(default)]
    pub new_tab: bool,
    /// The onclick event handler.
    pub onclick: Option<EventHandler<'a, MouseEvent>>,
    #[props(default)]
    /// Whether the default behavior should be executed if an `onclick` handler is provided.
    ///
    /// 1. When `onclick` is [`None`] (default if not specified), `onclick_only` has no effect.
    /// 2. If `onclick_only` is [`false`] (default if not specified), the provided `onclick` handler
    ///    will be executed after the links regular functionality.
    /// 3. If `onclick_only` is [`true`], only the provided `onclick` handler will be executed.
    pub onclick_only: bool,
    /// The rel attribute for the generated HTML anchor tag.
    ///
    /// For external `target`s, this defaults to `noopener noreferrer`.
    pub rel: Option<&'a str>,
    /// The navigation target. Roughly equivalent to the href attribute of an HTML anchor tag.
    #[props(into)]
    pub target: NavigationTarget<R>,
}

impl<R: Routable> Debug for LinkProps<'_, R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LinkProps")
            .field("active_class", &self.active_class)
            .field("children", &self.children)
            .field("class", &self.class)
            .field("id", &self.id)
            .field("new_tab", &self.new_tab)
            .field("onclick", &self.onclick.as_ref().map(|_| "onclick is set"))
            .field("onclick_only", &self.onclick_only)
            .field("rel", &self.rel)
            .field("target", &self.target.to_string())
            .finish()
    }
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
pub fn Link<'a, R: Routable + Clone>(cx: Scope<'a, LinkProps<'a, R>>) -> Element {
    let LinkProps {
        active_class,
        children,
        class,
        id,
        new_tab,
        onclick,
        onclick_only,
        rel,
        target,
    } = cx.props;

    // hook up to router
    let router = match use_router_internal::<R>(cx) {
        Some(r) => r,
        #[allow(unreachable_code)]
        None => {
            let msg = "`Link` must have access to a parent router";
            error!("{msg}, will be inactive");
            #[cfg(debug_assertions)]
            panic!("{}", msg);
            return None;
        }
    };

    let current_route = router.current();
    let href = current_route.to_string();
    let ac = active_class
        .and_then(|active_class| match target {
            NavigationTarget::Internal(target) => {
                if href == target.to_string() {
                    Some(format!(" {active_class}"))
                } else {
                    None
                }
            }
            _ => None,
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

    let do_default = onclick.is_none() || !onclick_only;
    let action = move |event| {
        if do_default && is_router_nav {
            router.push(target.clone());
        }

        if let Some(handler) = onclick {
            handler.call(event);
        }
    };

    render! {
        a {
            onclick: action,
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
