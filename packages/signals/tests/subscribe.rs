#![allow(unused, non_upper_case_globals, non_snake_case)]

use dioxus_core::{current_scope_id, generation, NoOpMutations};
use std::collections::HashMap;
use std::rc::Rc;

use dioxus::prelude::*;
use dioxus_core::ElementId;
use dioxus_signals::*;
use std::cell::RefCell;

#[test]
fn reading_subscribes() {
    tracing_subscriber::fmt::init();

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
            .entry(current_scope_id())
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

    dom.mark_dirty(ScopeId::APP);
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

#[test]
fn read_signal_point_to_moves_only_read_subscribers() {
    #[derive(Default)]
    struct RunCounter {
        parent_effect_runs: usize,
        child_renders: usize,
    }

    #[derive(Default)]
    struct Handles {
        use_b: Option<Signal<bool>>,
        signal_b: Option<Signal<i32>>,
    }

    let run_counter = Rc::new(RefCell::new(RunCounter::default()));
    let handles = Rc::new(RefCell::new(Handles::default()));

    let mut dom = VirtualDom::new_with_props(
        {
            let handles = handles.clone();
            move |counter: Rc<RefCell<RunCounter>>| {
                let counter = counter.clone();
                let effect_counter = counter.clone();
                let mut use_b = use_signal(|| false);
                let signal_a = use_signal(|| 0);
                let mut signal_b = use_signal(|| 0);

                {
                    let mut slots = handles.borrow_mut();
                    slots.use_b = Some(use_b);
                    slots.signal_b = Some(signal_b);
                }

                use_effect(move || {
                    signal_a();
                    effect_counter.borrow_mut().parent_effect_runs += 1;
                });

                let child_signal = if use_b() { signal_b } else { signal_a };

                rsx! {
                    Child {
                        sig: child_signal,
                        counts: counter
                    }
                }
            }
        },
        run_counter.clone(),
    );

    #[derive(Props, Clone)]
    struct ChildProps {
        sig: ReadSignal<i32>,
        counts: Rc<RefCell<RunCounter>>,
    }

    impl PartialEq for ChildProps {
        fn eq(&self, other: &Self) -> bool {
            self.sig == other.sig
        }
    }

    fn Child(props: ChildProps) -> Element {
        let mut counts = props.counts.borrow_mut();
        counts.child_renders += 1;
        let _value = (props.sig)();
        rsx! {
            "{props.sig}"
        }
    }

    dom.rebuild_in_place();
    dom.render_immediate(&mut NoOpMutations);

    {
        let current_counter = run_counter.borrow();
        assert_eq!(current_counter.parent_effect_runs, 1);
        assert_eq!(current_counter.child_renders, 1);
    }

    let mut use_b = handles.borrow().use_b.unwrap();
    use_b.set(true);
    dom.mark_dirty(ScopeId::APP);
    dom.render_immediate(&mut NoOpMutations);
    dom.render_immediate(&mut NoOpMutations);

    {
        let current_counter = run_counter.borrow();
        assert_eq!(current_counter.parent_effect_runs, 1);
        assert_eq!(current_counter.child_renders, 1);
    }

    let mut signal_b = handles.borrow().signal_b.unwrap();
    signal_b.set(1);
    dom.mark_dirty(ScopeId::APP);
    dom.render_immediate(&mut NoOpMutations);
    dom.render_immediate(&mut NoOpMutations);

    {
        let current_counter = run_counter.borrow();
        assert_eq!(current_counter.parent_effect_runs, 1);
        assert_eq!(current_counter.child_renders, 2);
    }
}

#[test]
fn read_signal_point_to_leaves_direct_underlying_subscribers() {
    #[derive(Default)]
    struct RunCounter {
        direct_effect_runs: usize,
        child_renders: usize,
    }

    #[derive(Default)]
    struct Handles {
        use_b: Option<Signal<bool>>,
        signal_a: Option<Signal<i32>>,
        signal_b: Option<Signal<i32>>,
    }

    let run_counter = Rc::new(RefCell::new(RunCounter::default()));
    let handles = Rc::new(RefCell::new(Handles::default()));

    let mut dom = VirtualDom::new_with_props(
        {
            let handles = handles.clone();
            move |counter: Rc<RefCell<RunCounter>>| {
                let counter = counter.clone();
                let effect_counter = counter.clone();
                let mut use_b = use_signal(|| false);
                let mut signal_a = use_signal(|| 0);
                let mut signal_b = use_signal(|| 0);

                {
                    let mut slots = handles.borrow_mut();
                    slots.use_b = Some(use_b);
                    slots.signal_a = Some(signal_a);
                    slots.signal_b = Some(signal_b);
                }

                use_effect(move || {
                    signal_a();
                    effect_counter.borrow_mut().direct_effect_runs += 1;
                });

                let child_signal = if use_b() { signal_b } else { signal_a };

                rsx! {
                    Child {
                        sig: ReadSignal::from(child_signal),
                        counts: counter
                    }
                }
            }
        },
        run_counter.clone(),
    );

    #[derive(Props, Clone)]
    struct ChildProps {
        sig: ReadSignal<i32>,
        counts: Rc<RefCell<RunCounter>>,
    }

    impl PartialEq for ChildProps {
        fn eq(&self, other: &Self) -> bool {
            self.sig == other.sig
        }
    }

    fn Child(props: ChildProps) -> Element {
        props.counts.borrow_mut().child_renders += 1;
        let _ = (props.sig)();
        rsx! { "{props.sig}" }
    }

    dom.rebuild_in_place();
    dom.render_immediate(&mut NoOpMutations);

    let mut use_b = handles.borrow().use_b.unwrap();
    use_b.set(true);
    dom.mark_dirty(ScopeId::APP);
    dom.render_immediate(&mut NoOpMutations);
    dom.render_immediate(&mut NoOpMutations);

    let mut signal_a = handles.borrow().signal_a.unwrap();
    signal_a.set(1);
    dom.mark_dirty(ScopeId::APP);
    dom.render_immediate(&mut NoOpMutations);
    dom.render_immediate(&mut NoOpMutations);

    {
        let current_counter = run_counter.borrow();
        assert_eq!(current_counter.direct_effect_runs, 2);
        assert_eq!(current_counter.child_renders, 1);
    }

    let mut signal_b = handles.borrow().signal_b.unwrap();
    signal_b.set(1);
    dom.mark_dirty(ScopeId::APP);
    dom.render_immediate(&mut NoOpMutations);
    dom.render_immediate(&mut NoOpMutations);

    {
        let current_counter = run_counter.borrow();
        assert_eq!(current_counter.direct_effect_runs, 2);
        assert_eq!(current_counter.child_renders, 2);
    }
}
