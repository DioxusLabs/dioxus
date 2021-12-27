mod utils;

use std::{cell::RefCell, rc::Rc};

use dioxus::Attribute;
use dioxus_core as dioxus;

use dioxus_core::prelude::*;
use dioxus_core_macro::{format_args_f, rsx, Props};
use dioxus_html as dioxus_elements;
// use wasm_bindgen::{JsCast, JsValue};

use crate::utils::strip_slash_suffix;

/// Initialize the app's router service and provide access to `Link` components
pub fn use_router<R: 'static>(cx: &ScopeState, f: impl Fn(&str) -> R) -> &R {
    let r = f("/");
    cx.use_hook(
        |_| {
            //
            r
        },
        |f| f,
    )
}

pub trait Routable: 'static + Send + Clone + PartialEq {}
impl<T> Routable for T where T: 'static + Send + Clone + PartialEq {}

#[derive(Props)]
pub struct LinkProps<'a, R: Routable> {
    to: R,

    /// The url that gets pushed to the history stack
    ///
    /// You can either put it your own inline method or just autoderive the route using `derive(Routable)`
    ///
    /// ```rust
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

    children: Element<'a>,

    #[props(default)]
    attributes: Option<&'a [Attribute<'a>]>,
}

pub fn Link<'a, R: Routable>(cx: Scope<'a, LinkProps<'a, R>>) -> Element {
    // let service = todo!();
    // let service: todo!() = use_router_service::<R>(&cx)?;
    let class = cx.props.class.unwrap_or("");
    cx.render(rsx! {
        a {
            href: "#",
            class: "{class}",
            {&cx.props.children}
            // onclick: move |_| service.push_route(cx.props.to.clone()),
            // href: format_args!("{}", (cx.props.href)(&cx.props.to)),
        }
    })
}
