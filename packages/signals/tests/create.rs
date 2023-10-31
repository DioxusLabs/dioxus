#![allow(unused, non_upper_case_globals, non_snake_case)]

use dioxus::prelude::*;
use dioxus_core::ElementId;
use dioxus_signals::*;

#[test]
fn create_signals_global() {
    let mut dom = VirtualDom::new(|cx| {
        render! {
            for _ in 0..10 {
                Child {}
            }
        }
    });

    fn Child(cx: Scope) -> Element {
        let signal = create_without_cx();

        render! {
            "{signal}"
        }
    }

    let _edits = dom.rebuild().santize();

    fn create_without_cx() -> Signal<String> {
        Signal::new("hello world".to_string())
    }
}

#[test]
fn deref_signal() {
    let mut dom = VirtualDom::new(|cx| {
        render! {
            for _ in 0..10 {
                Child {}
            }
        }
    });

    fn Child(cx: Scope) -> Element {
        let signal = Signal::new("hello world".to_string());

        // You can call signals like functions to get a Ref of their value.
        assert_eq!(&*signal(), "hello world");

        render! {
            "hello world"
        }
    }

    let _edits = dom.rebuild().santize();
}

#[test]
fn drop_signals() {
    let mut dom = VirtualDom::new(|cx| {
        let generation = cx.generation();

        let count = if generation % 2 == 0 { 10 } else { 0 };
        render! {
            for _ in 0..count {
                Child {}
            }
        }
    });

    fn Child(cx: Scope) -> Element {
        let signal = create_without_cx();

        render! {
            "{signal}"
        }
    }

    let _ = dom.rebuild().santize();
    dom.mark_dirty(ScopeId::ROOT);
    dom.render_immediate();

    fn create_without_cx() -> Signal<String> {
        Signal::new("hello world".to_string())
    }
}
