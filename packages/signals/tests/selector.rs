#![allow(unused, non_upper_case_globals, non_snake_case)]
use std::collections::HashMap;
use std::rc::Rc;

use dioxus::html::p;
use dioxus::prelude::*;
use dioxus_core::ElementId;
use dioxus_signals::*;

#[test]
fn memos_rerun() {
    let _ = simple_logger::SimpleLogger::new().init();

    #[derive(Default)]
    struct RunCounter {
        component: usize,
        effect: usize,
    }

    let counter = Rc::new(RefCell::new(RunCounter::default()));
    let mut dom = VirtualDom::new_with_props(
        |cx| {
            let counter = cx.props;
            counter.borrow_mut().component += 1;

            let mut signal = use_signal(cx, || 0);
            let memo = cx.use_hook(move || {
                to_owned![counter];
                selector(move || {
                    counter.borrow_mut().effect += 1;
                    println!("Signal: {:?}", signal);
                    signal.value()
                })
            });
            assert_eq!(memo.value(), 0);
            signal += 1;
            assert_eq!(memo.value(), 1);

            render! {
                div {}
            }
        },
        counter.clone(),
    );

    let _ = dom.rebuild().santize();

    let current_counter = counter.borrow();
    assert_eq!(current_counter.component, 1);
    assert_eq!(current_counter.effect, 2);
}

#[test]
fn memos_prevents_component_rerun() {
    let _ = simple_logger::SimpleLogger::new().init();

    #[derive(Default)]
    struct RunCounter {
        component: usize,
        effect: usize,
    }

    let counter = Rc::new(RefCell::new(RunCounter::default()));
    let mut dom = VirtualDom::new_with_props(
        |cx| {
            let mut signal = use_signal(cx, || 0);

            if cx.generation() == 1 {
                *signal.write() = 0;
            }
            if cx.generation() == 2 {
                println!("Writing to signal");
                *signal.write() = 1;
            }

            render! {
                Child {
                    signal: signal,
                    counter: cx.props.clone(),
                }
            }
        },
        counter.clone(),
    );

    #[derive(Default, Props)]
    struct ChildProps {
        signal: Signal<usize>,
        counter: Rc<RefCell<RunCounter>>,
    }

    impl PartialEq for ChildProps {
        fn eq(&self, other: &Self) -> bool {
            self.signal == other.signal
        }
    }

    fn Child(cx: Scope<ChildProps>) -> Element {
        let counter = &cx.props.counter;
        let signal = cx.props.signal;
        counter.borrow_mut().component += 1;

        let memo = cx.use_hook(move || {
            to_owned![counter];
            selector(move || {
                counter.borrow_mut().effect += 1;
                println!("Signal: {:?}", signal);
                signal.value()
            })
        });
        match cx.generation() {
            0 => {
                assert_eq!(memo.value(), 0);
            }
            1 => {
                assert_eq!(memo.value(), 1);
            }
            _ => panic!("Unexpected generation"),
        }

        render! {
            div {}
        }
    }

    let _ = dom.rebuild().santize();
    dom.mark_dirty(ScopeId::ROOT);
    dom.render_immediate();

    {
        let current_counter = counter.borrow();
        assert_eq!(current_counter.component, 1);
        assert_eq!(current_counter.effect, 2);
    }

    dom.mark_dirty(ScopeId::ROOT);
    dom.render_immediate();
    dom.render_immediate();

    {
        let current_counter = counter.borrow();
        assert_eq!(current_counter.component, 2);
        assert_eq!(current_counter.effect, 3);
    }
}
