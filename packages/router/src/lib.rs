mod utils;

use std::{cell::RefCell, collections::HashMap, rc::Rc};

use dioxus_core as dioxus;
use dioxus_core::prelude::*;
use dioxus_core_macro::{format_args_f, rsx, Props};
use dioxus_html as dioxus_elements;
use wasm_bindgen::JsValue;
use web_sys::Event;

use crate::utils::fetch_base_url;

pub struct RouterService<R: Routable> {
    history: RefCell<Vec<R>>,
    base_ur: RefCell<Option<String>>,
}

impl<R: Routable> RouterService<R> {
    fn push_route(&self, r: R) {
        self.history.borrow_mut().push(r);
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
pub fn use_router<R: Routable>(cx: Context, cfg: impl FnOnce(&str) -> R) -> Option<&R> {
    // for the web, attach to the history api
    cx.use_hook(
        |f| {
            //
            use gloo::events::EventListener;

            let base_url = fetch_base_url();

            let service: RouterService<R> = RouterService {
                history: RefCell::new(vec![]),
                base_ur: RefCell::new(base_url),
            };

            cx.provide_state(service);

            let regenerate = cx.schedule_update();

            // when "back" is called by the user, we want to to re-render the component
            let listener = EventListener::new(&web_sys::window().unwrap(), "popstate", move |_| {
                //
                regenerate();
            });
        },
        |f| {
            //
        },
    );

    todo!()
    // let router = use_router_service::<R>(cx)?;
    // Some(cfg(router.get_current_route()))
}

pub fn use_router_service<R: Routable>(cx: Context) -> Option<&Rc<RouterService<R>>> {
    cx.use_hook(|_| cx.consume_state::<RouterService<R>>(), |f| f.as_ref())
}

#[derive(Props)]
pub struct LinkProps<R: Routable> {
    to: R,
    children: Element,
}

pub fn Link<'a, R: Routable>(cx: Context, props: &LinkProps<R>) -> Element {
    let service = use_router_service::<R>(cx)?;
    cx.render(rsx! {
        a {
            href: format_args!("{}", props.to.to_path()),
            onclick: move |_| service.push_route(props.to.clone()),
            {&props.children},
        }
    })
}

pub trait Routable: Sized + Clone + 'static {
    /// Converts path to an instance of the routes enum.
    fn from_path(path: &str, params: &HashMap<&str, &str>) -> Option<Self>;

    /// Converts the route to a string that can passed to the history API.
    fn to_path(&self) -> String;

    /// Lists all the available routes
    fn routes() -> Vec<&'static str>;

    /// The route to redirect to on 404
    fn not_found_route() -> Option<Self>;

    /// Match a route based on the path
    fn recognize(pathname: &str) -> Option<Self>;
}
