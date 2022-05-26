use dioxus::prelude::*;
use dioxus_router::history::MemoryHistoryProvider;

use crate::{render, test_routes, ADDRESS};

#[test]
fn basic() {
    assert_eq!(
        r#"<a href="/" dioxus-prevent-default="onclick" class="" id="" rel="" target="">Test Link</a>"#,
        render(App)
    );
    #[allow(non_snake_case)]
    fn App(cx: Scope) -> Element {
        cx.render(rsx! {
            Router {
                routes: test_routes(&cx),
                init_only: true,
                Link {
                    target: NtPath(String::from("/")),
                    "Test Link"
                }
            }
        })
    }
}

#[test]
fn with_id() {
    assert_eq!(
        r#"<a href="/test/" dioxus-prevent-default="onclick" class="" id="test_id" rel="" target="">Test Link</a>"#,
        render(App)
    );

    #[allow(non_snake_case)]
    fn App(cx: Scope) -> Element {
        cx.render(rsx! {
            Router {
                routes: test_routes(&cx),
                init_only: true,
                Link {
                    target: NtName("test", vec![], QNone),
                    id: "test_id",
                    "Test Link"
                }
            }
        })
    }
}

#[test]
fn with_class() {
    assert_eq!(
        r#"<a href="/test/" dioxus-prevent-default="onclick" class="test_class" id="" rel="" target="">Test Link</a>"#,
        render(App)
    );

    #[allow(non_snake_case)]
    fn App(cx: Scope) -> Element {
        cx.render(rsx! {
            Router {
                init_only: true,
                routes: test_routes(&cx),
                Link {
                    target: NtName("test", vec![], QNone),
                    class: "test_class",
                    "Test Link"
                }
            }
        })
    }
}

#[test]
fn with_class_and_router_active() {
    assert_eq!(
        r#"<a href="/" dioxus-prevent-default="onclick" class="test_class active_router" id="" rel="" target="">Test Link</a>"#,
        render(App)
    );

    #[allow(non_snake_case)]
    fn App(cx: Scope) -> Element {
        cx.render(rsx! {
            Router {
                routes: test_routes(&cx),
                init_only: true,
                active_class: "active_router",
                Link {
                    target: NtName("root_index", vec![], QNone),
                    class: "test_class",
                    "Test Link"
                }
            }
        })
    }
}

#[test]
fn with_class_and_overridden_active() {
    assert_eq!(
        r#"<a href="/" dioxus-prevent-default="onclick" class="test_class active_link" id="" rel="" target="">Test Link</a>"#,
        render(App)
    );

    #[allow(non_snake_case)]
    fn App(cx: Scope) -> Element {
        cx.render(rsx! {
            Router {
                routes: test_routes(&cx),
                init_only: true,
                active_class: "active_router",
                Link {
                    target: NtName("root_index", vec![], QNone),
                    class: "test_class",
                    active_class: "active_link",
                    "Test Link"
                }
            }
        })
    }
}

#[test]
fn with_class_and_exact_active() {
    assert_eq!(
        format!(
            "{link1}{link2}{link3}{link4}",
            link1 = r#"<a href="/" dioxus-prevent-default="onclick" class="test_class_1" id="" rel="" target="">Test Link 1</a>"#,
            link2 = r#"<a href="/" dioxus-prevent-default="onclick" class="test_class_2 active" id="" rel="" target="">Test Link 2</a>"#,
            link3 = r#"<a href="/test/" dioxus-prevent-default="onclick" class="test_class_3 active" id="" rel="" target="">Test Link 3</a>"#,
            link4 = r#"<a href="/test/" dioxus-prevent-default="onclick" class="test_class_4 active" id="" rel="" target="">Test Link 4</a>"#,
        ),
        render(App)
    );

    #[allow(non_snake_case)]
    fn App(cx: Scope) -> Element {
        cx.render(rsx! {
            Router {
                routes: test_routes(&cx),
                init_only: true,
                active_class: "active",
                history: &|| MemoryHistoryProvider::with_first(String::from("/test/")),

                Link {
                    target: NtPath(String::from("/")),
                    exact: true,
                    class: "test_class_1",
                    "Test Link 1"
                }
                Link {
                    target: NtPath(String::from("/")),
                    class: "test_class_2",
                    "Test Link 2"
                }
                Link {
                    target: NtPath(String::from("/test/")),
                    exact:true,
                    class: "test_class_3",
                    "Test Link 3"
                }
                Link {
                    target: NtPath(String::from("/test/")),
                    class: "test_class_4",
                    "Test Link 4"
                }
            }
        })
    }
}

#[test]
fn with_new_tab() {
    assert_eq!(
        r#"<a href="/test/" dioxus-prevent-default="" class="" id="" rel="" target="_blank">Test Link</a>"#,
        render(App)
    );

    #[allow(non_snake_case)]
    fn App(cx: Scope) -> Element {
        cx.render(rsx! {
            Router {
                routes: test_routes(&cx),
                init_only: true,
                Link {
                    target: NtName("test", vec![], QNone),
                    new_tab: true,
                    "Test Link"
                }
            }
        })
    }
}

#[test]
fn with_rel() {
    assert_eq!(
        r#"<a href="/test/" dioxus-prevent-default="onclick" class="" id="" rel="custom" target="">Test Link</a>"#,
        render(App)
    );

    #[allow(non_snake_case)]
    fn App(cx: Scope) -> Element {
        cx.render(rsx! {
            Router {
                routes: test_routes(&cx),
                init_only: true,
                Link {
                    target: NtName("test", vec![], QNone),
                    rel: "custom",
                    "Test Link"
                }
            }
        })
    }
}

#[test]
fn with_external_target() {
    assert_eq!(
        format!(
            r#"<a href="{ADDRESS}" dioxus-prevent-default="" class="" id="" rel="noopener noreferrer" target="">Test Link</a>"#
        ),
        render(App)
    );

    #[allow(non_snake_case)]
    fn App(cx: Scope) -> Element {
        cx.render(rsx! {
            Router {
                routes: test_routes(&cx),
                init_only: true,
                Link {
                    target: NtExternal(ADDRESS.to_string()),
                    "Test Link"
                }
            }
        })
    }
}

#[test]
fn with_external_target_and_rel() {
    assert_eq!(
        format!(
            r#"<a href="{ADDRESS}" dioxus-prevent-default="" class="" id="" rel="custom" target="">Test Link</a>"#
        ),
        render(App)
    );

    #[allow(non_snake_case)]
    fn App(cx: Scope) -> Element {
        cx.render(rsx! {
            Router {
                routes: test_routes(&cx),
                init_only: true,
                Link {
                    target: NtExternal(ADDRESS.to_string()),
                    rel: "custom",
                    "Test Link"
                }
            }
        })
    }
}

#[cfg(debug_assertions)]
#[test]
#[should_panic = "`Link` can only be used as a descendent of a `Router`"]
fn without_router_panic_in_debug() {
    render(LinkWithoutRouter);
}

#[cfg(not(debug_assertions))]
#[test]
fn without_router_ignore_in_release() {
    render(LinkWithoutRouter);
}

#[allow(non_snake_case)]
fn LinkWithoutRouter(cx: Scope) -> Element {
    cx.render(rsx! {
        Link {
            target: NtPath(String::new()),
            "Test link"
        }
    })
}
