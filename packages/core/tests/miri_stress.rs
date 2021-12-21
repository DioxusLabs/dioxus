/*
Stress Miri as much as possible.

Prove that we don't leak memory and that our methods are safe.

Specifically:
- [ ] VirtualDom drops memory safely
- [ ] Borrowed components don't expose invalid pointers
- [ ] Async isn't busted
*/

use dioxus::{prelude::*, DomEdit, SchedulerMsg, ScopeId};
use dioxus_core as dioxus;
use dioxus_core_macro::*;
use dioxus_html as dioxus_elements;

mod test_logging;
use DomEdit::*;

const IS_LOGGING_ENABLED: bool = false;

fn new_dom<P: 'static + Send>(app: Component<P>, props: P) -> VirtualDom {
    test_logging::set_up_logging(IS_LOGGING_ENABLED);
    VirtualDom::new_with_props(app, props)
}

/// This test ensures that if a component aborts early, it is replaced with a placeholder.
/// In debug, this should also toss a warning.
#[test]
fn test_memory_leak() {
    fn app(cx: Scope<()>) -> Element {
        let val = cx.use_hook(|_| 0, |f| f);

        *val += 1;

        if *val == 2 || *val == 4 {
            return None;
        }

        let name = cx.use_hook(|_| String::from("asd"), |f| f);

        cx.render(rsx!(
            div { "Hello, world!" }
            child()
            child()
            child()
            child()
            child()
            child()
            borrowed_child(na: name)
            borrowed_child(na: name)
            borrowed_child(na: name)
            borrowed_child(na: name)
            borrowed_child(na: name)
        ))
    }

    #[derive(Props)]
    struct BorrowedProps<'a> {
        na: &'a str,
    }

    fn borrowed_child<'a>(cx: Scope<'a, BorrowedProps<'a>>) -> Element {
        rsx!(cx, div {
            "goodbye {cx.props.na}"
            child()
            child()
        })
    }

    fn child(cx: Scope<()>) -> Element {
        rsx!(cx, div { "goodbye world" })
    }

    let mut dom = new_dom(app, ());

    dom.rebuild();
    dom.hard_diff(ScopeId(0));
    dom.hard_diff(ScopeId(0));
    dom.hard_diff(ScopeId(0));
    dom.hard_diff(ScopeId(0));
    dom.hard_diff(ScopeId(0));
    dom.hard_diff(ScopeId(0));
}

#[test]
fn memo_works_properly() {
    fn app(cx: Scope<()>) -> Element {
        let val = cx.use_hook(|_| 0, |f| f);

        *val += 1;

        if *val == 2 || *val == 4 {
            return None;
        }

        let name = cx.use_hook(|_| String::from("asd"), |f| f);

        cx.render(rsx!(
            div { "Hello, world!" }
            child(na: "asdfg".to_string())
        ))
    }

    #[derive(PartialEq, Props)]
    struct ChildProps {
        na: String,
    }

    fn child(cx: Scope<ChildProps>) -> Element {
        rsx!(cx, div { "goodbye world" })
    }

    let mut dom = new_dom(app, ());

    dom.rebuild();
    dom.hard_diff(ScopeId(0));
    dom.hard_diff(ScopeId(0));
    dom.hard_diff(ScopeId(0));
    dom.hard_diff(ScopeId(0));
    dom.hard_diff(ScopeId(0));
    dom.hard_diff(ScopeId(0));
    dom.hard_diff(ScopeId(0));
}

#[test]
fn free_works_on_root_props() {
    fn app(cx: Scope<Custom>) -> Element {
        cx.render(rsx! {
            child(a: "alpha")
            child(a: "beta")
            child(a: "gamma")
            child(a: "delta")
        })
    }

    #[derive(Props, PartialEq)]
    struct ChildProps {
        a: &'static str,
    }

    fn child(cx: Scope<ChildProps>) -> Element {
        rsx!(cx, "child {cx.props.a}")
    }

    struct Custom {
        val: String,
    }

    impl Drop for Custom {
        fn drop(&mut self) {
            dbg!("dropped!");
        }
    }

    let mut dom = new_dom(
        app,
        Custom {
            val: String::from("asd"),
        },
    );
    dom.rebuild();
}

#[test]
fn free_works_on_borrowed() {
    fn app(cx: Scope<()>) -> Element {
        cx.render(rsx! {
            child(a: "alpha", b: "asd".to_string())
        })
    }
    #[derive(Props)]
    struct ChildProps<'a> {
        a: &'a str,
        b: String,
    }

    fn child<'a>(cx: Scope<'a, ChildProps<'a>>) -> Element {
        dbg!("rendering child");
        rsx!(cx, "child {cx.props.a}, {cx.props.b}")
    }

    impl Drop for ChildProps<'_> {
        fn drop(&mut self) {
            dbg!("dropped child!");
        }
    }

    let mut dom = new_dom(app, ());
    let _ = dom.rebuild();
}

#[test]
fn free_works_on_root_hooks() {
    /*
    On Drop, scopearena drops all the hook contents.
    */

    struct Droppable<T>(T);
    impl<T> Drop for Droppable<T> {
        fn drop(&mut self) {
            dbg!("dropping droppable");
        }
    }

    fn app(cx: Scope<()>) -> Element {
        let name = cx.use_hook(|_| Droppable(String::from("asd")), |f| f);
        rsx!(cx, div { "{name.0}" })
    }

    let mut dom = new_dom(app, ());
    let _ = dom.rebuild();
}

#[test]
fn old_props_arent_stale() {
    fn app(cx: Scope<()>) -> Element {
        dbg!("rendering parent");
        let cnt = cx.use_hook(|_| 0, |f| f);
        *cnt += 1;

        if *cnt == 1 {
            rsx!(cx, div { child(a: "abcdef".to_string()) })
        } else {
            rsx!(cx, div { child(a: "abcdef".to_string()) })
        }
    }

    #[derive(Props, PartialEq)]
    struct ChildProps {
        a: String,
    }
    fn child(cx: Scope<ChildProps>) -> Element {
        dbg!("rendering child", &cx.props.a);
        rsx!(cx, div { "child {cx.props.a}" })
    }

    let mut dom = new_dom(app, ());
    let _ = dom.rebuild();

    dom.handle_message(SchedulerMsg::Immediate(ScopeId(0)));
    dom.work_with_deadline(|| false);

    dom.handle_message(SchedulerMsg::Immediate(ScopeId(0)));
    dom.work_with_deadline(|| false);

    dom.handle_message(SchedulerMsg::Immediate(ScopeId(0)));
    dom.work_with_deadline(|| false);

    dbg!("forcing update to child");

    dom.handle_message(SchedulerMsg::Immediate(ScopeId(1)));
    dom.work_with_deadline(|| false);

    dom.handle_message(SchedulerMsg::Immediate(ScopeId(1)));
    dom.work_with_deadline(|| false);

    dom.handle_message(SchedulerMsg::Immediate(ScopeId(1)));
    dom.work_with_deadline(|| false);
}

#[test]
fn basic() {
    fn app(cx: Scope<()>) -> Element {
        rsx!(cx, div {
            child(a: "abcdef".to_string())
        })
    }

    #[derive(Props, PartialEq)]
    struct ChildProps {
        a: String,
    }

    fn child(cx: Scope<ChildProps>) -> Element {
        dbg!("rendering child", &cx.props.a);
        rsx!(cx, div { "child {cx.props.a}" })
    }

    let mut dom = new_dom(app, ());
    let _ = dom.rebuild();

    dom.handle_message(SchedulerMsg::Immediate(ScopeId(0)));
    dom.work_with_deadline(|| false);

    dom.handle_message(SchedulerMsg::Immediate(ScopeId(0)));
    dom.work_with_deadline(|| false);
}
