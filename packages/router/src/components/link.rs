use crate::RouterService;
use dioxus::Attribute;
use dioxus_core as dioxus;
use dioxus_core::prelude::*;
use dioxus_core_macro::{format_args_f, rsx, Props};
use dioxus_html as dioxus_elements;

#[derive(Props)]
pub struct LinkProps<'a> {
    to: &'a str,

    /// The url that gets pushed to the history stack
    ///
    /// You can either put it your own inline method or just autoderive the route using `derive(Routable)`
    ///
    /// ```rust, ignore
    ///
    /// Link { to: Route::Home, href: |_| "home".to_string() }
    ///
    /// // or
    ///
    /// Link { to: Route::Home, href: Route::as_url }
    ///
    /// ```
    #[props(default, setter(strip_option))]
    href: Option<&'a str>,

    #[props(default, setter(strip_option))]
    class: Option<&'a str>,

    #[props(default, setter(strip_option))]
    id: Option<&'a str>,

    children: Element<'a>,

    #[props(default)]
    attributes: Option<&'a [Attribute<'a>]>,
}

pub fn Link<'a>(cx: Scope<'a, LinkProps<'a>>) -> Element {
    let service = cx.consume_context::<RouterService>()?;
    cx.render(rsx! {
        a {
            href: "{cx.props.to}",
            class: format_args!("{}", cx.props.class.unwrap_or("")),
            id: format_args!("{}", cx.props.id.unwrap_or("")),

            prevent_default: "onclick",
            onclick: move |_| service.push_route(cx.props.to.clone()),

            &cx.props.children
        }
    })
}
