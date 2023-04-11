use dioxus::prelude::*;
use dioxus_router::prelude::*;

fn prepare(link: Component) -> String {
    #![allow(non_snake_case)]

    let mut vdom = VirtualDom::new_with_props(App, AppProps { link });
    let _ = vdom.rebuild();
    return dioxus_ssr::render(&vdom);

    #[derive(Props)]
    struct AppProps {
        link: Component,
    }

    impl PartialEq for AppProps {
        fn eq(&self, other: &Self) -> bool {
            false
        }
    }

    fn App(cx: Scope<AppProps>) -> Element {
        use_router(
            cx,
            &|| RouterConfiguration {
                synchronous: true,
                ..Default::default()
            },
            &|| Segment::content(comp(cx.props.link)),
        );

        render! {
            h1 { "App" }
            Outlet { }
        }
    }
}

#[test]
fn href_internal() {
    fn content(cx: Scope) -> Element {
        render! {
            Link {
                target: "/test",
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

    assert_eq!(prepare(content), expected);
}

#[test]
fn href_named() {
    fn content(cx: Scope) -> Element {
        render! {
            Link {
                target: named::<RootIndex>(),
                "Link"
            }
        }
    }

    let expected = format!(
        "<h1>App</h1><a {href} {default} {class} {id} {rel} {target}>Link</a>",
        href = r#"href="/""#,
        default = r#"dioxus-prevent-default="onclick""#,
        class = r#"class="""#,
        id = r#"id="""#,
        rel = r#"rel="""#,
        target = r#"target="""#
    );

    assert_eq!(prepare(content), expected);
}

#[test]
fn href_external() {
    fn content(cx: Scope) -> Element {
        render! {
            Link {
                target: "https://dioxuslabs.com/",
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

    assert_eq!(prepare(content), expected);
}

#[test]
fn with_class() {
    fn content(cx: Scope) -> Element {
        render! {
            Link {
                target: "/test",
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

    assert_eq!(prepare(content), expected);
}

#[test]
fn with_active_class_active() {
    fn content(cx: Scope) -> Element {
        render! {
            Link {
                target: "/",
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

    assert_eq!(prepare(content), expected);
}

#[test]
fn with_active_class_inactive() {
    fn content(cx: Scope) -> Element {
        render! {
            Link {
                target: "/test",
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

    assert_eq!(prepare(content), expected);
}

#[test]
fn with_id() {
    fn content(cx: Scope) -> Element {
        render! {
            Link {
                target: "/test",
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

    assert_eq!(prepare(content), expected);
}

#[test]
fn with_new_tab() {
    fn content(cx: Scope) -> Element {
        render! {
            Link {
                target: "/test",
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

    assert_eq!(prepare(content), expected);
}

#[test]
fn with_new_tab_external() {
    fn content(cx: Scope) -> Element {
        render! {
            Link {
                target: "https://dioxuslabs.com/",
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

    assert_eq!(prepare(content), expected);
}

#[test]
fn with_rel() {
    fn content(cx: Scope) -> Element {
        render! {
            Link {
                target: "/test",
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

    assert_eq!(prepare(content), expected);
}
