#![allow(unused, non_upper_case_globals, non_snake_case)]
use std::collections::HashMap;
use std::rc::Rc;

use dioxus::prelude::*;
use dioxus_core::ElementId;
use dioxus_signals::*;

#[test]
fn effects_rerun() {
    simple_logger::SimpleLogger::new().init().unwrap();

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
            cx.use_hook(move || {
                to_owned![counter];
                Effect::new(move || {
                    counter.borrow_mut().effect += 1;
                    println!("Signal: {:?}", signal);
                })
            });
            signal += 1;

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
