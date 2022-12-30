#![allow(non_snake_case)]

use std::rc::Rc;

use dioxus::prelude::*;

/// This test checks that we should release all memory used by the virtualdom when it exits.
///
/// When miri runs, it'll let us know if we leaked or aliased.
#[test]
fn test_memory_leak() {
    fn app(cx: Scope) -> Element {
        let val = cx.generation();

        cx.spawn(async {});

        if val == 2 || val == 4 {
            return cx.render(rsx!(()));
        }

        let name = cx.use_hook(|| String::from("numbers: "));

        name.push_str("123 ");

        cx.render(rsx!(
            div { "Hello, world!" }
            Child {}
            Child {}
            Child {}
            Child {}
            Child {}
            Child {}
            BorrowedChild { name: name }
            BorrowedChild { name: name }
            BorrowedChild { name: name }
            BorrowedChild { name: name }
            BorrowedChild { name: name }
        ))
    }

    #[derive(Props)]
    struct BorrowedProps<'a> {
        name: &'a str,
    }

    fn BorrowedChild<'a>(cx: Scope<'a, BorrowedProps<'a>>) -> Element {
        cx.render(rsx! {
            div {
                "goodbye {cx.props.name}"
                Child {}
                Child {}
            }
        })
    }

    fn Child(cx: Scope) -> Element {
        render!(div { "goodbye world" })
    }

    let mut dom = VirtualDom::new(app);

    _ = dom.rebuild();

    for _ in 0..5 {
        dom.mark_dirty(ScopeId(0));
        _ = dom.render_immediate();
    }
}

#[test]
fn memo_works_properly() {
    fn app(cx: Scope) -> Element {
        let val = cx.generation();

        if val == 2 || val == 4 {
            return cx.render(rsx!(()));
        }

        let name = cx.use_hook(|| String::from("asd"));

        cx.render(rsx!(
            div { "Hello, world! {name}" }
            Child { na: "asdfg".to_string() }
        ))
    }

    #[derive(PartialEq, Props)]
    struct ChildProps {
        na: String,
    }

    fn Child(cx: Scope<ChildProps>) -> Element {
        render!(div { "goodbye world" })
    }

    let mut dom = VirtualDom::new(app);

    _ = dom.rebuild();
    // todo!()
    // dom.hard_diff(ScopeId(0));
    // dom.hard_diff(ScopeId(0));
    // dom.hard_diff(ScopeId(0));
    // dom.hard_diff(ScopeId(0));
    // dom.hard_diff(ScopeId(0));
    // dom.hard_diff(ScopeId(0));
    // dom.hard_diff(ScopeId(0));
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

    fn app(cx: Scope<AppProps>) -> Element {
        let name: &AppProps = cx.use_hook(|| cx.props.clone());
        render!(child_component { inner: name.inner.clone() })
    }

    fn child_component(cx: Scope<AppProps>) -> Element {
        render!(div { "{cx.props.inner}" })
    }

    let ptr = Rc::new("asdasd".to_string());
    let mut dom = VirtualDom::new_with_props(app, AppProps { inner: ptr.clone() });
    let _ = dom.rebuild();

    // ptr gets cloned into props and then into the hook
    assert_eq!(Rc::strong_count(&ptr), 4);

    drop(dom);

    assert_eq!(Rc::strong_count(&ptr), 1);
}

#[test]
fn supports_async() {
    use std::time::Duration;
    use tokio::time::sleep;

    fn app(cx: Scope) -> Element {
        let colors = use_state(&cx, || vec!["green", "blue", "red"]);
        let padding = use_state(&cx, || 10);

        use_effect(&cx, colors, |colors| async move {
            sleep(Duration::from_millis(1000)).await;
            colors.with_mut(|colors| colors.reverse());
        });

        use_effect(&cx, padding, |padding| async move {
            sleep(Duration::from_millis(10)).await;
            padding.with_mut(|padding| {
                if *padding < 65 {
                    *padding += 1;
                } else {
                    *padding = 5;
                }
            });
        });

        let big = colors[0];
        let mid = colors[1];
        let small = colors[2];

        cx.render(rsx! {
            div {
                background: "{big}",
                height: "stretch",
                width: "stretch",
                padding: "50",
                label {
                    "hello",
                }
                div {
                    background: "{mid}",
                    height: "auto",
                    width: "stretch",
                    padding: "{padding}",
                    label {
                        "World",
                    }
                    div {
                        background: "{small}",
                        height: "auto",
                        width: "stretch",
                        padding: "20",
                        label {
                            "ddddddd",
                        }
                    }
                },
            }
        })
    }

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .unwrap();

    rt.block_on(async {
        let mut dom = VirtualDom::new(app);
        let _ = dom.rebuild();

        for x in 0..10 {
            let _ = dom.wait_for_work().await;
            let edits = dom.render_immediate();
            dbg!(edits);
        }
    });
}

// #[test]
// fn old_props_arent_stale() {
//     fn app(cx: Scope) -> Element {
//         dbg!("rendering parent");
//         let cnt = cx.use_hook(|| 0);
//         *cnt += 1;

//         if *cnt == 1 {
//             render!(div { Child { a: "abcdef".to_string() } })
//         } else {
//             render!(div { Child { a: "abcdef".to_string() } })
//         }
//     }

//     #[derive(Props, PartialEq)]
//     struct ChildProps {
//         a: String,
//     }
//     fn Child(cx: Scope<ChildProps>) -> Element {
//         dbg!("rendering child", &cx.props.a);
//         render!(div { "child {cx.props.a}" })
//     }

//     let mut dom = new_dom(app, ());
//     let _ = dom.rebuild();

//     dom.handle_message(SchedulerMsg::Immediate(ScopeId(0)));
//     dom.work_with_deadline(|| false);

//     dom.handle_message(SchedulerMsg::Immediate(ScopeId(0)));
//     dom.work_with_deadline(|| false);

//     dom.handle_message(SchedulerMsg::Immediate(ScopeId(0)));
//     dom.work_with_deadline(|| false);

//     dbg!("forcing update to child");

//     dom.handle_message(SchedulerMsg::Immediate(ScopeId(1)));
//     dom.work_with_deadline(|| false);

//     dom.handle_message(SchedulerMsg::Immediate(ScopeId(1)));
//     dom.work_with_deadline(|| false);

//     dom.handle_message(SchedulerMsg::Immediate(ScopeId(1)));
//     dom.work_with_deadline(|| false);
// }

// #[test]
// fn basic() {
//     fn app(cx: Scope) -> Element {
//         render!(div {
//             Child { a: "abcdef".to_string() }
//         })
//     }

//     #[derive(Props, PartialEq)]
//     struct ChildProps {
//         a: String,
//     }

//     fn Child(cx: Scope<ChildProps>) -> Element {
//         dbg!("rendering child", &cx.props.a);
//         render!(div { "child {cx.props.a}" })
//     }

//     let mut dom = new_dom(app, ());
//     let _ = dom.rebuild();

//     dom.handle_message(SchedulerMsg::Immediate(ScopeId(0)));
//     dom.work_with_deadline(|| false);

//     dom.handle_message(SchedulerMsg::Immediate(ScopeId(0)));
//     dom.work_with_deadline(|| false);
// }

// #[test]
// fn leak_thru_children() {
//     fn app(cx: Scope) -> Element {
//         cx.render(rsx! {
//             Child {
//                 name: "asd".to_string(),
//             }
//         });
//         cx.render(rsx! {
//             div {}
//         })
//     }

//     #[inline_props]
//     fn Child(cx: Scope, name: String) -> Element {
//         render!(div { "child {name}" })
//     }

//     let mut dom = new_dom(app, ());
//     let _ = dom.rebuild();

//     dom.handle_message(SchedulerMsg::Immediate(ScopeId(0)));
//     dom.work_with_deadline(|| false);

//     dom.handle_message(SchedulerMsg::Immediate(ScopeId(0)));
//     dom.work_with_deadline(|| false);
// }

// #[test]
// fn test_pass_thru() {
//     #[inline_props]
//     fn NavContainer<'a>(cx: Scope, children: Element<'a>) -> Element {
//         cx.render(rsx! {
//             header {
//                 nav { children }
//             }
//         })
//     }

//     fn NavMenu(cx: Scope) -> Element {
//         render!(            NavBrand {}
//             div {
//                 NavStart {}
//                 NavEnd {}
//             }
//         )
//     }

//     fn NavBrand(cx: Scope) -> Element {
//         render!(div {})
//     }

//     fn NavStart(cx: Scope) -> Element {
//         render!(div {})
//     }

//     fn NavEnd(cx: Scope) -> Element {
//         render!(div {})
//     }

//     #[inline_props]
//     fn MainContainer<'a>(
//         cx: Scope,
//         nav: Element<'a>,
//         body: Element<'a>,
//         footer: Element<'a>,
//     ) -> Element {
//         cx.render(rsx! {
//             div {
//                 class: "columns is-mobile",
//                 div {
//                     class: "column is-full",
//                     nav,
//                     body,
//                     footer,
//                 }
//             }
//         })
//     }

//     fn app(cx: Scope) -> Element {
//         let nav = cx.render(rsx! {
//             NavContainer {
//                 NavMenu {}
//             }
//         });
//         let body = cx.render(rsx! {
//             div {}
//         });
//         let footer = cx.render(rsx! {
//             div {}
//         });

//         cx.render(rsx! {
//             MainContainer {
//                 nav: nav,
//                 body: body,
//                 footer: footer,
//             }
//         })
//     }

//     let mut dom = new_dom(app, ());
//     let _ = dom.rebuild();

//     for _ in 0..40 {
//         dom.handle_message(SchedulerMsg::Immediate(ScopeId(0)));
//         dom.work_with_deadline(|| false);
//         dom.handle_message(SchedulerMsg::Immediate(ScopeId(0)));
//         dom.work_with_deadline(|| false);
//         dom.handle_message(SchedulerMsg::Immediate(ScopeId(0)));
//         dom.work_with_deadline(|| false);

//         dom.handle_message(SchedulerMsg::Immediate(ScopeId(1)));
//         dom.work_with_deadline(|| false);
//         dom.handle_message(SchedulerMsg::Immediate(ScopeId(1)));
//         dom.work_with_deadline(|| false);
//         dom.handle_message(SchedulerMsg::Immediate(ScopeId(1)));
//         dom.work_with_deadline(|| false);

//         dom.handle_message(SchedulerMsg::Immediate(ScopeId(2)));
//         dom.work_with_deadline(|| false);
//         dom.handle_message(SchedulerMsg::Immediate(ScopeId(2)));
//         dom.work_with_deadline(|| false);
//         dom.handle_message(SchedulerMsg::Immediate(ScopeId(2)));
//         dom.work_with_deadline(|| false);

//         dom.handle_message(SchedulerMsg::Immediate(ScopeId(3)));
//         dom.work_with_deadline(|| false);
//         dom.handle_message(SchedulerMsg::Immediate(ScopeId(3)));
//         dom.work_with_deadline(|| false);
//         dom.handle_message(SchedulerMsg::Immediate(ScopeId(3)));
//         dom.work_with_deadline(|| false);
//     }
// }
