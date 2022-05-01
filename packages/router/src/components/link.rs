use dioxus_core::{self as dioxus, prelude::*};
use dioxus_core_macro::*;
use dioxus_html as dioxus_elements;
use log::error;

use crate::{
    helpers::{construct_named_path, sub_to_router},
    prelude::NavigationTarget,
    service::RouterMessage,
};

/// The properties for a [`Link`].
#[derive(Props)]
pub struct LinkProps<'a> {
    /// The children to render within the [`Link`].
    pub children: Element<'a>,
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
    let LinkProps { children, target } = cx.props;

    // hook up to router
    let router = match sub_to_router(&cx) {
        Some(x) => x,
        None => {
            error!("`Link` can only be used as a descendent of a `Router`");
            return None;
        }
    };
    // let state = router.state.read().expect("router lock poison");
    let tx = router.tx.clone();

    // generate url for path
    let href = match target {
        NavigationTarget::RPath(path) | NavigationTarget::RExternal(path) => path.to_string(),
        NavigationTarget::RName(name, vars) => {
            construct_named_path(name, vars, &router.named_routes)
                .unwrap_or(String::from("invalid path"))
        }
    };

    // prepare prevented defaults
    let prevent = match target.is_rexternal() {
        true => "",
        false => "onclick",
    };

    cx.render(rsx! {
        a {
            href: "{href}",
            prevent_default: "{prevent}",
            onclick: move |_| {
                if !target.is_rexternal() {
                    tx.unbounded_send(RouterMessage::Push(target.clone().into())).ok();
                }
            },
            children
        }
    })
}
