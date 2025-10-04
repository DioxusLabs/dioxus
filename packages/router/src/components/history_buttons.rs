use dioxus_core::{Element, VNode};
use dioxus_core_macro::{rsx, Props};
use dioxus_html as dioxus_elements;

use tracing::error;

use crate::utils::use_router_internal::use_router_internal;

/// The properties for a [`GoBackButton`] or a [`GoForwardButton`].
#[derive(Debug, Props, Clone, PartialEq)]
pub struct HistoryButtonProps {
    /// The children to render within the generated HTML button tag.
    pub children: Element,
}

/// A button to go back through the navigation history. Similar to a browsers back button.
///
/// Only works as descendant of a [`super::Link`] component, otherwise it will be inactive.
///
/// The button will disable itself if it is known that no prior history is available.
///
/// # Panic
/// - When the [`GoBackButton`] is not nested within a [`super::Link`] component
///   hook, but only in debug builds.
///
/// # Example
/// ```rust
/// # use dioxus::prelude::*;
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
///         GoBackButton {
///             "go back"
///         }
///     }
/// }
/// #
/// # let mut vdom = VirtualDom::new(App);
/// # vdom.rebuild_in_place();
/// # assert_eq!(
/// #     dioxus_ssr::render(&vdom),
/// #     r#"<button disabled="true">go back</button>"#
/// # );
/// ```
pub fn GoBackButton(props: HistoryButtonProps) -> Element {
    let HistoryButtonProps { children } = props;

    // hook up to router
    let router = match use_router_internal() {
        Some(r) => r,
        #[allow(unreachable_code)]
        None => {
            let msg = "`GoBackButton` must have access to a parent router";
            error!("{msg}, will be inactive");
            #[cfg(debug_assertions)]
            panic!("{}", msg);
            return VNode::empty();
        }
    };

    let disabled = !router.can_go_back();

    rsx! {
        button {
            disabled: "{disabled}",
            onclick: move |evt| {
                evt.prevent_default();
                router.go_back()
            },
            {children}
        }
    }
}

/// A button to go forward through the navigation history. Similar to a browsers forward button.
///
/// Only works as descendant of a [`super::Link`] component, otherwise it will be inactive.
///
/// The button will disable itself if it is known that no later history is available.
///
/// # Panic
/// - When the [`GoForwardButton`] is not nested within a [`super::Link`] component
///   hook, but only in debug builds.
///
/// # Example
/// ```rust
/// # use dioxus::prelude::*;
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
///         GoForwardButton {
///             "go forward"
///         }
///     }
/// }
/// #
/// # let mut vdom = VirtualDom::new(App);
/// # vdom.rebuild_in_place();
/// # assert_eq!(
/// #     dioxus_ssr::render(&vdom),
/// #     r#"<button disabled="true">go forward</button>"#
/// # );
/// ```
pub fn GoForwardButton(props: HistoryButtonProps) -> Element {
    let HistoryButtonProps { children } = props;

    // hook up to router
    let router = match use_router_internal() {
        Some(r) => r,
        #[allow(unreachable_code)]
        None => {
            let msg = "`GoForwardButton` must have access to a parent router";
            error!("{msg}, will be inactive");
            #[cfg(debug_assertions)]
            panic!("{}", msg);
            return VNode::empty();
        }
    };

    let disabled = !router.can_go_forward();

    rsx! {
        button {
            disabled: "{disabled}",
            onclick: move |evt| {
                evt.prevent_default();
                router.go_forward()
            },
            {children}
        }
    }
}
