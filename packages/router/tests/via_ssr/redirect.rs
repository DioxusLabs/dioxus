use dioxus::prelude::*;
use dioxus_history::{History, MemoryHistory};
use dioxus_router::components::HistoryProvider;
use std::{rc::Rc, str::FromStr};

// Tests for regressions of <https://github.com/DioxusLabs/dioxus/issues/2549>
#[test]
fn redirects_apply_in_order() {
    let path = Route::from_str("/").unwrap();
    assert_eq!(
        path,
        Route::Home {
            lang: "en".to_string()
        }
    );
    let mut vdom = VirtualDom::new_with_props(App, AppProps { path });
    vdom.rebuild_in_place();
    let as_string = dioxus_ssr::render(&vdom);
    assert_eq!(as_string, "en");
}

#[derive(Clone, Routable, Debug, PartialEq)]
enum Route {
    // The redirect should try to parse first because it is placed first in the enum
    #[redirect("/", || Route::Home { lang: "en".to_string() })]
    #[route("/?:lang")]
    Home { lang: String },
}

#[component]
fn Home(lang: String) -> Element {
    rsx! { "{lang}" }
}

#[component]
fn App(path: Route) -> Element {
    rsx! {
        HistoryProvider {
            history:  move |_| Rc::new(MemoryHistory::with_initial_path(path.clone())) as Rc<dyn History>,
            Router::<Route> {}
        }
    }
}
