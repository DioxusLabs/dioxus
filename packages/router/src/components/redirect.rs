use dioxus_core as dioxus;
use dioxus_core::prelude::*;
use dioxus_core_macro::Props;

use crate::use_router;

/// The props for the [`Router`](fn.Router.html) component.
#[derive(Props)]
pub struct RedirectProps<'a> {
    /// The route to link to. This can be a relative path, or a full URL.
    ///
    /// ```rust
    /// // Absolute path
    /// Redirect { from: "", to: "/home" }
    ///
    /// // Relative path
    /// Redirect { from: "", to: "../" }
    /// ```
    pub to: &'a str,

    /// The route to link from. This can be a relative path, or a full URL.
    ///
    /// ```rust
    /// // Absolute path
    /// Redirect { from: "", to: "/home" }
    ///
    /// // Relative path
    /// Redirect { from: "", to: "../" }
    /// ```
    #[props(optional)]
    pub from: Option<&'a str>,
}

/// If this component is rendered, it will redirect the user to the given route.
///
/// It will replace the current route rather than pushing the current one to the stack.
pub fn Redirect<'a>(cx: Scope<'a, RedirectProps<'a>>) -> Element {
    let router = use_router(&cx);

    // todo: check if the current location matches the "from" pattern
    router.replace_route(cx.props.to, None, None);

    None
}
