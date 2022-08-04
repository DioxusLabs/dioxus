#![allow(unused, non_upper_case_globals, non_snake_case)]

use dioxus::prelude::*;
use dioxus_core::{DomEdit, Mutations, SchedulerMsg, ScopeId};
use std::rc::Rc;
use DomEdit::*;

#[test]
fn shared_state_test() {
    struct MySharedState(&'static str);

    static App: Component = |cx| {
        cx.provide_context(Rc::new(MySharedState("world!")));
        cx.render(rsx!(Child {}))
    };

    static Child: Component = |cx| {
        let shared = cx.consume_context::<Rc<MySharedState>>()?;
        cx.render(rsx!("Hello, {shared.0}"))
    };

    let mut dom = VirtualDom::new(App);
    let Mutations { edits, .. } = dom.rebuild();

    assert_eq!(
        edits,
        [
            CreateTextNode { root: 1, text: "Hello, world!" },
            AppendChildren { many: 1 },
        ]
    );
}

#[test]
fn swap_test() {
    struct MySharedState(&'static str);

    fn app(cx: Scope) -> Element {
        let val = cx.use_hook(|| 0);
        *val += 1;

        cx.provide_context(Rc::new(MySharedState("world!")));

        let child = match *val % 2 {
            0 => rsx!(
                Child1 {
                    Child1 { }
                    Child2 { }
                }
            ),
            _ => rsx!(
                Child2 {
                    Child2 { }
                    Child2 { }
                }
            ),
        };

        cx.render(rsx!(
            Router {
                div { child }
            }
        ))
    }

    #[inline_props]
    fn Router<'a>(cx: Scope, children: Element<'a>) -> Element<'a> {
        cx.render(rsx!(div { children }))
    }

    #[inline_props]
    fn Child1<'a>(cx: Scope, children: Element<'a>) -> Element {
        let shared = cx.consume_context::<Rc<MySharedState>>().unwrap();
        println!("Child1: {}", shared.0);
        cx.render(rsx! {
            div {
                "{shared.0}",
                children
            }
        })
    }

    #[inline_props]
    fn Child2<'a>(cx: Scope, children: Element<'a>) -> Element {
        let shared = cx.consume_context::<Rc<MySharedState>>().unwrap();
        println!("Child2: {}", shared.0);
        cx.render(rsx! {
            h1 {
                "{shared.0}",
                children
            }
        })
    }

    let mut dom = VirtualDom::new(app);
    let Mutations { edits, .. } = dom.rebuild();

    dom.handle_message(SchedulerMsg::Immediate(ScopeId(0)));
    dom.work_with_deadline(|| false);
    dom.handle_message(SchedulerMsg::Immediate(ScopeId(0)));
    dom.work_with_deadline(|| false);
    dom.handle_message(SchedulerMsg::Immediate(ScopeId(0)));
    dom.work_with_deadline(|| false);
    dom.handle_message(SchedulerMsg::Immediate(ScopeId(0)));
    dom.work_with_deadline(|| false);
    dom.handle_message(SchedulerMsg::Immediate(ScopeId(0)));
    dom.work_with_deadline(|| false);
    dom.handle_message(SchedulerMsg::Immediate(ScopeId(0)));
    dom.work_with_deadline(|| false);

    // dom.handle_message(SchedulerMsg::Immediate(ScopeId(1)));
    // dom.work_with_deadline(|| false);

    // dom.handle_message(SchedulerMsg::Immediate(ScopeId(2)));
    // dom.work_with_deadline(|| false);

    // dom.handle_message(SchedulerMsg::Immediate(ScopeId(3)));
    // dom.work_with_deadline(|| false);

    // dom.handle_message(SchedulerMsg::Immediate(ScopeId(0)));
    // dom.work_with_deadline(|| false);
    // dom.handle_message(SchedulerMsg::Immediate(ScopeId(0)));
    // dom.work_with_deadline(|| false);
    // dom.handle_message(SchedulerMsg::Immediate(ScopeId(0)));
    // dom.work_with_deadline(|| false);
}
