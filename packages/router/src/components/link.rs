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
    /// You can either put in your own inline method or just autoderive the route using `derive(Routable)`
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
    #[props(default, strip_option)]
    href: Option<&'a str>,

    #[props(default, strip_option)]
    class: Option<&'a str>,

    #[props(default, strip_option)]
    id: Option<&'a str>,

    #[props(default, strip_option)]
    title: Option<&'a str>,

    #[props(default = true)]
    autodetect: bool,

    /// Is this link an external link?
    #[props(default = false)]
    external: bool,

    /// New tab?
    #[props(default = false)]
    new_tab: bool,

    children: Element<'a>,

    #[props(default)]
    attributes: Option<&'a [Attribute<'a>]>,
}

pub fn Link<'a>(cx: Scope<'a, LinkProps<'a>>) -> Element {
    if let Some(service) = cx.consume_context::<RouterService>() {
        let LinkProps {
            to,
            href,
            class,
            id,
            title,
            autodetect,
            external,
            new_tab,
            children,
            ..
        } = cx.props;

        let is_http = to.starts_with("http") || to.starts_with("https");
        let outerlink = (*autodetect && is_http) || *external;

        let prevent_default = if outerlink { "" } else { "onclick" };

        return cx.render(rsx! {
            a {
                href: "{to}",
                class: format_args!("{}", class.unwrap_or("")),
                id: format_args!("{}", id.unwrap_or("")),
                title: format_args!("{}", title.unwrap_or("")),
                prevent_default: "{prevent_default}",
                target: format_args!("{}", if *new_tab { "_blank" } else { "" }),
                onclick: move |_| {
                    if !outerlink {
                        service.push_route(to);
                    }
                },

                &cx.props.children
            }
        });
    }
    log::warn!(
        "Attempted to create a Link to {} outside of a Router context",
        cx.props.to,
    );
    None
}
