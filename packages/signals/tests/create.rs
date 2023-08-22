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
    dom.mark_dirty(ScopeId(0));
    dom.render_immediate();

    fn create_without_cx() -> Signal<String> {
        Signal::new("hello world".to_string())
    }
}
