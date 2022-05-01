use dioxus_core::{self as dioxus, prelude::*};
use dioxus_core_macro::*;
use dioxus_html as dioxus_elements;
use log::error;

use crate::{helpers::sub_to_router, service::RouterMessage};

/// The props for [`GoBackButton`] and [`GoForwardButton`].
#[derive(Props)]
pub struct HistoryButtonProps<'a> {
    /// The children to render inside the button.
    pub children: Element<'a>,
}

/// A button that acts like a browsers back button.
///
/// Needs a [Router](crate::components::Router) as an ancestor. If the rendered button is disabled,
/// no prior history is available.
#[allow(non_snake_case)]
pub fn GoBackButton<'a>(cx: Scope<'a, HistoryButtonProps<'a>>) -> Element {
    // hook up to router
    let router = match sub_to_router(&cx) {
        Some(x) => x,
        None => {
            error!("`GoButton` can only be used as a descendent of a `Router`");
            return None;
        }
    };
    let state = router.state.read().expect("router lock poison");
    let tx = router.tx.clone();

    let disabled = !state.can_go_back;

    cx.render(rsx! {
        button {
            disabled: "{disabled}",
            prevent_default: "onclick",
            onclick: move |_| {tx.unbounded_send(RouterMessage::GoBack).ok();},
            &cx.props.children
        }
    })
}

/// A button that acts like a browsers forward button.
///
/// Needs a [Router](crate::components::Router) as an ancestor. If the button is disabled, no
/// "future history" is available.
#[allow(non_snake_case)]
pub fn GoForwardButton<'a>(cx: Scope<'a, HistoryButtonProps<'a>>) -> Element {
    // hook up to router
    // hook up to router
    let router = match sub_to_router(&cx) {
        Some(x) => x,
        None => {
            error!("`GoButton` can only be used as a descendent of a `Router`");
            return None;
        }
    };
    let state = router.state.read().expect("router lock poison");
    let tx = router.tx.clone();

    let disabled = !state.can_go_forward;

    cx.render(rsx! {
        button {
            disabled: "{disabled}",
            prevent_default: "onclick",
            onclick: move |_| {tx.unbounded_send(RouterMessage::GoForward).ok();},
            &cx.props.children
        }
    })
}
