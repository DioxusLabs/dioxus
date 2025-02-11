use dioxus::prelude::*;
use dioxus_history::{History, MemoryHistory};
use dioxus_router::components::HistoryProvider;
use std::{rc::Rc, str::FromStr};

fn prepare<R: Routable>() -> String
where
    <R as FromStr>::Err: std::fmt::Display,
{
    prepare_at::<R>("/")
}

fn prepare_at<R: Routable>(at: impl ToString) -> String
where
    <R as FromStr>::Err: std::fmt::Display,
{
    let mut vdom = VirtualDom::new_with_props(
        App,
        AppProps::<R> {
            at: at.to_string(),
            phantom: std::marker::PhantomData,
        },
    );
    vdom.rebuild_in_place();
    return dioxus_ssr::render(&vdom);

    #[derive(Props)]
    struct AppProps<R: Routable> {
        at: String,
        phantom: std::marker::PhantomData<R>,
    }

    impl<R: Routable> Clone for AppProps<R> {
        fn clone(&self) -> Self {
            Self {
                at: self.at.clone(),
                phantom: std::marker::PhantomData,
            }
        }
    }

    impl<R: Routable> PartialEq for AppProps<R> {
        fn eq(&self, _other: &Self) -> bool {
            false
        }
    }

    #[allow(non_snake_case)]
    fn App<R: Routable>(props: AppProps<R>) -> Element
    where
        <R as FromStr>::Err: std::fmt::Display,
    {
        rsx! {
            h1 { "App" }
            HistoryProvider {
                history:  move |_| Rc::new(MemoryHistory::with_initial_path(props.at.clone())) as Rc<dyn History>,
                Router::<R> {}
            }
        }
    }
}

#[test]
fn href_internal() {
    #[derive(Routable, Clone)]
    enum Route {
        #[route("/")]
        Root {},
        #[route("/test")]
        Test {},
    }

    #[component]
    fn Test() -> Element {
        unimplemented!()
    }

    #[component]
    fn Root() -> Element {
        rsx! {
            Link {
                to: Route::Test {},
                "Link"
            }
        }
    }

    let expected = format!("<h1>App</h1><a {href}>Link</a>", href = r#"href="/test""#,);

    assert_eq!(prepare::<Route>(), expected);
}

#[test]
fn href_external() {
    #[derive(Routable, Clone)]
    enum Route {
        #[route("/")]
        Root {},
        #[route("/test")]
        Test {},
    }

    #[component]
    fn Test() -> Element {
        unimplemented!()
    }

    #[component]
    fn Root() -> Element {
        rsx! {
            Link {
                to: "https://dioxuslabs.com/",
                "Link"
            }
        }
    }

    let expected = format!(
        "<h1>App</h1><a {href} {rel}>Link</a>",
        href = r#"href="https://dioxuslabs.com/""#,
        rel = r#"rel="noopener noreferrer""#,
    );

    assert_eq!(prepare::<Route>(), expected);
}

#[test]
fn with_class() {
    #[derive(Routable, Clone)]
    enum Route {
        #[route("/")]
        Root {},
        #[route("/test")]
        Test {},
    }

    #[component]
    fn Test() -> Element {
        unimplemented!()
    }

    #[component]
    fn Root() -> Element {
        rsx! {
            Link {
                to: Route::Test {},
                class: "test_class",
                "Link"
            }
        }
    }

    let expected = format!(
        "<h1>App</h1><a {href} {class}>Link</a>",
        href = r#"href="/test""#,
        class = r#"class="test_class""#,
    );

    assert_eq!(prepare::<Route>(), expected);
}

#[test]
fn with_active_class_active() {
    #[derive(Routable, Clone)]
    enum Route {
        #[route("/")]
        Root {},
    }

    #[component]
    fn Root() -> Element {
        rsx! {
            Link {
                to: Route::Root {},
                active_class: "active_class".to_string(),
                class: "test_class",
                "Link"
            }
        }
    }

    let expected = format!(
        "<h1>App</h1><a {href} {class} {aria}>Link</a>",
        href = r#"href="/""#,
        class = r#"class="test_class active_class""#,
        aria = r#"aria-current="page""#,
    );

    assert_eq!(prepare::<Route>(), expected);
}

#[test]
fn with_active_class_inactive() {
    #[derive(Routable, Clone)]
    enum Route {
        #[route("/")]
        Root {},
        #[route("/test")]
        Test {},
    }

    #[component]
    fn Test() -> Element {
        unimplemented!()
    }

    #[component]
    fn Root() -> Element {
        rsx! {
            Link {
                to: Route::Test {},
                active_class: "active_class".to_string(),
                class: "test_class",
                "Link"
            }
        }
    }

    let expected = format!(
        "<h1>App</h1><a {href} {class}>Link</a>",
        href = r#"href="/test""#,
        class = r#"class="test_class""#,
    );

    assert_eq!(prepare::<Route>(), expected);
}

#[test]
fn with_id() {
    #[derive(Routable, Clone)]
    enum Route {
        #[route("/")]
        Root {},
        #[route("/test")]
        Test {},
    }

    #[component]
    fn Test() -> Element {
        unimplemented!()
    }

    #[component]
    fn Root() -> Element {
        rsx! {
            Link {
                to: Route::Test {},
                id: "test_id",
                "Link"
            }
        }
    }

    let expected = format!(
        "<h1>App</h1><a {href} {id}>Link</a>",
        href = r#"href="/test""#,
        id = r#"id="test_id""#,
    );

    assert_eq!(prepare::<Route>(), expected);
}

#[test]
fn with_new_tab() {
    #[derive(Routable, Clone)]
    enum Route {
        #[route("/")]
        Root {},
        #[route("/test")]
        Test {},
    }

    #[component]
    fn Test() -> Element {
        unimplemented!()
    }

    #[component]
    fn Root() -> Element {
        rsx! {
            Link {
                to: Route::Test {},
                new_tab: true,
                "Link"
            }
        }
    }

    let expected = format!(
        "<h1>App</h1><a {href} {target}>Link</a>",
        href = r#"href="/test""#,
        target = r#"target="_blank""#
    );

    assert_eq!(prepare::<Route>(), expected);
}

#[test]
fn with_new_tab_external() {
    #[derive(Routable, Clone)]
    enum Route {
        #[route("/")]
        Root {},
    }

    #[component]
    fn Root() -> Element {
        rsx! {
            Link {
                to: "https://dioxuslabs.com/",
                new_tab: true,
                "Link"
            }
        }
    }

    let expected = format!(
        "<h1>App</h1><a {href} {rel} {target}>Link</a>",
        href = r#"href="https://dioxuslabs.com/""#,
        rel = r#"rel="noopener noreferrer""#,
        target = r#"target="_blank""#
    );

    assert_eq!(prepare::<Route>(), expected);
}

#[test]
fn with_rel() {
    #[derive(Routable, Clone)]
    enum Route {
        #[route("/")]
        Root {},
        #[route("/test")]
        Test {},
    }

    #[component]
    fn Test() -> Element {
        unimplemented!()
    }

    #[component]
    fn Root() -> Element {
        rsx! {
            Link {
                to: Route::Test {},
                rel: "test_rel".to_string(),
                "Link"
            }
        }
    }

    let expected = format!(
        "<h1>App</h1><a {href} {rel}>Link</a>",
        href = r#"href="/test""#,
        rel = r#"rel="test_rel""#,
    );

    assert_eq!(prepare::<Route>(), expected);
}

#[test]
fn with_child_route() {
    #[derive(Routable, Clone, PartialEq, Debug)]
    enum ChildRoute {
        #[route("/")]
        ChildRoot {},
        #[route("/:not_static")]
        NotStatic { not_static: String },
    }

    #[derive(Routable, Clone, PartialEq, Debug)]
    enum Route {
        #[route("/")]
        Root {},
        #[route("/test")]
        Test {},
        #[child("/child")]
        Nested { child: ChildRoute },
    }

    #[component]
    fn Test() -> Element {
        unimplemented!()
    }

    #[component]
    fn Root() -> Element {
        rsx! {
            Link {
                to: Route::Test {},
                "Parent Link"
            }
            Link {
                to: Route::Nested { child: ChildRoute::NotStatic { not_static: "this-is-a-child-route".to_string() } },
                "Child Link"
            }
        }
    }

    #[component]
    fn ChildRoot() -> Element {
        rsx! {
            Link {
                to: Route::Test {},
                "Parent Link"
            }
            Link {
                to: ChildRoute::NotStatic { not_static: "this-is-a-child-route".to_string() },
                "Child Link 1"
            }
            Link {
                to: Route::Nested { child: ChildRoute::NotStatic { not_static: "this-is-a-child-route".to_string() } },
                "Child Link 2"
            }
        }
    }

    #[component]
    fn NotStatic(not_static: String) -> Element {
        unimplemented!()
    }

    assert_eq!(
        prepare_at::<Route>("/"),
        "<h1>App</h1><a href=\"/test\">Parent Link</a><a href=\"/child/this-is-a-child-route\">Child Link</a>"
    );

    assert_eq!(
        prepare_at::<Route>("/child"),
        "<h1>App</h1><a href=\"/test\">Parent Link</a><a href=\"/child/this-is-a-child-route\">Child Link 1</a><a href=\"/child/this-is-a-child-route\">Child Link 2</a>"
    );
}

#[test]
fn with_hash_segment() {
    #[derive(Routable, Clone)]
    enum Route {
        #[route("/#:data")]
        Root { data: String },
    }

    #[component]
    fn Root(data: String) -> Element {
        rsx! {
            Link {
                to: Route::Root { data: "test".to_string() },
                "Link"
            }
            Link {
                to: Route::Root { data: "".to_string() },
                "Empty"
            }
        }
    }

    assert_eq!(
        prepare_at::<Route>("/#test"),
        "<h1>App</h1><a href=\"/#test\" aria-current=\"page\">Link</a><a href=\"/\">Empty</a>"
    );
}
