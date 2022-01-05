use dioxus_core::Element;

use dioxus_core as dioxus;
use dioxus_core::prelude::*;
use dioxus_core_macro::*;
use dioxus_html as dioxus_elements;

use crate::RouterService;

#[derive(Props)]
pub struct RouterProps<'a> {
    children: Element<'a>,

    #[props(default, setter(strip_option))]
    onchange: Option<&'a Fn(&'a str)>,
}

pub fn Router<'a>(cx: Scope<'a, RouterProps<'a>>) -> Element {
    let p = cx.use_hook(|_| {
        let update = cx.schedule_update_any();
        cx.provide_context(RouterService::new(update, cx.scope_id()))
    });

    log::debug!("rendering router {:?}", cx.scope_id());

    cx.render(rsx!(
        div { &cx.props.children }
    ))
}
