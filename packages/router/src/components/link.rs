use crate::RouterService;
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

    #[props(default = true)]
    autodetect: bool,

    /// Is this link an external link?
    #[props(default = false)]
    external: bool,

    /// New tab?
    #[props(default = false)]
    new_tab: bool,

    children: Element<'a>,
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
    let svc = cx.use_hook(|_| cx.consume_context::<RouterService>());

    let LinkProps {
        to,
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

    cx.render(rsx! {
        a {
            href: "{to}",
            class: format_args!("{}", class.unwrap_or("")),
            id: format_args!("{}", id.unwrap_or("")),
            title: format_args!("{}", title.unwrap_or("")),
            prevent_default: "{prevent_default}",
            target: format_args!("{}", if *new_tab { "_blank" } else { "" }),
            onclick: move |_| {
                if !outerlink {
                    if let Some(service) = svc {
                        service.push_route(to);
                    } else {
                        log::error!(
                            "Attempted to create a Link to {} outside of a Router context",
                            cx.props.to,
                        );
                    }
                }
            },

            children
        }
    })
}
