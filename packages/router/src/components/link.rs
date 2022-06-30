use std::sync::Arc;

use crate::{use_route, RouterCore};
use dioxus::prelude::*;

/// Props for the [`Link`](struct.Link.html) component.
#[derive(Props)]
pub struct LinkProps<'a> {
    /// The route to link to. This can be a relative path, or a full URL.
    ///
    /// ```rust, ignore
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

    /// Set the class added to the inner link when the current route is the same as the "to" route.
    ///
    /// To set all of the active classes inside a Router at the same time use the `active_class`
    /// prop on the Router component. If both the Router prop as well as this prop are provided then
    /// this one has precedence. By default set to `"active"`.
    #[props(default, strip_option)]
    pub active_class: Option<&'a str>,

    /// Set the ID of the inner link ['a'](https://www.w3schools.com/tags/tag_a.asp) element.
    ///
    /// This can be useful when styling the inner link element.
    #[props(default, strip_option)]
    pub id: Option<&'a str>,

    /// Set the title of the window after the link is clicked..
    #[props(default, strip_option)]
    pub title: Option<&'a str>,

    /// Autodetect if a link is external or not.
    ///
    /// This is automatically set to `true` and will use http/https detection
    #[props(default = true)]
    pub autodetect: bool,

    /// Is this link an external link?
    #[props(default = false)]
    pub external: bool,

    /// New tab?
    #[props(default = false)]
    pub new_tab: bool,

    /// Pass children into the `<a>` element
    pub children: Element<'a>,
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
/// ```rust, ignore
/// fn Header(cx: Scope) -> Element {
///     cx.render(rsx!{
///         Link { to: "/home", "Go Home" }
///     })
/// }
/// ```
pub fn Link<'a>(cx: Scope<'a, LinkProps<'a>>) -> Element {
    let svc = cx.use_hook(|_| cx.consume_context::<Arc<RouterCore>>());

    let LinkProps {
        to,
        class,
        id,
        title,
        autodetect,
        external,
        new_tab,
        children,
        active_class,
        ..
    } = cx.props;

    let is_http = to.starts_with("http") || to.starts_with("https");
    let outerlink = (*autodetect && is_http) || *external;
    let prevent_default = if outerlink { "" } else { "onclick" };

    let active_class_name = match active_class {
        Some(c) => (*c).into(),
        None => {
            let active_from_router = match svc {
                Some(service) => service.cfg.active_class.clone(),
                None => None,
            };
            active_from_router.unwrap_or_else(|| "active".into())
        }
    };

    let route = use_route(&cx);
    let url = route.url();
    let path = url.path();
    let active = path == cx.props.to;
    let active_class = if active { active_class_name } else { "".into() };

    cx.render(rsx! {
        a {
            href: "{to}",
            class: format_args!("{} {}", class.unwrap_or(""), active_class),
            id: format_args!("{}", id.unwrap_or("")),
            title: format_args!("{}", title.unwrap_or("")),
            prevent_default: "{prevent_default}",
            target: format_args!("{}", if *new_tab { "_blank" } else { "" }),
            onclick: move |_| {
                if !outerlink {
                    if let Some(service) = svc {
                        service.push_route(to, cx.props.title.map(|f| f.to_string()), None);
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
