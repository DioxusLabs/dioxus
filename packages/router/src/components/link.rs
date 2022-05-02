use dioxus_core::{self as dioxus, prelude::*};
use dioxus_core_macro::*;
use dioxus_html as dioxus_elements;
use log::error;

use crate::{
    helpers::{construct_named_path, sub_to_router},
    navigation::NavigationTarget::{self, *},
    service::RouterMessage,
};

/// The properties for a [`Link`].
#[derive(Props)]
pub struct LinkProps<'a> {
    /// A class to apply to the inner `a` tag when the link is active.
    pub active_class: Option<&'a str>,
    /// The children to render within the [`Link`].
    pub children: Element<'a>,
    /// The classes of the inner `a` tag.
    ///
    /// When the link is active and an `active_class` is provided, it is appended at the end.
    pub class: Option<&'a str>,
    /// An ID of the inner `a` tag.
    pub id: Option<&'a str>,
    /// Specify whether the link should be opened in a new tab.
    #[props(default)]
    pub new_tab: bool,
    /// The `rel` attribute of the inner `a` tag.
    ///
    /// Defaults to `"noreferrer noopener"` for [`NtExternal`] targets.
    pub rel: Option<&'a str>,
    /// The navigation target. Corresponds to the `href` of an `a` tag.
    pub target: NavigationTarget,
}

/// A link to navigate to another route.
///
/// Needs a [Router](crate::components::Router) as an ancestor.
///
/// Unlike a regular `a` tag, a [`Link`] allows the router to handle the navigation and doesn't
/// cause the browser to load a new page.
///
/// However, in the background a [`Link`] still generates an `a` tag, which you can use for styling
/// as normal.
#[allow(non_snake_case)]
pub fn Link<'a>(cx: Scope<'a, LinkProps<'a>>) -> Element {
    let LinkProps {
        active_class,
        children,
        class,
        id,
        new_tab,
        rel,
        target,
    } = cx.props;

    // hook up to router
    let router = match sub_to_router(&cx) {
        Some(x) => x,
        None => {
            error!("`Link` can only be used as a descendent of a `Router`");
            return None;
        }
    };
    let state = router.state.read().expect("router lock poison");
    let tx = router.tx.clone();

    // generate href
    let href = match target {
        NtPath(path) | NavigationTarget::NtExternal(path) => path.to_string(),
        NtName(name, vars) => construct_named_path(name, vars, &router.named_routes)
            .unwrap_or(String::from("invalid path")),
    };

    // check if route is active
    let active_class = active_class
        .map(|ac| ac.to_string())
        .or(router.active_class.clone())
        .map(|ac| {
            match target {
                NtPath(p) => {
                    if p.starts_with("/") && state.path.starts_with(p) {
                        return format!(" {ac}");
                    }

                    if !state.path.ends_with("/") {
                        if let Some(path) = state.path.split("/").last() {
                            if p == path {
                                return format!(" {ac}");
                            }
                        }
                    }
                }
                NtName(n, _) => {
                    if state.names.contains(n) {
                        return format!(" {ac}");
                    }
                }
                NtExternal(_) => {}
            }
            String::new()
        })
        .unwrap_or_default();

    // prepare id, class and target for the `a` tag
    let id = id.unwrap_or_default();
    let class = class.unwrap_or_default();
    let class = format!("{class}{active_class}");
    let tag_target = match new_tab {
        true => "_blank",
        false => "",
    };

    // prepare prevented defaults
    let prevent = match target.is_nt_external() {
        true => "",
        false => "onclick",
    };

    // get rel attribute or apply default if external
    let rel = rel
        .or(if target.is_nt_external() {
            Some("noopener noreferrer")
        } else {
            None
        })
        .unwrap_or("");

    cx.render(rsx! {
        a {
            href: "{href}",
            prevent_default: "{prevent}",
            onclick: move |_| {
                if !target.is_nt_external() {
                    tx.unbounded_send(RouterMessage::Push(target.clone().into())).ok();
                }
            },
            class: "{class}",
            id: "{id}",
            rel: "{rel}",
            target: "{tag_target}",
            children
        }
    })
}
