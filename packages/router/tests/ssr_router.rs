#![allow(non_snake_case)]

use dioxus_core::{self as dioxus, prelude::*};
use dioxus_core_macro::*;
use dioxus_html as dioxus_elements;
use dioxus_router::{
    history::{
        ControlledHistoryProvider, HistoryController, HistoryProvider, MemoryHistoryProvider,
    },
    prelude::*,
};

#[test]
fn generates_without_error() {
    let (mut c, history) = HistoryController::new(Box::new(MemoryHistoryProvider::default()));

    c.replace(String::from("/other"));

    let mut app = VirtualDom::new_with_props(App, AppProps { history });
    app.rebuild();

    let out = dioxus_ssr::render_vdom(&app);

    assert_eq!(out, "<nav>navbar</nav><h1>Other</h1>");
}

#[derive(Props)]
struct AppProps {
    history: ControlledHistoryProvider,
}

impl PartialEq for AppProps {
    fn eq(&self, _: &Self) -> bool {
        false
    }
}

fn App<'a>(cx: Scope<AppProps>) -> Element {
    let routes = cx.use_hook(|_| Segment {
        index: RcComponent(Home),
        fixed: vec![(
            String::from("other"),
            Route {
                content: RcComponent(Other),
                ..Default::default()
            },
        )],
        ..Default::default()
    });
    let history = cx.use_hook(|_| {
        let history = cx.props.history.clone();

        return move || {
            let x: Box<dyn HistoryProvider> = Box::new(history.clone());
            x
        };
    });

    cx.render(rsx! {
        Router {
            init_only: true,
            history: history,
            routes: routes,
            nav { "navbar" }
            Outlet {}
        }
    })
}

fn Home(cx: Scope) -> Element {
    cx.render(rsx! { h1 { "Home" } })
}

fn Other(cx: Scope) -> Element {
    cx.render(rsx! { h1 { "Other" }})
}
