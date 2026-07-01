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

mod dynamic_prefix {
    use super::*;

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
            #[child("/file/:file_id")]
            File { file_id: String, child: ChildRoute },
        }

        #[derive(Routable, Clone, PartialEq)]
        #[rustfmt::skip]
        enum ChildRoute {
            #[route("/view")]
            View {},
        }

        #[component]
        fn App(path: Route) -> Element {
            rsx! {
                h1 { "App" }
                HistoryProvider {
                    history: move |_| Rc::new(MemoryHistory::with_initial_path(path.clone())) as Rc<dyn History>,
                    Router::<Route> {}
                }
            }
        }

        #[component]
        fn View() -> Element {
            rsx! { h2 { "view" } }
        }
    }

    #[test]
    fn renders_under_dynamic_parent_prefix() {
        let vdom = prepare("/file/abc/view");
        let html = dioxus_ssr::render(&vdom);

        assert_eq!(html, "<h1>App</h1><h2>view</h2>");
    }
}

mod depth_2_chain {
    use super::*;

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
        enum OuterRoute {
            #[child("/host/:host_id")]
            Host { host_id: String, child: MidRoute },
        }

        #[derive(Routable, Clone, PartialEq)]
        #[rustfmt::skip]
        enum MidRoute {
            #[child("/mount")]
            Mount { child: InnerRoute },
        }

        #[derive(Routable, Clone, PartialEq)]
        #[rustfmt::skip]
        enum InnerRoute {
            #[route("/:item_id")]
            Item { item_id: String },
        }

        #[component]
        fn App(path: OuterRoute) -> Element {
            rsx! {
                h1 { "App" }
                HistoryProvider {
                    history: move |_| Rc::new(MemoryHistory::with_initial_path(path.clone())) as Rc<dyn History>,
                    Router::<OuterRoute> {}
                }
            }
        }

        #[component]
        fn Item(item_id: String) -> Element {
            rsx! { h2 { "item={item_id}" } }
        }
    }

    #[test]
    fn chain_preserves_deepest_dynamic() {
        let vdom = prepare("/host/H1/mount/X7");
        let html = dioxus_ssr::render(&vdom);

        assert_eq!(html, "<h1>App</h1><h2>item=X7</h2>");
    }
}

mod link_roundtrip_at_depth_2 {
    use super::*;

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
        enum OuterRoute {
            #[child("/host/:host_id")]
            Host { host_id: String, child: MidRoute },
        }

        #[derive(Routable, Clone, PartialEq)]
        #[rustfmt::skip]
        enum MidRoute {
            #[route("/")]
            Index {},
            #[route("/item/:item_id")]
            Item { item_id: String },
        }

        #[component]
        fn App(path: OuterRoute) -> Element {
            rsx! {
                HistoryProvider {
                    history: move |_| Rc::new(MemoryHistory::with_initial_path(path.clone())) as Rc<dyn History>,
                    Router::<OuterRoute> {}
                }
            }
        }

        #[component]
        fn Index() -> Element {
            rsx! {
                Link {
                    to: MidRoute::Item { item_id: "X7".to_string() },
                    "go"
                }
            }
        }

        #[component]
        fn Item(item_id: String) -> Element {
            rsx! { "item={item_id}" }
        }
    }

    #[test]
    fn link_href_includes_parent_dynamic() {
        let vdom = prepare("/host/H1/");
        let html = dioxus_ssr::render(&vdom);

        assert!(
            html.contains("/host/H1/item/X7"),
            "expected captured parent host_id in link href; got: {}",
            html
        );
    }
}

mod depth_3_chain {
    use super::*;

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
        enum OuterRoute {
            #[child("/host/:host_id")]
            Host { host_id: String, child: MidRoute },
        }

        #[derive(Routable, Clone, PartialEq)]
        #[rustfmt::skip]
        enum MidRoute {
            #[child("/mount/:mount_id")]
            Mount { mount_id: String, child: InnerRoute },
        }

        #[derive(Routable, Clone, PartialEq)]
        #[rustfmt::skip]
        enum InnerRoute {
            #[child("/items")]
            Items { child: LeafRoute },
        }

        #[derive(Routable, Clone, PartialEq)]
        #[rustfmt::skip]
        enum LeafRoute {
            #[route("/:item_id")]
            Item { item_id: String },
        }

        #[component]
        fn App(path: OuterRoute) -> Element {
            rsx! {
                h1 { "App" }
                HistoryProvider {
                    history: move |_| Rc::new(MemoryHistory::with_initial_path(path.clone())) as Rc<dyn History>,
                    Router::<OuterRoute> {}
                }
            }
        }

        #[component]
        fn Item(item_id: String) -> Element {
            rsx! { h2 { "item={item_id}" } }
        }
    }

    #[test]
    fn two_chain_hops_preserve_deepest_dynamic() {
        let vdom = prepare("/host/H1/mount/M9/items/X7");
        let html = dioxus_ssr::render(&vdom);

        assert_eq!(html, "<h1>App</h1><h2>item=X7</h2>");
    }
}

mod link_roundtrip_at_depth_3 {
    use super::*;

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
        enum OuterRoute {
            #[child("/host/:host_id")]
            Host { host_id: String, child: MidRoute },
        }

        #[derive(Routable, Clone, PartialEq)]
        #[rustfmt::skip]
        enum MidRoute {
            #[child("/mount/:mount_id")]
            Mount { mount_id: String, child: InnerRoute },
        }

        #[derive(Routable, Clone, PartialEq)]
        #[rustfmt::skip]
        enum InnerRoute {
            #[route("/")]
            Index {},
            #[route("/item/:item_id")]
            Item { item_id: String },
        }

        #[component]
        fn App(path: OuterRoute) -> Element {
            rsx! {
                HistoryProvider {
                    history: move |_| Rc::new(MemoryHistory::with_initial_path(path.clone())) as Rc<dyn History>,
                    Router::<OuterRoute> {}
                }
            }
        }

        #[component]
        fn Index() -> Element {
            rsx! {
                Link {
                    to: InnerRoute::Item { item_id: "X7".to_string() },
                    "go"
                }
            }
        }

        #[component]
        fn Item(item_id: String) -> Element {
            rsx! { "item={item_id}" }
        }
    }

    #[test]
    fn link_href_includes_two_parent_dynamics() {
        let vdom = prepare("/host/H1/mount/M9/");
        let html = dioxus_ssr::render(&vdom);

        assert!(
            html.contains("/host/H1/mount/M9/item/X7"),
            "expected both captured parent dynamics in link href; got: {}",
            html
        );
    }
}
