#![allow(unused, non_upper_case_globals, non_snake_case)]
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::time::Duration;

use dioxus::prelude::*;
use dioxus_core::ElementId;
use dioxus_signals::*;

#[tokio::test]
async fn effects_rerun() {
    #[derive(Default)]
    struct RunCounter {
        component: usize,
        effect: usize,
    }

    let counter = Rc::new(RefCell::new(RunCounter::default()));
    let mut dom = VirtualDom::new_with_props(
        |counter: Rc<RefCell<RunCounter>>| {
            counter.borrow_mut().component += 1;

            let mut signal = use_signal(|| 0);
            use_effect({
                to_owned![counter];
                move || {
                    counter.borrow_mut().effect += 1;
                    // This will subscribe the effect to the signal
                    println!("Signal: {:?}", signal);

                    // Stop the wait for work manually
                    needs_update();
                }
            });
            signal += 1;

            rsx! {
                div {}
            }
        },
        counter.clone(),
    );

    dom.rebuild_in_place();
    tokio::select! {
        _ = dom.wait_for_work() => {}
        _ = tokio::time::sleep(Duration::from_millis(500)) => panic!("timed out")
    };

    let current_counter = counter.borrow();
    assert_eq!(current_counter.component, 1);
    assert_eq!(current_counter.effect, 1);
}

// https://github.com/DioxusLabs/dioxus/issues/2347
// Effects should rerun when the signal changes if there are no changes to the component
#[tokio::test]
async fn effects_rerun_without_rerender() {
    #[derive(Default)]
    struct RunCounter {
        component: usize,
        effect: usize,
    }

    let counter = Rc::new(RefCell::new(RunCounter::default()));
    let mut dom = VirtualDom::new_with_props(
        |counter: Rc<RefCell<RunCounter>>| {
            counter.borrow_mut().component += 1;
            println!("component {}", counter.borrow().component);

            let mut signal = use_signal(|| 0);
            use_effect({
                to_owned![counter];
                move || {
                    counter.borrow_mut().effect += 1;
                    // This will subscribe the effect to the signal
                    println!("Signal: {}", signal);
                }
            });
            use_future(move || async move {
                for i in 0..10 {
                    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                    println!("future {}", i);
                    signal += 1;
                }
            });

            rsx! {
                div {}
            }
        },
        counter.clone(),
    );

    dom.rebuild_in_place();
    tokio::select! {
        _ = dom.wait_for_work() => {}
        _ = tokio::time::sleep(Duration::from_millis(500)) => {}
    };

    let current_counter = counter.borrow();
    assert_eq!(current_counter.component, 1);
    assert_eq!(current_counter.effect, 11);
}
