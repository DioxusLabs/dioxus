/*
Stress Miri as much as possible.

Prove that we don't leak memory and that our methods are safe.

Specifically:
- [ ] VirtualDom drops memory safely
- [ ] Borrowed components don't expose invalid pointers
- [ ] Async isn't busted
*/

use dioxus::{prelude::*, DomEdit, ScopeId};
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

    fn child(cx: Scope<()>) -> Element {
        rsx!(cx, div {
            "goodbye world"
        })
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
            child(na: "asd".to_string())
        ))
    }

    #[derive(PartialEq, Props)]
    struct ChildProps {
        na: String,
    }

    fn child(cx: Scope<ChildProps>) -> Element {
        rsx!(cx, div {
            "goodbye world"
        })
    }

    let mut dom = new_dom(app, ());

    dom.rebuild();
    dom.hard_diff(ScopeId(0));
    dom.hard_diff(ScopeId(0));
    dom.hard_diff(ScopeId(0));
    dom.hard_diff(ScopeId(0));
    dom.hard_diff(ScopeId(0));
}
