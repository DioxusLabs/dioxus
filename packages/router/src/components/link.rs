use dioxus_core::{self as dioxus, prelude::*};
use dioxus_core_macro::*;
use dioxus_html as dioxus_elements;
use log::error;

use crate::{
    helpers::{construct_named_path, sub_to_router},
    navigation::NavigationTarget,
    service::RouterMessage,
};

/// The properties for a [`Link`].
#[derive(Props)]
pub struct LinkProps<'a> {
    /// A class to apply to the generated `a` tag when the link is active.
    pub active_class: Option<&'a str>,
    /// The children to render within the [`Link`].
    pub children: Element<'a>,
    /// The `rel` attribute of the rendered `a` tag.
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

    // check if route is active
    let active_class = active_class
        .map(|ac| ac.to_string())
        .or(router.active_class.clone());
    let mut active = String::new();
    if let Some(ac) = active_class {
        match target {
            NavigationTarget::NtPath(p) => {
                if state.path.starts_with(p) {
                    active = ac;
                }
            }
            NavigationTarget::NtName(n, _) => {
                if state.names.contains(n) {
                    active = ac
                }
            }
            NavigationTarget::NtExternal(_) => { /* do nothing */ }
        }
    }

    // generate href
    let href = match target {
        NavigationTarget::NtPath(path) | NavigationTarget::NtExternal(path) => path.to_string(),
        NavigationTarget::NtName(name, vars) => {
            construct_named_path(name, vars, &router.named_routes)
                .unwrap_or(String::from("invalid path"))
        }
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
            class: "{active}",
            prevent_default: "{prevent}",
            onclick: move |_| {
                if !target.is_nt_external() {
                    tx.unbounded_send(RouterMessage::Push(target.clone().into())).ok();
                }
            },
            rel: "{rel}",
            children
        }
    })
}
