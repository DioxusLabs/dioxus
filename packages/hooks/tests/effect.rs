#![allow(unused, non_upper_case_globals, non_snake_case)]
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

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
    dom.wait_for_work().await;

    let current_counter = counter.borrow();
    assert_eq!(current_counter.component, 1);
    assert_eq!(current_counter.effect, 1);
}
