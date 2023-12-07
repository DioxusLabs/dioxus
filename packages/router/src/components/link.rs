#![allow(clippy::type_complexity)]

use std::any::Any;
use std::fmt::Debug;
use std::rc::Rc;

use dioxus::prelude::*;
use tracing::error;

use crate::navigation::NavigationTarget;
use crate::prelude::Routable;
use crate::utils::use_router_internal::use_router_internal;

use url::Url;

/// Something that can be converted into a [`NavigationTarget`].
#[derive(Clone)]
pub enum IntoRoutable {
    /// A raw string target.
    FromStr(String),
    /// A internal target.
    Route(Rc<dyn Any>),
}

impl<R: Routable> From<R> for IntoRoutable {
    fn from(value: R) -> Self {
        IntoRoutable::Route(Rc::new(value) as Rc<dyn Any>)
    }
}

impl<R: Routable> From<NavigationTarget<R>> for IntoRoutable {
    fn from(value: NavigationTarget<R>) -> Self {
        match value {
            NavigationTarget::Internal(route) => IntoRoutable::Route(Rc::new(route) as Rc<dyn Any>),
            NavigationTarget::External(url) => IntoRoutable::FromStr(url),
        }
    }
}

impl From<String> for IntoRoutable {
    fn from(value: String) -> Self {
        IntoRoutable::FromStr(value)
    }
}

impl From<&String> for IntoRoutable {
    fn from(value: &String) -> Self {
        IntoRoutable::FromStr(value.to_string())
    }
}

impl From<&str> for IntoRoutable {
    fn from(value: &str) -> Self {
        IntoRoutable::FromStr(value.to_string())
    }
}

impl From<Url> for IntoRoutable {
    fn from(url: Url) -> Self {
        IntoRoutable::FromStr(url.to_string())
    }
}

impl From<&Url> for IntoRoutable {
    fn from(url: &Url) -> Self {
        IntoRoutable::FromStr(url.to_string())
    }
}

/// The properties for a [`Link`].
#[derive(Props)]
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
    pub to: IntoRoutable,
}

impl Debug for LinkProps<'_> {
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
            .finish()
    }
}

/// A link to navigate to another route.
///
/// Only works as descendant of a [`Router`] component, otherwise it will be inactive.
///
/// Unlike a regular HTML anchor, a [`Link`] allows the router to handle the navigation and doesn't
/// cause the browser to load a new page.
///
/// However, in the background a [`Link`] still generates an anchor, which you can use for styling
/// as normal.
///
/// # External targets
/// When the [`Link`]s target is an [`NavigationTarget::External`] target, that is used as the `href` directly. This
/// means that a [`Link`] can always navigate to an [`NavigationTarget::External`] target, even if the [`HistoryProvider`] does not support it.
///
/// # Panic
/// - When the [`Link`] is not nested within a [`Router`], but
///   only in debug builds.
///
/// # Example
/// ```rust
/// # use dioxus::prelude::*;
/// # use dioxus_router::prelude::*;
///
/// #[derive(Clone, Routable)]
/// enum Route {
///     #[route("/")]
///     Index {},
/// }
///
/// #[component]
/// fn App(cx: Scope) -> Element {
///     render! {
///         Router::<Route> {}
///     }
/// }
///
/// #[component]
/// fn Index(cx: Scope) -> Element {
///     render! {
///         render! {
///             Link {
///                 active_class: "active",
///                 class: "link_class",
///                 id: "link_id",
///                 new_tab: true,
///                 rel: "link_rel",
///                 to: Route::Index {},
///    
///                 "A fully configured link"
///             }
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
        id,
        new_tab,
        onclick,
        onclick_only,
        rel,
        to,
        ..
    } = cx.props;

    // hook up to router
    let router = match use_router_internal(cx) {
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

    let current_url = router.current_route_string();
    let href = match to {
        IntoRoutable::FromStr(url) => url.to_string(),
        IntoRoutable::Route(route) => router.any_route_to_string(&**route),
    };
    let parsed_route: NavigationTarget<Rc<dyn Any>> = router.resolve_into_routable(to.clone());
    let ac = active_class
        .and_then(|active_class| (href == current_url).then(|| format!(" {active_class}")))
        .unwrap_or_default();

    let id = id.unwrap_or_default();
    let class = format!("{}{ac}", class.unwrap_or_default());
    let tag_target = new_tab.then_some("_blank").unwrap_or_default();

    let is_external = matches!(parsed_route, NavigationTarget::External(_));
    let is_router_nav = !is_external && !new_tab;
    let prevent_default = is_router_nav.then_some("onclick").unwrap_or_default();
    let rel = rel
        .or_else(|| is_external.then_some("noopener noreferrer"))
        .unwrap_or_default();

    let do_default = onclick.is_none() || !onclick_only;
    let action = move |event| {
        if do_default && is_router_nav {
            router.push_any(router.resolve_into_routable(to.clone()));
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
