use dioxus_core::Element;

use dioxus_core as dioxus;
use dioxus_core::prelude::*;
use dioxus_core_macro::*;
use dioxus_html as dioxus_elements;

use crate::RouterService;

#[derive(Props)]
pub struct RouterProps<'a> {
    children: Element<'a>,

    #[props(default)]
    onchange: EventHandler<'a, String>,
}

#[allow(non_snake_case)]
pub fn Router<'a>(cx: Scope<'a, RouterProps<'a>>) -> Element {
    log::debug!("running router {:?}", cx.scope_id());
    let svc = cx.use_hook(|_| {
        let update = cx.schedule_update_any();
        cx.provide_context(RouterService::new(update, cx.scope_id()))
    });

    let any_pending = svc.pending_events.borrow().len() > 0;
    svc.pending_events.borrow_mut().clear();

    if any_pending {
        let location = svc.current_location();
        let path = location.path();
        cx.props.onchange.call(path.to_string());
    }

    cx.render(rsx!(&cx.props.children))
}
