#![allow(unused, non_upper_case_globals, non_snake_case)]
use std::collections::HashMap;
use std::rc::Rc;

use dioxus::prelude::*;
use dioxus_core::ElementId;
use dioxus_signals::*;

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
        |cx| {
            let mut signal = use_signal(cx, || 0);

            println!("Parent: {:?}", cx.scope_id());
            if cx.generation() == 1 {
                signal += 1;
            }

            cx.props.borrow_mut().parent += 1;

            render! {
                for id in 0..10 {
                    Child {
                        signal: signal,
                        counter: cx.props.clone()
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

    fn Child(cx: Scope<ChildProps>) -> Element {
        println!("Child: {:?}", cx.scope_id());
        *cx.props
            .counter
            .borrow_mut()
            .children
            .entry(cx.scope_id())
            .or_default() += 1;

        render! {
            "{cx.props.signal}"
        }
    }

    let _ = dom.rebuild().santize();

    {
        let current_counter = counter.borrow();
        assert_eq!(current_counter.parent, 1);

        for (scope_id, rerun_count) in current_counter.children.iter() {
            assert_eq!(rerun_count, &1);
        }
    }

    dom.mark_dirty(ScopeId::ROOT);
    dom.render_immediate();
    dom.render_immediate();

    {
        let current_counter = counter.borrow();
        assert_eq!(current_counter.parent, 2);

        for (scope_id, rerun_count) in current_counter.children.iter() {
            assert_eq!(rerun_count, &2);
        }
    }
}
