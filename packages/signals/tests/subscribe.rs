#![allow(unused, non_upper_case_globals, non_snake_case)]

use dioxus_core::NoOpMutations;
use std::collections::HashMap;
use std::rc::Rc;

use dioxus::prelude::*;
use dioxus_core::ElementId;
use dioxus_signals::*;
use std::cell::RefCell;

#[test]
fn reading_subscribes() {
    simple_logger::SimpleLogger::new().init().unwrap();

    #[derive(Default)]
    struct RunCounter {
        parent: usize,
        children: HashMap<ScopeId, usize>,
    }

    let counter = Rc::new(RefCell::new(RunCounter::default()));
    let mut dom = VirtualDom::new_with_props(
        |props: Rc<RefCell<RunCounter>>| {
            let mut signal = use_signal(|| 0);

            println!("Parent: {:?}", current_scope_id());
            if generation() == 1 {
                signal += 1;
            }

            props.borrow_mut().parent += 1;

            rsx! {
                for id in 0..10 {
                    Child {
                        signal: signal,
                        counter: props.clone()
                    }
                }
            }
        },
        counter.clone(),
    );

    #[derive(Props, Clone)]
    struct ChildProps {
        signal: Signal<usize>,
        counter: Rc<RefCell<RunCounter>>,
    }

    impl PartialEq for ChildProps {
        fn eq(&self, other: &Self) -> bool {
            self.signal == other.signal
        }
    }

    fn Child(props: ChildProps) -> Element {
        println!("Child: {:?}", current_scope_id());
        *props
            .counter
            .borrow_mut()
            .children
            .entry(current_scope_id().unwrap())
            .or_default() += 1;

        rsx! {
            "{props.signal}"
        }
    }

    dom.rebuild_in_place();

    {
        let current_counter = counter.borrow();
        assert_eq!(current_counter.parent, 1);

        for (scope_id, rerun_count) in current_counter.children.iter() {
            assert_eq!(rerun_count, &1);
        }
    }

    dom.mark_dirty(ScopeId::ROOT);
    dom.render_immediate(&mut NoOpMutations);
    dom.render_immediate(&mut NoOpMutations);

    {
        let current_counter = counter.borrow();
        assert_eq!(current_counter.parent, 2);

        for (scope_id, rerun_count) in current_counter.children.iter() {
            assert_eq!(rerun_count, &2);
        }
    }
}
