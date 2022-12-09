use dioxus::prelude::*;
use dioxus_router_core::{navigation::NavigationTarget, RouterMessage};
use log::error;

use crate::utils::use_router_internal::use_router_internal;

/// The properties for a [`Link`].
#[derive(Debug, Props)]
pub struct LinkProps<'a> {
    /// A class to apply to the generated HTML anchor when the `target` route is active.
    ///
    /// This overrides the `active_class` property of a [`Router`].
    ///
    /// [`Router`]: crate::components::Router
    pub active_class: Option<&'a str>,
    /// The children to render within the generated HTML anchor.
    pub children: Element<'a>,
    /// The `class` attribute of the generated HTML anchor.
    ///
    /// When the `target` route is active, `active_class` is appended at the end.
    pub class: Option<&'a str>,
    /// Require the _exact_ target route to be active, for the link to be active. See
    /// [`RouterState::is_active`](crate::state::RouterState::is_active).
    #[props(default)]
    pub exact: bool,
    /// The `id` attribute of the generated HTML anchor.
    pub id: Option<&'a str>,
    /// When [`true`], the `target` will be opened in a new tab.
    #[props(default)]
    pub new_tab: bool,
    /// The `rel` attribute of the generated HTML anchor.
    ///
    /// Defaults to `"noreferrer noopener"` for [`ExternalTarget`] targets.
    pub rel: Option<&'a str>,
    /// The navigation target. Corresponds to the `href` of an HTML anchor.
    #[props(into)]
    pub target: NavigationTarget,
}

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
        None => {
            let msg = "`Link` must have access to a parent router";
            error!("{msg}, will be inactive");
            #[cfg(debug_assertions)]
            panic!("{}", msg);
            #[cfg(not(debug_assertions))]
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
    let rel = rel.unwrap_or(
        is_external
            .then_some("noopener noreferrer")
            .unwrap_or_default(),
    );

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
