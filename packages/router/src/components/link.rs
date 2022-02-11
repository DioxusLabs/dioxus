use crate::RouterService;
use dioxus::Attribute;
use dioxus_core as dioxus;
use dioxus_core::prelude::*;
use dioxus_core_macro::{format_args_f, rsx, Props};
use dioxus_html as dioxus_elements;

/// Props for the [`Link`](struct.Link.html) component.
#[derive(Props)]
pub struct LinkProps<'a> {
    /// The route to link to. This can be a relative path, or a full URL.
    ///
    /// ```rust
    /// // Absolute path
    /// Link { to: "/home", "Go Home" }
    ///
    /// // Relative path
    /// Link { to: "../", "Go Up" }
    /// ```
    pub to: &'a str,

    /// Set the class of the inner link ['a'](https://www.w3schools.com/tags/tag_a.asp) element.
    ///
    /// This can be useful when styling the inner link element.
    #[props(default, strip_option)]
    pub class: Option<&'a str>,

    /// Set the ID of the inner link ['a'](https://www.w3schools.com/tags/tag_a.asp) element.
    ///
    /// This can be useful when styling the inner link element.
    #[props(default, strip_option)]
    pub id: Option<&'a str>,

    /// Set the title of the window after the link is clicked..
    #[props(default, strip_option)]
    pub title: Option<&'a str>,

    /// Set child elements of the link.
    pub children: Element<'a>,

    #[props(default)]
    attributes: Option<&'a [Attribute<'a>]>,
}

/// A component that renders a link to a route.
///
/// `Link` components are just [`<a>`](https://www.w3schools.com/tags/tag_a.asp) elements
/// that link to different pages *within* your single-page app.
///
/// If you need to link to a resource outside of your app, then just use a regular
/// `<a>` element directly.
///
/// # Examples
///
/// ```rust
/// fn Header(cx: Scope) -> Element {
///     cx.render(rsx!{
///         Link { to: "/home", "Go Home" }
///     })
/// }
/// ```
pub fn Link<'a>(cx: Scope<'a, LinkProps<'a>>) -> Element {
    if let Some(service) = cx.consume_context::<RouterService>() {
        cx.render(rsx! {
            a {
                href: "{cx.props.to}",
                class: format_args!("{}", cx.props.class.unwrap_or("")),
                id: format_args!("{}", cx.props.id.unwrap_or("")),
                title: format_args!("{}", cx.props.title.unwrap_or("")),
                prevent_default: "onclick",
                onclick: move |_| service.push_route(cx.props.to),
                &cx.props.children
            }
        })
    } else {
        log::error!(
            "Attempted to create a Link to {} outside of a Router context",
            cx.props.to,
        );
        None
    }
}
