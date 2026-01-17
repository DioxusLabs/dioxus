#![allow(clippy::type_complexity)]

use std::fmt::Debug;

use dioxus_core::{Attribute, Element, EventHandler, VNode};
use dioxus_core_macro::{rsx, Props};
use dioxus_html::{
    self as dioxus_elements, ModifiersInteraction, MountedEvent, MouseEvent, PointerInteraction,
};

use tracing::error;

use crate::navigation::NavigationTarget;
use crate::utils::use_router_internal::use_router_internal;

/// The properties for a [`Link`].
#[derive(Props, Clone, PartialEq)]
pub struct LinkProps {
    /// The class attribute for the `a` tag.
    pub class: Option<String>,

    /// A class to apply to the generate HTML anchor tag if the `target` route is active.
    pub active_class: Option<String>,

    /// The children to render within the generated HTML anchor tag.
    pub children: Element,

    /// When [`true`], the `target` route will be opened in a new tab.
    ///
    /// This does not change whether the [`Link`] is active or not.
    #[props(default)]
    pub new_tab: bool,

    /// The onclick event handler.
    pub onclick: Option<EventHandler<MouseEvent>>,

    /// The onmounted event handler.
    /// Fired when the `<a>` element is mounted.
    pub onmounted: Option<EventHandler<MountedEvent>>,

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
    /// For external `a`s, this defaults to `noopener noreferrer`.
    pub rel: Option<String>,

    /// The navigation target. Roughly equivalent to the href attribute of an HTML anchor tag.
    #[props(into)]
    pub to: NavigationTarget,

    #[props(extends = GlobalAttributes)]
    attributes: Vec<Attribute>,
}

impl Debug for LinkProps {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LinkProps")
            .field("active_class", &self.active_class)
            .field("children", &self.children)
            .field("attributes", &self.attributes)
            .field("new_tab", &self.new_tab)
            .field("onclick", &self.onclick.as_ref().map(|_| "onclick is set"))
            .field("onclick_only", &self.onclick_only)
            .field("rel", &self.rel)
            .finish()
    }
}

/// A link to navigate to another route.
///
/// Only works as descendant of a [`super::Router`] component, otherwise it will be inactive.
///
/// Unlike a regular HTML anchor, a [`Link`] allows the router to handle the navigation and doesn't
/// cause the browser to load a new page.
///
/// However, in the background a [`Link`] still generates an anchor, which you can use for styling
/// as normal.
///
/// # External targets
/// When the [`Link`]s target is an [`NavigationTarget::External`] target, that is used as the `href` directly. This
/// means that a [`Link`] can always navigate to an [`NavigationTarget::External`] target, even if the [`dioxus_history::History`] does not support it.
///
/// # Panic
/// - When the [`Link`] is not nested within a [`super::Router`], but
///   only in debug builds.
///
/// # Example
/// ```rust
/// # use dioxus::prelude::*;
///
/// #[derive(Clone, Routable)]
/// enum Route {
///     #[route("/")]
///     Index {},
/// }
///
/// #[component]
/// fn App() -> Element {
///     rsx! {
///         Router::<Route> {}
///     }
/// }
///
/// #[component]
/// fn Index() -> Element {
///     rsx! {
///         Link {
///             active_class: "active",
///             class: "link_class",
///             id: "link_id",
///             new_tab: true,
///             rel: "link_rel",
///             to: Route::Index {},
///
///             "A fully configured link"
///         }
///     }
/// }
/// #
/// # let mut vdom = VirtualDom::new(App);
/// # vdom.rebuild_in_place();
/// # assert_eq!(
/// #     dioxus_ssr::render(&vdom),
/// #     r#"<a href="/" class="link_class active" rel="link_rel" target="_blank" aria-current="page" id="link_id">A fully configured link</a>"#
/// # );
/// ```
#[doc(alias = "<a>")]
#[allow(non_snake_case)]
pub fn Link(props: LinkProps) -> Element {
    let LinkProps {
        active_class,
        children,
        attributes,
        new_tab,
        onclick,
        onclick_only,
        rel,
        to,
        class,
        ..
    } = props;

    // hook up to router
    let router = match use_router_internal() {
        Some(r) => r,
        #[allow(unreachable_code)]
        None => {
            let msg = "`Link` must have access to a parent router";
            error!("{msg}, will be inactive");
            #[cfg(debug_assertions)]
            panic!("{}", msg);
            return VNode::empty();
        }
    };

    let current_url = router.full_route_string();
    let href = match &to {
        NavigationTarget::Internal(url) => url.clone(),
        NavigationTarget::External(route) => route.clone(),
    };
    // Add the history's prefix to internal hrefs for use in the rsx
    let full_href = match &to {
        NavigationTarget::Internal(url) => router.prefix().unwrap_or_default() + url,
        NavigationTarget::External(route) => route.clone(),
    };

    let mut class_ = String::new();
    if let Some(c) = class {
        class_.push_str(&c);
    }
    if let Some(c) = active_class {
        if href == current_url {
            if !class_.is_empty() {
                class_.push(' ');
            }
            class_.push_str(&c);
        }
    }

    let class = if class_.is_empty() {
        None
    } else {
        Some(class_)
    };

    let aria_current = (href == current_url).then_some("page");

    let tag_target = new_tab.then_some("_blank");

    let is_external = matches!(to, NavigationTarget::External(_));
    let is_router_nav = !is_external && !new_tab;
    let rel = rel.or_else(|| is_external.then_some("noopener noreferrer".to_string()));

    let do_default = onclick.is_none() || !onclick_only;

    let action = move |event: MouseEvent| {
        // Only handle events without modifiers
        if !event.modifiers().is_empty() {
            return;
        }
        // Only handle left clicks
        if event.trigger_button() != Some(dioxus_elements::input_data::MouseButton::Primary) {
            return;
        }

        // If we need to open in a new tab, let the browser handle it
        if new_tab {
            return;
        }

        // todo(jon): this is extra hacky for no reason - we should fix prevent default on Links
        if do_default && is_external {
            return;
        }

        event.prevent_default();

        if do_default && is_router_nav {
            router.push_any(to.clone());
        }

        if let Some(handler) = onclick {
            handler.call(event);
        }
    };

    let onmounted = move |event| {
        if let Some(handler) = props.onmounted {
            handler.call(event);
        }
    };

    // In liveview, we need to prevent the default action if the user clicks on the link with modifiers
    // in javascript. The prevent_default method is not available in the liveview renderer because
    // event handlers are handled over a websocket.
    let liveview_prevent_default = {
        // If the event is a click with the left mouse button and no modifiers, prevent the default action
        // and navigate to the href with client side routing
        router.include_prevent_default().then_some(
            "if (event.button === 0 && !event.ctrlKey && !event.metaKey && !event.shiftKey && !event.altKey) { event.preventDefault() }"
        )
    };

    rsx! {
        a {
            onclick: action,
            "onclick": liveview_prevent_default,
            href: full_href,
            onmounted: onmounted,
            class,
            rel,
            target: tag_target,
            aria_current,
            ..attributes,
            {children}
        }
    }
}
