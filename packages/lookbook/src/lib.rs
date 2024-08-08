use std::cell::RefCell;

use dioxus::prelude::*;
use dioxus_router::prelude::*;

pub use lookbook_macros::preview;

mod control;
pub use control::{Control, Json};

mod ui;
use ui::Wrap;
pub use ui::{Look, LookBook};

mod prefixed_route;
pub(crate) use prefixed_route::PrefixedRoute;

#[doc(hidden)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Preview {
    name: &'static str,
    component: Component,
}

impl Preview {
    pub const fn new(name: &'static str, component: Component) -> Self {
        Self { name, component }
    }
}

thread_local! {
    static CONTEXT: RefCell<Vec<(&'static str, Component)>>= RefCell::new(Vec::new());

    static HOME: RefCell<Option<Component>> = RefCell::new(None);
}

fn register(name: &'static str, component: Component) {
    CONTEXT
        .try_with(|cx| cx.borrow_mut().push((name, component)))
        .unwrap();
}

#[derive(Clone, Routable, Debug, PartialEq)]
enum Route {
    #[layout(Wrap)]
    #[route("/")]
    Home,
    #[route("/:name")]
    ComponentScreen { name: String },
}

#[component]
fn Home() -> Element {
    #[allow(non_snake_case)]
    let Child = HOME
        .try_with(|cell| cell.borrow().clone().unwrap())
        .unwrap();
    rsx!(Child {})
}

#[component]
fn ComponentScreen(name: String) -> Element {
    #[allow(non_snake_case)]
    if let Some((_name, Child)) = CONTEXT
        .try_with(|cx| cx.borrow().iter().find(|(n, _)| *n == name).cloned())
        .unwrap()
    {
        rsx!(Child {})
    } else {
        // TODO
        rsx!(div {})
    }
}
