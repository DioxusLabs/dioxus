use dioxus::prelude::*;
use tracing::error;

use crate::utils::use_router_internal::use_router_internal;

/// The properties for a [`GoBackButton`] or a [`GoForwardButton`].
#[derive(Debug, Props)]
pub struct HistoryButtonProps<'a> {
    /// The children to render within the generated HTML button tag.
    pub children: Element<'a>,
}

/// A button to go back through the navigation history. Similar to a browsers back button.
///
/// Only works as descendant of a [`Link`] component, otherwise it will be inactive.
///
/// The button will disable itself if it is known that no prior history is available.
///
/// # Panic
/// - When the [`GoBackButton`] is not nested within a [`Link`] component
///   hook, but only in debug builds.
///
/// # Example
/// ```rust
/// # use dioxus::prelude::*;
/// # use dioxus_router::prelude::*;
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
#[component]
pub fn GoBackButton<'a>(cx: Scope<'a, HistoryButtonProps<'a>>) -> Element {
    let HistoryButtonProps { children } = cx.props;

    // hook up to router
    let router = match use_router_internal(cx) {
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
/// Only works as descendant of a [`Link`] component, otherwise it will be inactive.
///
/// The button will disable itself if it is known that no later history is available.
///
/// # Panic
/// - When the [`GoForwardButton`] is not nested within a [`Link`] component
///   hook, but only in debug builds.
///
/// # Example
/// ```rust
/// # use dioxus::prelude::*;
/// # use dioxus_router::prelude::*;
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
#[component]
pub fn GoForwardButton<'a>(cx: Scope<'a, HistoryButtonProps<'a>>) -> Element {
    let HistoryButtonProps { children } = cx.props;

    // hook up to router
    let router = match use_router_internal(cx) {
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
