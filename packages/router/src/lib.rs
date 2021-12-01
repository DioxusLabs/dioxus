mod utils;

use std::{cell::RefCell, rc::Rc};

use dioxus_core as dioxus;
use dioxus_core::prelude::*;
use dioxus_core_macro::{rsx, Props};
use dioxus_html as dioxus_elements;
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{window, Event};

use crate::utils::strip_slash_suffix;

pub trait Routable: 'static + Send + Clone + PartialEq {}
impl<T> Routable for T where T: 'static + Send + Clone + PartialEq {}

pub struct RouterService<R: Routable> {
    historic_routes: Vec<R>,
    history_service: RefCell<web_sys::History>,
    base_ur: RefCell<Option<String>>,
}

impl<R: Routable> RouterService<R> {
    fn push_route(&self, r: R) {
        todo!()
        // self.historic_routes.borrow_mut().push(r);
    }

    fn get_current_route(&self) -> &str {
        todo!()
    }

    fn update_route_impl(&self, url: String, push: bool) {
        let history = web_sys::window().unwrap().history().expect("no history");
        let base = self.base_ur.borrow();
        let path = match base.as_ref() {
            Some(base) => {
                let path = format!("{}{}", base, url);
                if path.is_empty() {
                    "/".to_string()
                } else {
                    path
                }
            }
            None => url,
        };

        if push {
            history
                .push_state_with_url(&JsValue::NULL, "", Some(&path))
                .expect("push history");
        } else {
            history
                .replace_state_with_url(&JsValue::NULL, "", Some(&path))
                .expect("replace history");
        }
        let event = Event::new("popstate").unwrap();

        web_sys::window()
            .unwrap()
            .dispatch_event(&event)
            .expect("dispatch");
    }
}

/// This hould only be used once per app
///
/// You can manually parse the route if you want, but the derived `parse` method on `Routable` will also work just fine
pub fn use_router<R: Routable>(cx: Context, mut parse: impl FnMut(&str) -> R + 'static) -> &R {
    // for the web, attach to the history api
    cx.use_hook(
        |f| {
            //
            use gloo::events::EventListener;

            let base = window()
                .unwrap()
                .document()
                .unwrap()
                .query_selector("base[href]")
                .ok()
                .flatten()
                .and_then(|base| {
                    let base = JsCast::unchecked_into::<web_sys::HtmlBaseElement>(base).href();
                    let url = web_sys::Url::new(&base).unwrap();

                    if url.pathname() != "/" {
                        Some(strip_slash_suffix(&base).to_string())
                    } else {
                        None
                    }
                });

            let location = window().unwrap().location();
            let pathname = location.pathname().unwrap();
            let initial_route = parse(&pathname);

            let service: RouterService<R> = RouterService {
                historic_routes: vec![initial_route],
                history_service: RefCell::new(
                    web_sys::window().unwrap().history().expect("no history"),
                ),
                base_ur: RefCell::new(base),
            };

            // let base = base_url();
            // let url = route.to_path();
            // pending_routes: RefCell::new(vec![]),
            // service.history_service.push_state(data, title);

            // cx.provide_state(service);

            let regenerate = cx.schedule_update();

            // // when "back" is called by the user, we want to to re-render the component
            let listener = EventListener::new(&web_sys::window().unwrap(), "popstate", move |_| {
                //
                regenerate();
            });

            service
        },
        |state| {
            let base = state.base_ur.borrow();
            if let Some(base) = base.as_ref() {
                //
                let path = format!("{}{}", base, state.get_current_route());
            }
            let history = state.history_service.borrow();

            state.historic_routes.last().unwrap()
        },
    )
}

pub fn use_router_service<R: Routable>(cx: Context) -> Option<&Rc<RouterService<R>>> {
    cx.use_hook(|_| cx.consume_state::<RouterService<R>>(), |f| f.as_ref())
}

#[derive(Props)]
pub struct LinkProps<R: Routable> {
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
    children: Element,
}

pub fn Link<R: Routable>(cx: Context, props: &LinkProps<R>) -> Element {
    let service = use_router_service::<R>(cx)?;
    cx.render(rsx! {
        a {
            href: format_args!("{}", (props.href)(&props.to)),
            onclick: move |_| service.push_route(props.to.clone()),
            {&props.children},
        }
    })
}
