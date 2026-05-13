#![allow(unused, non_upper_case_globals, non_snake_case)]

use dioxus_core::{NoOpMutations, ReactiveContext, RuntimeGuard, current_scope_id, generation};
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use dioxus::prelude::*;
use dioxus_core::ElementId;
use dioxus_signals::*;
use std::cell::RefCell;

fn flush(dom: &mut VirtualDom) {
    dom.render_immediate(&mut NoOpMutations);
    dom.render_immediate(&mut NoOpMutations);
}

fn rerender_app(dom: &mut VirtualDom) {
    dom.mark_dirty(ScopeId::APP);
    flush(dom);
}

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

    rerender_app(&mut dom);

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
    rerender_app(&mut dom);

    {
        let current_counter = run_counter.borrow();
        assert_eq!(current_counter.parent_effect_runs, 1);
        assert_eq!(current_counter.child_renders, 1);
    }

    let mut signal_b = handles.borrow().signal_b.unwrap();
    signal_b.set(1);
    rerender_app(&mut dom);

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
    rerender_app(&mut dom);

    let mut signal_a = handles.borrow().signal_a.unwrap();
    signal_a.set(1);
    rerender_app(&mut dom);

    {
        let current_counter = run_counter.borrow();
        assert_eq!(current_counter.direct_effect_runs, 2);
        assert_eq!(current_counter.child_renders, 1);
    }

    let mut signal_b = handles.borrow().signal_b.unwrap();
    signal_b.set(1);
    rerender_app(&mut dom);

    {
        let current_counter = run_counter.borrow();
        assert_eq!(current_counter.direct_effect_runs, 2);
        assert_eq!(current_counter.child_renders, 2);
    }
}

#[test]
fn read_signal_point_to_preserves_overlapping_direct_source_subscription() {
    let dirty_count = Arc::new(AtomicUsize::new(0));

    let mut dom = VirtualDom::new_with_props(
        |dirty_count: Arc<AtomicUsize>| {
            let mut signal_a = use_signal(|| 0);
            let signal_b = use_signal(|| 0);
            let target = use_hook(|| ReadSignal::from(signal_a));
            let replacement = ReadSignal::from(signal_b);
            let dirty_count = dirty_count.clone();
            let context = ReactiveContext::new_with_callback(
                move || {
                    dirty_count.fetch_add(1, Ordering::SeqCst);
                },
                current_scope_id(),
                std::panic::Location::caller(),
            );

            context.run_in(|| {
                assert_eq!(signal_a(), 0);
                assert_eq!(target(), 0);
            });
            target.point_to(replacement).unwrap();
            signal_a.set(1);

            rsx! { "" }
        },
        dirty_count.clone(),
    );

    dom.rebuild_in_place();

    assert_eq!(dirty_count.load(Ordering::SeqCst), 1);
}

#[test]
fn read_signal_point_to_migrated_subscribers_can_unsubscribe() {
    type Props = (
        Rc<RefCell<Option<(Signal<i32>, ReactiveContext)>>>,
        Arc<AtomicUsize>,
    );

    let handles = Rc::new(RefCell::new(None));
    let dirty_count = Arc::new(AtomicUsize::new(0));

    let mut dom = VirtualDom::new_with_props(
        |(handles, dirty_count): Props| {
            let signal_a = use_signal(|| 0);
            let signal_b = use_signal(|| 0);
            let target = use_hook(|| ReadSignal::from(signal_a));
            let replacement = ReadSignal::from(signal_b);
            let context = use_hook({
                let dirty_count = dirty_count.clone();
                move || {
                    ReactiveContext::new_with_callback(
                        {
                            let dirty_count = dirty_count.clone();
                            move || {
                                dirty_count.fetch_add(1, Ordering::SeqCst);
                            }
                        },
                        current_scope_id(),
                        std::panic::Location::caller(),
                    )
                }
            });

            context.run_in(|| assert_eq!(target(), 0));
            target.point_to(replacement).unwrap();
            *handles.borrow_mut() = Some((signal_b, context));

            rsx! { "" }
        },
        (handles.clone(), dirty_count.clone()),
    );

    dom.rebuild_in_place();
    assert_eq!(dirty_count.load(Ordering::SeqCst), 0);

    let (mut signal_b, context) = handles.borrow().unwrap();
    context.reset_and_run_in(|| {});
    signal_b.set(1);

    assert_eq!(dirty_count.load(Ordering::SeqCst), 0);
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
    flush(&mut dom);

    {
        let current_render_count = render_count.borrow();
        assert_eq!(*current_render_count, 2);
    }
}

#[test]
fn boxed_read_signal_subscribers_bridge_before_tracked_read() {
    type Props = (Arc<AtomicUsize>, Rc<RefCell<Option<Signal<i32>>>>);

    let dirty_count = Arc::new(AtomicUsize::new(0));
    let signal_handle = Rc::new(RefCell::new(None));

    let mut dom = VirtualDom::new_with_props(
        |(dirty_count, signal_handle): Props| {
            let signal = use_signal(|| 0);
            *signal_handle.borrow_mut() = Some(signal);

            let boxed = ReadSignal::from(signal);
            let _context = use_hook({
                let dirty_count = dirty_count.clone();
                move || {
                    let context = ReactiveContext::new_with_callback(
                        {
                            let dirty_count = dirty_count.clone();
                            move || {
                                dirty_count.fetch_add(1, Ordering::SeqCst);
                            }
                        },
                        current_scope_id(),
                        std::panic::Location::caller(),
                    );
                    context.subscribe(boxed.subscribers());
                    context
                }
            });

            rsx! { "" }
        },
        (dirty_count.clone(), signal_handle.clone()),
    );

    dom.rebuild_in_place();
    assert_eq!(dirty_count.load(Ordering::SeqCst), 0);

    let mut signal = signal_handle.borrow().unwrap();
    signal.set(1);

    assert_eq!(dirty_count.load(Ordering::SeqCst), 1);
}

#[test]
fn boxed_read_signal_read_in_context_without_current_scope_does_not_panic() {
    type Props = (
        Rc<RefCell<Option<(Signal<i32>, ReadSignal<i32>, ReactiveContext)>>>,
        Arc<AtomicUsize>,
    );

    let captured = Rc::new(RefCell::new(None));
    let dirty_count = Arc::new(AtomicUsize::new(0));

    let mut dom = VirtualDom::new_with_props(
        |(captured, dirty_count): Props| {
            let signal = use_signal(|| 0);
            let boxed = ReadSignal::from(signal);
            let context = ReactiveContext::new_with_callback(
                {
                    let dirty_count = dirty_count.clone();
                    move || {
                        dirty_count.fetch_add(1, Ordering::SeqCst);
                    }
                },
                current_scope_id(),
                std::panic::Location::caller(),
            );

            *captured.borrow_mut() = Some((signal, boxed, context));

            rsx! { "" }
        },
        (captured.clone(), dirty_count.clone()),
    );

    dom.rebuild_in_place();

    let (mut signal, boxed, context) = captured.borrow().unwrap();
    {
        let _runtime = RuntimeGuard::new(dom.runtime());
        context.run_in(|| assert_eq!(boxed(), 0));
    }

    signal.set(1);

    assert_eq!(dirty_count.load(Ordering::SeqCst), 1);
}

#[test]
fn boxed_read_signal_try_read_in_context_returns_error_after_wrapped_signal_drops() {
    type Props = Rc<RefCell<Option<(Signal<i32>, ReadSignal<i32>, ReactiveContext)>>>;

    let captured = Rc::new(RefCell::new(None));

    let mut dom = VirtualDom::new_with_props(
        |captured: Props| {
            let signal = use_signal(|| 0);
            let boxed = ReadSignal::from(signal);
            let context = ReactiveContext::new_with_callback(
                || {},
                current_scope_id(),
                std::panic::Location::caller(),
            );

            *captured.borrow_mut() = Some((signal, boxed, context));

            rsx! { "" }
        },
        captured.clone(),
    );

    dom.rebuild_in_place();

    let (signal, boxed, context) = captured.borrow().unwrap();
    signal.manually_drop();

    let _runtime = RuntimeGuard::new(dom.runtime());
    context.run_in(|| {
        assert!(boxed.try_read().is_err());
    });
}

// `point_to` must not panic when called on a wrapper that has never been read.
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

// `point_to` with a copy of `self` must be a no-op.
#[test]
fn point_to_self_is_noop() {
    let captured = Rc::new(RefCell::new(None));

    let mut dom = VirtualDom::new_with_props(
        |captured: Rc<RefCell<Option<i32>>>| {
            let signal = use_signal(|| 42);
            let target = ReadSignal::from(signal);

            // Self-point must be a no-op after first-read init.
            let _ = target();

            target.point_to(target).unwrap();

            // Slot must still be valid.
            *captured.borrow_mut() = Some(target());

            rsx! { "{target}" }
        },
        captured.clone(),
    );

    dom.rebuild_in_place();

    assert_eq!(*captured.borrow(), Some(42));
}

// The forwarding context must outlive the first reader.
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

            // The child does the wrapper's first read.
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
        // First read of `wrapper` happens here, in the child scope.
        let _ = (props.sig)();
        rsx! { "{props.sig}" }
    }

    dom.rebuild_in_place();
    let after_initial = *parent_renders.borrow();

    // Unmount the first reader before reading the wrapper from the parent.
    let mut show_child = show_child_slot.borrow().unwrap();
    show_child.set(false);
    rerender_app(&mut dom);
    let after_unmount = *parent_renders.borrow();
    assert!(after_unmount > after_initial);

    // Parent should rerender if forwarding survived the child unmount.
    let mut signal = signal_slot.borrow().unwrap();
    signal.set(7);
    flush(&mut dom);

    assert!(
        *parent_renders.borrow() > after_unmount,
        "parent should re-render on inner write: forwarding context must outlive the first-reader scope"
    );
}

#[test]
fn point_to_keeps_source_handle_readable() {
    let observed = Rc::new(RefCell::new(None));

    let mut dom = VirtualDom::new_with_props(
        |observed: Rc<RefCell<Option<(i32, i32, i32)>>>| {
            let signal_a = use_signal(|| 7);
            let signal_b = use_signal(|| 42);

            let target = ReadSignal::from(signal_a);
            let source = ReadSignal::from(signal_b);
            let source_copy = source;

            target.point_to(source).unwrap();

            *observed.borrow_mut() = Some((target(), source(), source_copy()));
            rsx! { "{target}" }
        },
        observed.clone(),
    );

    dom.rebuild_in_place();

    assert_eq!(*observed.borrow(), Some((42, 42, 42)));
}

// Sibling `point_to` calls may share the same replacement slot.
#[test]
fn point_to_tolerates_shared_other_slot() {
    let observed = Rc::new(RefCell::new(None));

    let mut dom = VirtualDom::new_with_props(
        |observed: Rc<RefCell<Option<(i32, i32, i32)>>>| {
            let signal_a = use_signal(|| 1);
            let signal_b = use_signal(|| 99);

            // Two sibling stored props.
            let target_a = ReadSignal::from(signal_a);
            let target_a2 = ReadSignal::from(signal_a);

            // One shared replacement.
            let replacement = ReadSignal::from(signal_b);

            target_a.point_to(replacement).unwrap();
            // The replacement must remain reusable.
            target_a2.point_to(replacement).unwrap();

            *observed.borrow_mut() = Some((target_a(), target_a2(), replacement()));
            rsx! { "{target_a}-{target_a2}" }
        },
        observed.clone(),
    );

    dom.rebuild_in_place();

    assert_eq!(*observed.borrow(), Some((99, 99, 99)));
}

#[test]
fn failed_point_to_preserves_existing_wrapper_subscribers() {
    #[derive(Default)]
    struct Captured {
        signal_a: Option<Signal<i32>>,
        show_child: Option<Signal<bool>>,
        target: Option<ReadSignal<i32>>,
        stale_replacement: Option<ReadSignal<i32>>,
        context: Option<ReactiveContext>,
    }

    type Props = (Rc<RefCell<Captured>>, Arc<AtomicUsize>);

    #[derive(Props, Clone)]
    struct ReplacementOwnerProps {
        captured: Rc<RefCell<Captured>>,
    }

    impl PartialEq for ReplacementOwnerProps {
        fn eq(&self, other: &Self) -> bool {
            Rc::ptr_eq(&self.captured, &other.captured)
        }
    }

    let captured = Rc::new(RefCell::new(Captured::default()));
    let dirty_count = Arc::new(AtomicUsize::new(0));

    let mut dom = VirtualDom::new_with_props(
        |(captured, dirty_count): Props| {
            let signal_a = use_signal(|| 0);
            let show_child = use_signal(|| true);
            let target = use_hook(|| ReadSignal::from(signal_a));
            let context = use_hook({
                let dirty_count = dirty_count.clone();
                move || {
                    ReactiveContext::new_with_callback(
                        {
                            let dirty_count = dirty_count.clone();
                            move || {
                                dirty_count.fetch_add(1, Ordering::SeqCst);
                            }
                        },
                        current_scope_id(),
                        std::panic::Location::caller(),
                    )
                }
            });

            context.run_in(|| assert_eq!(target(), 0));

            {
                let mut captured = captured.borrow_mut();
                captured.signal_a = Some(signal_a);
                captured.show_child = Some(show_child);
                captured.target = Some(target);
                captured.context = Some(context);
            }

            rsx! {
                if show_child() {
                    ReplacementOwner { captured: captured.clone() }
                }
            }
        },
        (captured.clone(), dirty_count.clone()),
    );

    fn ReplacementOwner(props: ReplacementOwnerProps) -> Element {
        let signal_b = use_signal(|| 1);
        props.captured.borrow_mut().stale_replacement = Some(ReadSignal::from(signal_b));
        rsx! { "" }
    }

    dom.rebuild_in_place();
    assert_eq!(dirty_count.load(Ordering::SeqCst), 0);

    let mut show_child = captured.borrow().show_child.unwrap();
    show_child.set(false);
    rerender_app(&mut dom);

    let (mut signal_a, target, stale_replacement) = {
        let captured = captured.borrow();
        (
            captured.signal_a.unwrap(),
            captured.target.unwrap(),
            captured.stale_replacement.unwrap(),
        )
    };

    assert!(target.point_to(stale_replacement).is_err());

    signal_a.set(1);
    assert_eq!(
        dirty_count.load(Ordering::SeqCst),
        1,
        "failed point_to should leave the old wrapper subscriptions attached"
    );
}
