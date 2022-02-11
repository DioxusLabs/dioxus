use dioxus_core as dioxus;
use dioxus_core::prelude::*;
use dioxus_core::Element;
use dioxus_core_macro::*;
use dioxus_html as dioxus_elements;

use crate::{RouterProvider, RouterService};

/// The props for the [`Router`](fn.Router.html) component.
#[derive(Props)]
pub struct RouterProps<'a> {
    /// The routes and elements that should be rendered when the path matches.
    ///
    /// If elements are not contained within Routes, the will be rendered
    /// regardless of the path.
    pub children: Element<'a>,

    ///
    #[props(default)]
    pub onchange: EventHandler<'a, String>,
}

///
///
///
///
///
///
///
#[allow(non_snake_case)]
pub fn Router<'a>(cx: Scope<'a, RouterProps<'a>>) -> Element {
    let svc = cx.use_hook(|_| {
        cx.provide_context(RouterService::new(cx.schedule_update_any(), cx.scope_id()))
    });

    let any_pending = svc.pending_events.borrow().len() > 0;
    svc.pending_events.borrow_mut().clear();

    if any_pending {
        cx.props.onchange.call(svc.current_location().to_string());
    }

    cx.render(rsx!(
        div { &cx.props.children }
    ))
}
