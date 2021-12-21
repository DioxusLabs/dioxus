mod utils;

use std::{cell::RefCell, rc::Rc};

use dioxus_core as dioxus;
use dioxus_core::prelude::*;
use dioxus_core_macro::{rsx, Props};
use dioxus_html as dioxus_elements;
// use wasm_bindgen::{JsCast, JsValue};

use crate::utils::strip_slash_suffix;

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
    href: fn(&R) -> String,

    #[builder(default)]
    children: Element<'a>,
}

pub fn Link<'a, R: Routable>(cx: Scope<'a, LinkProps<'a, R>>) -> Element {
    let service = todo!();
    // let service: todo!() = use_router_service::<R>(&cx)?;
    // cx.render(rsx! {
    //     a {
    //         href: format_args!("{}", (cx.props.href)(&cx.props.to)),
    //         onclick: move |_| service.push_route(cx.props.to.clone()),
    //         // todo!() {&cx.props.children},
    //     }
    // })
}
