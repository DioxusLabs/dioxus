#![allow(unused, non_upper_case_globals, non_snake_case)]

use dioxus_core::{NoOpMutations, current_scope_id, generation};
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

#[test]
fn boxed_read_signal_subscribes_to_underlying_updates() {
    type Props = (Rc<RefCell<usize>>, Rc<RefCell<Option<Signal<i32>>>>);

    let render_count = Rc::new(RefCell::new(0usize));
    let signal_handle = Rc::new(RefCell::new(None));

    let mut dom = VirtualDom::new_with_props(
        |(render_count, signal_handle): Props| {
            let mut signal = use_signal(|| 0);
            *signal_handle.borrow_mut() = Some(signal);

            let boxed = ReadSignal::from(signal);
            let _ = boxed();
            *render_count.borrow_mut() += 1;

            rsx! { "{boxed}" }
        },
        (render_count.clone(), signal_handle.clone()),
    );

    dom.rebuild_in_place();

    {
        let current_render_count = render_count.borrow();
        assert_eq!(*current_render_count, 1);
    }

    let mut signal = signal_handle.borrow().unwrap();
    signal.set(1);
    dom.render_immediate(&mut NoOpMutations);
    dom.render_immediate(&mut NoOpMutations);

    {
        let current_render_count = render_count.borrow();
        assert_eq!(*current_render_count, 2);
    }
}

// Exercises the Send+Sync forwarding-context path on SyncStorage. Mirrors
// `boxed_read_signal_subscribes_to_underlying_updates` but with sync storage so the lazy-init
// helpers' rechecked-under-write paths are exercised on a SyncStorage value.
#[test]
fn boxed_sync_read_signal_subscribes_to_underlying_updates() {
    use generational_box::SyncStorage;

    type SyncSignal = Signal<i32, SyncStorage>;
    type Props = (
        std::sync::Arc<std::sync::Mutex<usize>>,
        std::sync::Arc<std::sync::Mutex<Option<SyncSignal>>>,
    );

    let render_count = std::sync::Arc::new(std::sync::Mutex::new(0usize));
    let signal_handle: std::sync::Arc<std::sync::Mutex<Option<SyncSignal>>> =
        std::sync::Arc::new(std::sync::Mutex::new(None));

    let mut dom = VirtualDom::new_with_props(
        |(render_count, signal_handle): Props| {
            let mut signal = use_signal_sync(|| 0);
            *signal_handle.lock().unwrap() = Some(signal);

            let boxed: ReadSignal<i32, SyncStorage> = ReadSignal::from(signal);
            let _ = boxed();
            *render_count.lock().unwrap() += 1;

            rsx! { "{boxed}" }
        },
        (render_count.clone(), signal_handle.clone()),
    );

    dom.rebuild_in_place();

    assert_eq!(*render_count.lock().unwrap(), 1);

    let mut signal = signal_handle.lock().unwrap().unwrap();
    signal.set(1);
    dom.render_immediate(&mut NoOpMutations);
    dom.render_immediate(&mut NoOpMutations);

    assert_eq!(*render_count.lock().unwrap(), 2);
}

// `point_to` must not panic when called on a wrapper whose `forwarding_context` was never lazily
// initialized (e.g. the wrapper has never been read). Locks in the `if let Some(forwarding_context)`
// branch's None path.
#[test]
fn point_to_on_never_read_wrapper_does_not_panic() {
    let captured = Rc::new(RefCell::new(None));

    let mut dom = VirtualDom::new_with_props(
        |captured: Rc<RefCell<Option<i32>>>| {
            let signal_a = use_signal(|| 7);
            let signal_b = use_signal(|| 42);

            // Build two wrappers without ever reading them.
            let target = ReadSignal::from(signal_a);
            let replacement = ReadSignal::from(signal_b);

            // Drive the never-initialized forwarding_context branch in `point_to`.
            target.point_to(replacement).unwrap();

            // Now read; should reflect signal_b's value.
            *captured.borrow_mut() = Some(target());

            rsx! { "{target}" }
        },
        captured.clone(),
    );

    dom.rebuild_in_place();

    assert_eq!(*captured.borrow(), Some(42));
}

// `point_to` with a copy of `self` must be a no-op. The naive implementation runs `manually_drop`
// on the slot just after repopulating it, which would invalidate the wrapper. Locks in the
// `self.inner == other.inner` early-return.
#[test]
fn point_to_self_is_noop() {
    let captured = Rc::new(RefCell::new(None));

    let mut dom = VirtualDom::new_with_props(
        |captured: Rc<RefCell<Option<i32>>>| {
            let signal = use_signal(|| 42);
            let target = ReadSignal::from(signal);

            // Force first-read init so the forwarding context exists. Self-point must still be a
            // no-op in this path.
            let _ = target();

            target.point_to(target).unwrap();

            // Slot must still be valid; this would panic if `manually_drop` had recycled it.
            *captured.borrow_mut() = Some(target());

            rsx! { "{target}" }
        },
        captured.clone(),
    );

    dom.rebuild_in_place();

    assert_eq!(*captured.borrow(), Some(42));
}

// The forwarding `ReactiveContext` must outlive the scope that first *read* the wrapper, since
// the wrapper itself may outlive that scope. We tie the forwarding context's generational slot
// to a per-wrapper owner so it shares the wrapper's lifetime. Pre-fix, the forwarding context
// was owned by the first reader's scope, so a child unmount silently broke reactivity for the
// still-alive wrapper.
#[test]
fn forwarding_context_survives_first_reader_scope_drop() {
    type Props = (
        Rc<RefCell<usize>>,
        Rc<RefCell<Option<Signal<i32>>>>,
        Rc<RefCell<Option<Signal<bool>>>>,
    );

    let parent_renders = Rc::new(RefCell::new(0usize));
    let signal_slot: Rc<RefCell<Option<Signal<i32>>>> = Rc::new(RefCell::new(None));
    let show_child_slot: Rc<RefCell<Option<Signal<bool>>>> = Rc::new(RefCell::new(None));

    let mut dom = VirtualDom::new_with_props(
        |(parent_renders, signal_slot, show_child_slot): Props| {
            let show_child = use_signal(|| true);
            let signal = use_signal(|| 0);
            *signal_slot.borrow_mut() = Some(signal);
            *show_child_slot.borrow_mut() = Some(show_child);

            // Persist the wrapper across renders so the child's first read is the wrapper's
            // first read. Wrapper is owned by this (parent) scope.
            let wrapper: ReadSignal<i32> = use_hook(|| ReadSignal::from(signal));

            *parent_renders.borrow_mut() += 1;

            rsx! {
                if show_child() {
                    Child { sig: wrapper }
                } else {
                    "after-child: {wrapper}"
                }
            }
        },
        (
            parent_renders.clone(),
            signal_slot.clone(),
            show_child_slot.clone(),
        ),
    );

    #[derive(Props, Clone, PartialEq)]
    struct ChildProps {
        sig: ReadSignal<i32>,
    }

    fn Child(props: ChildProps) -> Element {
        // First read of `wrapper` happens here, in the child scope. Pre-fix, the forwarding
        // context's owner was captured from this scope.
        let _ = (props.sig)();
        rsx! { "{props.sig}" }
    }

    dom.rebuild_in_place();
    let after_initial = *parent_renders.borrow();

    // Unmount the child. Pre-fix, this drops the forwarding context.
    let mut show_child = show_child_slot.borrow().unwrap();
    show_child.set(false);
    dom.mark_dirty(ScopeId::APP);
    dom.render_immediate(&mut NoOpMutations);
    dom.render_immediate(&mut NoOpMutations);
    let after_unmount = *parent_renders.borrow();
    assert!(after_unmount > after_initial);

    // Write to the underlying signal. Parent has subscribed via the "after-child" branch's
    // `{wrapper}` read, so it should re-render *if* the forwarding context is still alive.
    let mut signal = signal_slot.borrow().unwrap();
    signal.set(7);
    dom.render_immediate(&mut NoOpMutations);
    dom.render_immediate(&mut NoOpMutations);

    assert!(
        *parent_renders.borrow() > after_unmount,
        "parent should re-render on inner write: forwarding context must outlive the first-reader scope"
    );
}

// Sibling `point_to` calls on the same `other` slot — the rsx-clone scenario — must both
// succeed. The first one consumes `other`'s wrapped value and bumps its generation via
// `manually_drop`; the second sees `other` as dropped and short-circuits.
#[test]
fn point_to_tolerates_shared_other_slot() {
    let observed = Rc::new(RefCell::new(None));

    let mut dom = VirtualDom::new_with_props(
        |observed: Rc<RefCell<Option<(i32, i32)>>>| {
            let signal_a = use_signal(|| 1);
            let signal_b = use_signal(|| 99);

            // Two distinct `self` slots simulating two sibling components' stored props.sig.
            let target_a = ReadSignal::from(signal_a);
            let target_a2 = ReadSignal::from(signal_a);

            // One shared `other` slot used as the new value for both — the rsx-clone case.
            let replacement = ReadSignal::from(signal_b);

            target_a.point_to(replacement).unwrap();
            // Same `replacement` reused — must not panic on the dropped generational slot.
            target_a2.point_to(replacement).unwrap();

            *observed.borrow_mut() = Some((target_a(), target_a2()));
            rsx! { "{target_a}-{target_a2}" }
        },
        observed.clone(),
    );

    dom.rebuild_in_place();

    // First call moved `replacement`'s box into target_a. Second call short-circuited (other was
    // dropped/empty) and left target_a2 unchanged — still pointing at signal_a.
    assert_eq!(*observed.borrow(), Some((99, 1)));
}

