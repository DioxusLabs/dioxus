use dioxus::prelude::*;
use dioxus_router_core::RouterMessage;
use log::error;

use crate::utils::use_router_internal::use_router_internal;

#[derive(Debug, Props)]
pub struct HistoryButtonProps<'a> {
    pub children: Element<'a>,
}

#[allow(non_snake_case)]
pub fn GoBackButton<'a>(cx: Scope<'a, HistoryButtonProps<'a>>) -> Element {
    let HistoryButtonProps { children } = cx.props;

    // hook up to router
    let router = match use_router_internal(&cx) {
        Some(r) => r,
        None => {
            let msg = "`GoBackButton` must have access to a parent router";
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

    let disabled = !state.can_go_back;

    render! {
        button {
            disabled: "{disabled}",
            prevent_default: "onclick",
            onclick: move |_| { let _ = sender.unbounded_send(RouterMessage::GoBack); },
            children
        }
    }
}

#[allow(non_snake_case)]
pub fn GoForwardButton<'a>(cx: Scope<'a, HistoryButtonProps<'a>>) -> Element {
    let HistoryButtonProps { children } = cx.props;

    // hook up to router
    let router = match use_router_internal(&cx) {
        Some(r) => r,
        None => {
            let msg = "`GoForwardButton` must have access to a parent router";
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

    let disabled = !state.can_go_back;

    render! {
        button {
            disabled: "{disabled}",
            prevent_default: "onclick",
            onclick: move |_| { let _ = sender.unbounded_send(RouterMessage::GoForward); },
            children
        }
    }
}
