#![allow(unused, non_upper_case_globals, non_snake_case)]

use dioxus::prelude::*;
use dioxus_core::ElementId;
use dioxus_core::NoOpMutations;
use dioxus_signals::*;

#[test]
fn create_signals_global() {
    let mut dom = VirtualDom::new(|| {
        rsx! {
            for _ in 0..10 {
                Child {}
            }
        }
    });

    fn Child() -> Element {
        let signal = create_without_cx();

        rsx! {
            "{signal}"
        }
    }

    dom.rebuild_in_place();

    fn create_without_cx() -> Signal<String> {
        Signal::new("hello world".to_string())
    }
}

#[test]
fn deref_signal() {
    let mut dom = VirtualDom::new(|| {
        rsx! {
            for _ in 0..10 {
                Child {}
            }
        }
    });

    fn Child() -> Element {
        let signal = Signal::new("hello world".to_string());

        // You can call signals like functions to get a Ref of their value.
        assert_eq!(&*signal(), "hello world");

        rsx! {
            "hello world"
        }
    }

    dom.rebuild_in_place();
}

#[test]
fn drop_signals() {
    let mut dom = VirtualDom::new(|| {
        let generation = generation();

        let count = if generation % 2 == 0 { 10 } else { 0 };
        rsx! {
            for _ in 0..count {
                Child {}
            }
        }
    });

    fn Child() -> Element {
        let signal = create_without_cx();

        rsx! {
            "{signal}"
        }
    }

    dom.rebuild_in_place();
    dom.mark_dirty(ScopeId::ROOT);
    dom.render_immediate(&mut NoOpMutations);

    fn create_without_cx() -> Signal<String> {
        Signal::new("hello world".to_string())
    }
}
