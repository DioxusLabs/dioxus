#![allow(non_snake_case)]
use dioxus::prelude::*;
use dioxus_router::prelude::*;
use std::str::FromStr;

fn prepare<R: Routable>() -> String
where
    <R as FromStr>::Err: std::fmt::Display,
{
    let mut vdom = VirtualDom::new_with_props(
        App,
        AppProps::<R> {
            phantom: std::marker::PhantomData,
        },
    );
    let _ = vdom.rebuild();
    return dioxus_ssr::render(&vdom);

    #[derive(Props)]
    struct AppProps<R: Routable> {
        phantom: std::marker::PhantomData<R>,
    }

    impl<R: Routable> PartialEq for AppProps<R> {
        fn eq(&self, _other: &Self) -> bool {
            false
        }
    }

    fn App<R: Routable>(cx: Scope<AppProps<R>>) -> Element
    where
        <R as FromStr>::Err: std::fmt::Display,
    {
        render! {
            h1 { "App" }
            GenericRouter::<R> {
                config: || RouterConfig::default().history(MemoryHistory::default())
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

    #[inline_props]
    fn Test(cx: Scope) -> Element {
        todo!()
    }

    #[inline_props]
    fn Root(cx: Scope) -> Element {
        render! {
            Link {
                target: Route::Test {},
                "Link"
            }
        }
    }

    let expected = format!(
        "<h1>App</h1><a {href} {default} {class} {id} {rel} {target}>Link</a>",
        href = r#"href="/test""#,
        default = r#"dioxus-prevent-default="onclick""#,
        class = r#"class="""#,
        id = r#"id="""#,
        rel = r#"rel="""#,
        target = r#"target="""#
    );

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

    #[inline_props]
    fn Test(cx: Scope) -> Element {
        todo!()
    }

    #[inline_props]
    fn Root(cx: Scope) -> Element {
        render! {
            Link {
                target: NavigationTarget::External("https://dioxuslabs.com/".into()),
                "Link"
            }
        }
    }

    let expected = format!(
        "<h1>App</h1><a {href} {default} {class} {id} {rel} {target}>Link</a>",
        href = r#"href="https://dioxuslabs.com/""#,
        default = r#"dioxus-prevent-default="""#,
        class = r#"class="""#,
        id = r#"id="""#,
        rel = r#"rel="noopener noreferrer""#,
        target = r#"target="""#
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

    #[inline_props]
    fn Test(cx: Scope) -> Element {
        todo!()
    }

    #[inline_props]
    fn Root(cx: Scope) -> Element {
        render! {
            Link {
                target: Route::Test {},
                class: "test_class",
                "Link"
            }
        }
    }

    let expected = format!(
        "<h1>App</h1><a {href} {default} {class} {id} {rel} {target}>Link</a>",
        href = r#"href="/test""#,
        default = r#"dioxus-prevent-default="onclick""#,
        class = r#"class="test_class""#,
        id = r#"id="""#,
        rel = r#"rel="""#,
        target = r#"target="""#
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

    #[inline_props]
    fn Root(cx: Scope) -> Element {
        render! {
            Link {
                target: Route::Root {},
                active_class: "active_class",
                class: "test_class",
                "Link"
            }
        }
    }

    let expected = format!(
        "<h1>App</h1><a {href} {default} {class} {id} {rel} {target}>Link</a>",
        href = r#"href="/""#,
        default = r#"dioxus-prevent-default="onclick""#,
        class = r#"class="test_class active_class""#,
        id = r#"id="""#,
        rel = r#"rel="""#,
        target = r#"target="""#
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

    #[inline_props]
    fn Test(cx: Scope) -> Element {
        todo!()
    }

    #[inline_props]
    fn Root(cx: Scope) -> Element {
        render! {
            Link {
                target: Route::Test {},
                active_class: "active_class",
                class: "test_class",
                "Link"
            }
        }
    }

    let expected = format!(
        "<h1>App</h1><a {href} {default} {class} {id} {rel} {target}>Link</a>",
        href = r#"href="/test""#,
        default = r#"dioxus-prevent-default="onclick""#,
        class = r#"class="test_class""#,
        id = r#"id="""#,
        rel = r#"rel="""#,
        target = r#"target="""#
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

    #[inline_props]
    fn Test(cx: Scope) -> Element {
        todo!()
    }

    #[inline_props]
    fn Root(cx: Scope) -> Element {
        render! {
            Link {
                target: Route::Test {},
                id: "test_id",
                "Link"
            }
        }
    }

    let expected = format!(
        "<h1>App</h1><a {href} {default} {class} {id} {rel} {target}>Link</a>",
        href = r#"href="/test""#,
        default = r#"dioxus-prevent-default="onclick""#,
        class = r#"class="""#,
        id = r#"id="test_id""#,
        rel = r#"rel="""#,
        target = r#"target="""#
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

    #[inline_props]
    fn Test(cx: Scope) -> Element {
        todo!()
    }

    #[inline_props]
    fn Root(cx: Scope) -> Element {
        render! {
            Link {
                target: Route::Test {},
                new_tab: true,
                "Link"
            }
        }
    }

    let expected = format!(
        "<h1>App</h1><a {href} {default} {class} {id} {rel} {target}>Link</a>",
        href = r#"href="/test""#,
        default = r#"dioxus-prevent-default="""#,
        class = r#"class="""#,
        id = r#"id="""#,
        rel = r#"rel="""#,
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

    #[inline_props]
    fn Root(cx: Scope) -> Element {
        render! {
            Link {
                target: NavigationTarget::External("https://dioxuslabs.com/".into()),
                new_tab: true,
                "Link"
            }
        }
    }

    let expected = format!(
        "<h1>App</h1><a {href} {default} {class} {id} {rel} {target}>Link</a>",
        href = r#"href="https://dioxuslabs.com/""#,
        default = r#"dioxus-prevent-default="""#,
        class = r#"class="""#,
        id = r#"id="""#,
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

    #[inline_props]
    fn Test(cx: Scope) -> Element {
        todo!()
    }

    #[inline_props]
    fn Root(cx: Scope) -> Element {
        render! {
            Link {
                target: Route::Test {},
                rel: "test_rel",
                "Link"
            }
        }
    }

    let expected = format!(
        "<h1>App</h1><a {href} {default} {class} {id} {rel} {target}>Link</a>",
        href = r#"href="/test""#,
        default = r#"dioxus-prevent-default="onclick""#,
        class = r#"class="""#,
        id = r#"id="""#,
        rel = r#"rel="test_rel""#,
        target = r#"target="""#
    );

    assert_eq!(prepare::<Route>(), expected);
}
