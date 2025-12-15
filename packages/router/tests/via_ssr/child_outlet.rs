#![allow(unused)]

use std::rc::Rc;

use dioxus::prelude::*;
use dioxus_history::{History, MemoryHistory};
use dioxus_router::components::HistoryProvider;

fn prepare(path: impl Into<String>) -> VirtualDom {
    let mut vdom = VirtualDom::new_with_props(
        App,
        AppProps {
            path: path.into().parse().unwrap(),
        },
    );
    vdom.rebuild_in_place();
    return vdom;

    #[derive(Routable, Clone, PartialEq)]
    #[rustfmt::skip]
    enum Route {
        #[layout(Layout)]
            #[child("/")]
            Child { child: ChildRoute },
    }

    #[derive(Routable, Clone, PartialEq)]
    #[rustfmt::skip]
    enum ChildRoute{
        #[layout(ChildLayout)]
            #[route("/")]
            RootIndex {}
    }

    #[component]
    fn App(path: Route) -> Element {
        rsx! {
            h1 { "App" }
            HistoryProvider {
                history:  move |_| Rc::new(MemoryHistory::with_initial_path(path.clone())) as Rc<dyn History>,
                Router::<Route> {}
            }
        }
    }

    #[component]
    fn RootIndex() -> Element {
        rsx! { h2 { "Root Index" } }
    }

    #[component]
    fn Layout() -> Element {
        rsx! {
            h2 { "parent layout" }
            Outlet::<Route> { }
        }
    }

    #[component]
    fn ChildLayout() -> Element {
        rsx! {
            h2 { "child layout" }
            Outlet::<ChildRoute> { }
        }
    }
}

#[test]
fn root_index() {
    let vdom = prepare("/");
    let html = dioxus_ssr::render(&vdom);

    assert_eq!(
        html,
        "<h1>App</h1><h2>parent layout</h2><h2>child layout</h2><h2>Root Index</h2>"
    );
}
