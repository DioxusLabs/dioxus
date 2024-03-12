#![allow(non_snake_case)]

use std::rc::Rc;

use dioxus::prelude::*;
use dioxus_core::NoOpMutations;

/// This test checks that we should release all memory used by the virtualdom when it exits.
///
/// When miri runs, it'll let us know if we leaked or aliased.
#[test]
fn test_memory_leak() {
    fn app() -> Element {
        let val = generation();

        spawn(async {});

        if val == 2 || val == 4 {
            return rsx!({});
        }

        let mut name = use_hook(|| String::from("numbers: "));

        name.push_str("123 ");

        rsx!(
            div { "Hello, world!" }
            Child {}
            Child {}
            Child {}
            Child {}
            Child {}
            Child {}
            BorrowedChild { name: name.clone() }
            BorrowedChild { name: name.clone() }
            BorrowedChild { name: name.clone() }
            BorrowedChild { name: name.clone() }
            BorrowedChild { name: name.clone() }
        )
    }

    #[derive(Props, Clone, PartialEq)]
    struct BorrowedProps {
        name: String,
    }

    fn BorrowedChild(cx: BorrowedProps) -> Element {
        rsx! {
            div {
                "goodbye {cx.name}"
                Child {}
                Child {}
            }
        }
    }

    fn Child() -> Element {
        rsx!( div { "goodbye world" } )
    }

    let mut dom = VirtualDom::new(app);

    dom.rebuild(&mut dioxus_core::NoOpMutations);

    for _ in 0..5 {
        dom.mark_dirty(ScopeId::ROOT);
        _ = dom.render_immediate_to_vec();
    }
}

#[test]
fn memo_works_properly() {
    fn app() -> Element {
        let val = generation();

        if val == 2 || val == 4 {
            return None;
        }

        let name = use_hook(|| String::from("asd"));

        rsx!(
            div { "Hello, world! {name}" }
            Child { na: "asdfg".to_string() }
        )
    }

    #[derive(PartialEq, Clone, Props)]
    struct ChildProps {
        na: String,
    }

    fn Child(_props: ChildProps) -> Element {
        rsx!( div { "goodbye world" } )
    }

    let mut dom = VirtualDom::new(app);

    dom.rebuild(&mut dioxus_core::NoOpMutations);
}

#[test]
fn free_works_on_root_hooks() {
    /*
    On Drop, scopearena drops all the hook contents. and props
    */
    #[derive(PartialEq, Clone, Props)]
    struct AppProps {
        inner: Rc<String>,
    }

    fn app(cx: AppProps) -> Element {
        let name: AppProps = use_hook(|| cx.clone());
        rsx!(child_component { inner: name.inner.clone() })
    }

    fn child_component(props: AppProps) -> Element {
        rsx!( div { "{props.inner}" } )
    }

    let ptr = Rc::new("asdasd".to_string());
    let mut dom = VirtualDom::new_with_props(app, AppProps { inner: ptr.clone() });
    dom.rebuild(&mut dioxus_core::NoOpMutations);

    // ptr gets cloned into props and then into the hook
    assert_eq!(Rc::strong_count(&ptr), 5);

    drop(dom);

    assert_eq!(Rc::strong_count(&ptr), 1);
}

#[test]
fn supports_async() {
    use std::time::Duration;
    use tokio::time::sleep;

    fn app() -> Element {
        let mut colors = use_signal(|| vec!["green", "blue", "red"]);
        let mut padding = use_signal(|| 10);

        use_hook(|| {
            spawn(async move {
                loop {
                    sleep(Duration::from_millis(1000)).await;
                    colors.with_mut(|colors| colors.reverse());
                }
            })
        });

        use_hook(|| {
            spawn(async move {
                loop {
                    sleep(Duration::from_millis(10)).await;
                    padding.with_mut(|padding| {
                        if *padding < 65 {
                            *padding += 1;
                        } else {
                            *padding = 5;
                        }
                    });
                }
            })
        });

        let colors = colors.read();
        let big = colors[0];
        let mid = colors[1];
        let small = colors[2];

        rsx! {
            div { background: "{big}", height: "stretch", width: "stretch", padding: "50",
                label { "hello" }
                div { background: "{mid}", height: "auto", width: "stretch", padding: "{padding}",
                    label { "World" }
                    div { background: "{small}", height: "auto", width: "stretch", padding: "20", label { "ddddddd" } }
                }
            }
        }
    }

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .unwrap();

    rt.block_on(async {
        let mut dom = VirtualDom::new(app);
        dom.rebuild(&mut dioxus_core::NoOpMutations);

        for _ in 0..10 {
            dom.wait_for_work().await;
            dom.render_immediate(&mut NoOpMutations);
        }
    });
}
