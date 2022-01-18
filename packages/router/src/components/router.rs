use dioxus_core::Element;

use dioxus_core as dioxus;
use dioxus_core::prelude::*;
use dioxus_core_macro::*;
use dioxus_html as dioxus_elements;

use crate::RouterService;

#[derive(Props)]
pub struct RouterProps<'a> {
    children: Element<'a>,

    #[allow(unused)] // temporarily
    #[props(default)]
    onchange: EventHandler<'a, String>,
}

#[allow(non_snake_case)]
pub fn Router<'a>(cx: Scope<'a, RouterProps<'a>>) -> Element {
    cx.use_hook(|_| {
        let update = cx.schedule_update_any();
        cx.provide_context(RouterService::new(update, cx.scope_id()))
    });

    cx.render(rsx!(&cx.props.children))
}
