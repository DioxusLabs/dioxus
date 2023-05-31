use dioxus::prelude::*;
use log::error;

use crate::{prelude::*, utils::use_router_internal::use_router_internal};

/// The properties for a [`GoBackButton`] or a [`GoForwardButton`].
#[derive(Debug, Props)]
pub struct GenericHistoryButtonProps<'a> {
    /// The children to render within the generated HTML button tag.
    pub children: Element<'a>,
}

/// A button to go back through the navigation history. Similar to a browsers back button.
///
/// Only works as descendant of a [`GenericRouter`] component, otherwise it will be inactive.
///
/// The button will disable itself if it is known that no prior history is available.
///
/// # Panic
/// - When the [`GoBackButton`] is not nested within a [`GenericRouter`] component
///   hook, but only in debug builds.
///
/// # Example
/// ```rust
/// # use dioxus::prelude::*;
/// # use dioxus_router::prelude::*;
/// # use serde::{Deserialize, Serialize};
/// #[derive(Clone, Serialize, Deserialize, Routable)]
/// enum Route {
///     #[route("/")]
///     Index {},
/// }
///
/// fn App(cx: Scope) -> Element {
///     render! {
///         Router {}
///     }
/// }
///
/// #[inline_props]
/// fn Index(cx: Scope) -> Element {
///     render! {
///         GoBackButton {
///             "go back"
///         }
///     }
/// }
/// #
/// # let mut vdom = VirtualDom::new(App);
/// # let _ = vdom.rebuild();
/// # assert_eq!(
/// #     dioxus_ssr::render(&vdom),
/// #     r#"<button disabled="true" dioxus-prevent-default="onclick">go back</button>"#
/// # );
/// ```
#[allow(non_snake_case)]
pub fn GenericGoBackButton<'a, R: Routable>(
    cx: Scope<'a, GenericHistoryButtonProps<'a>>,
) -> Element {
    let GenericHistoryButtonProps { children } = cx.props;

    // hook up to router
    let router = match use_router_internal::<R>(cx) {
        Some(r) => r,
        #[allow(unreachable_code)]
        None => {
            let msg = "`GoBackButton` must have access to a parent router";
            error!("{msg}, will be inactive");
            #[cfg(debug_assertions)]
            panic!("{}", msg);
            return None;
        }
    };

    let disabled = !router.can_go_back();

    render! {
        button {
            disabled: "{disabled}",
            prevent_default: "onclick",
            onclick: move |_| router.go_back(),
            children
        }
    }
}

/// A button to go forward through the navigation history. Similar to a browsers forward button.
///
/// Only works as descendant of a [`GenericRouter`] component, otherwise it will be inactive.
///
/// The button will disable itself if it is known that no later history is available.
///
/// # Panic
/// - When the [`GoForwardButton`] is not nested within a [`GenericRouter`] component
///   hook, but only in debug builds.
///
/// # Example
/// ```rust
/// # use dioxus::prelude::*;
/// # use dioxus_router::prelude::*;
/// # use serde::{Deserialize, Serialize};
/// #[derive(Clone, Serialize, Deserialize, Routable)]
/// enum Route {
///     #[route("/")]
///     Index {},
/// }
///
/// fn App(cx: Scope) -> Element {
///     render! {
///         Router {}
///     }
/// }
///
/// #[inline_props]
/// fn Index(cx: Scope) -> Element {
///     render! {
///         GoForwardButton {
///             "go forward"
///         }
///     }
/// }
/// #
/// # let mut vdom = VirtualDom::new(App);
/// # let _ = vdom.rebuild();
/// # assert_eq!(
/// #     dioxus_ssr::render(&vdom),
/// #     r#"<button disabled="true" dioxus-prevent-default="onclick">go forward</button>"#
/// # );
/// ```
#[allow(non_snake_case)]
pub fn GenericGoForwardButton<'a, R: Routable>(
    cx: Scope<'a, GenericHistoryButtonProps<'a>>,
) -> Element {
    let GenericHistoryButtonProps { children } = cx.props;

    // hook up to router
    let router = match use_router_internal::<R>(cx) {
        Some(r) => r,
        #[allow(unreachable_code)]
        None => {
            let msg = "`GoForwardButton` must have access to a parent router";
            error!("{msg}, will be inactive");
            #[cfg(debug_assertions)]
            panic!("{}", msg);
            return None;
        }
    };

    let disabled = !router.can_go_back();

    render! {
        button {
            disabled: "{disabled}",
            prevent_default: "onclick",
            onclick: move |_| router.go_forward(),
            children
        }
    }
}
