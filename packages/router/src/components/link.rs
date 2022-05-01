use dioxus_core::{self as dioxus, prelude::*};
use dioxus_core_macro::*;
use dioxus_html as dioxus_elements;
use log::error;

use crate::{helpers::sub_to_router, service::RouterMessage};

/// The properties for a [`Link`].
#[derive(Props)]
pub struct LinkProps<'a> {
    /// The children to render within the [`Link`].
    pub children: Element<'a>,
    /// The navigation target. Corresponds to the `href` of an `a` tag.
    pub target: &'a str,
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

    cx.render(rsx! {
        a {
            href: "{target}",
            prevent_default: "onclick",
            onclick: move |_| {tx.unbounded_send(RouterMessage::Push(target.to_string())).ok();},
            children
        }
    })
}
