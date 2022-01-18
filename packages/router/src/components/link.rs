use crate::RouterService;
use dioxus_core as dioxus;
use dioxus_core::prelude::*;
use dioxus_core_macro::{format_args_f, rsx, Props};
use dioxus_html as dioxus_elements;

#[derive(Props)]
pub struct LinkProps<'a> {
    to: &'a str,

    #[props(optional)]
    class: Option<&'a str>,

    #[props(optional)]
    id: Option<&'a str>,

    #[props(optional)]
    title: Option<&'a str>,

    #[allow(unused)] // temporarily while we work on adding attribute spreading
    #[props(default)]
    attributes: Attributes<'a>,

    children: Element<'a>,
}

pub fn Link<'a>(cx: Scope<'a, LinkProps<'a>>) -> Element {
    match cx.consume_context::<RouterService>() {
        Some(service) => cx.render(rsx! {
            a {
                href: "{cx.props.to}",
                class: format_args!("{}", cx.props.class.unwrap_or("")),
                id: format_args!("{}", cx.props.id.unwrap_or("")),
                title: format_args!("{}", cx.props.title.unwrap_or("")),

                prevent_default: "onclick",
                onclick: move |_| service.push_route(cx.props.to),

                &cx.props.children
            }
        }),
        None => {
            log::warn!(
                "Attempted to create a Link to {} outside of a Router context",
                cx.props.to,
            );
            None
        }
    }
}
