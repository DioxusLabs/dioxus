use dioxus::prelude::*;
use dioxus_core::{Mutation, Mutations, RuntimeGuard, ScopeId, UpdatePriority, VirtualDom};
use futures_util::FutureExt;
use std::cell::{Cell, RefCell};
use std::future::Future;
use std::pin::pin;
use std::task::{Context, Poll, Waker};
use std::{any::Any, rc::Rc};

fn app() -> Element {
    let generation = dioxus_core::generation();
    rsx! {
        div { "{generation}" }
    }
}

fn click_event() -> Event<dyn Any> {
    Event::new(
        Rc::new(PlatformEventData::new(Box::<SerializedMouseData>::default())) as Rc<dyn Any>,
        true,
    )
}

thread_local! {
    static CHILD_A_SIGNAL: RefCell<Option<Signal<i32>>> = const { RefCell::new(None) };
    static EFFECT_COMMIT_SEEN: Cell<bool> = const { Cell::new(false) };
    static EFFECT_SIGNAL: RefCell<Option<Signal<i32>>> = const { RefCell::new(None) };
    static EFFECT_VALUES: RefCell<Vec<i32>> = const { RefCell::new(Vec::new()) };
    static IMMEDIATE_PARENT_SIGNAL: RefCell<Option<Signal<u32>>> = const { RefCell::new(None) };
    static IMMEDIATE_CHILD_RENDERS: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
    static PARENT_ROUND_SIGNAL: RefCell<Option<Signal<u32>>> = const { RefCell::new(None) };
    static PARENT_TICK_SIGNAL: RefCell<Option<Signal<u32>>> = const { RefCell::new(None) };
    /// Records the `UpdatePriority` each instrumented component renders at,
    /// in the order rendering happens. Tests that need to observe scheduler
    /// ordering without the deleted callback API read from this trace.
    static RENDER_PRIORITY_TRACE: RefCell<Vec<(ScopeId, UpdatePriority)>> = const {
        RefCell::new(Vec::new())
    };
}

fn record_render_priority() {
    let scope = dioxus_core::current_scope_id();
    let priority = dioxus_core::Runtime::current().current_update_priority();
    RENDER_PRIORITY_TRACE.with_borrow_mut(|trace| trace.push((scope, priority)));
}

fn take_priority_trace() -> Vec<(ScopeId, UpdatePriority)> {
    RENDER_PRIORITY_TRACE.with_borrow_mut(std::mem::take)
}

fn reset_priority_trace() {
    RENDER_PRIORITY_TRACE.with_borrow_mut(Vec::clear);
}

/// Poll a future once with a no-op waker. Useful for stepping
/// `render_concurrent` one work unit at a time so the test can inject signal
/// writes or events between cooperative yields.
fn step<F: Future + Unpin>(fut: &mut F) -> Poll<F::Output> {
    let waker = Waker::noop();
    let mut cx = Context::from_waker(waker);
    fut.poll_unpin(&mut cx)
}

/// Drive a future to completion under a no-op waker, returning its output.
/// This is the no-runtime equivalent of `.await` — the render driver wakes
/// itself, so a tight poll loop always makes progress.
fn drive<F: Future>(fut: F) -> F::Output {
    let mut fut = pin!(fut);
    let waker = Waker::noop();
    let mut cx = Context::from_waker(waker);
    loop {
        if let Poll::Ready(value) = fut.as_mut().poll(&mut cx) {
            return value;
        }
    }
}

#[tokio::test]
async fn concurrent_render_writes_to_mutation_queue() {
    let mut dom = VirtualDom::new(app);
    dom.rebuild();

    dom.mark_dirty_with_priority(ScopeId::APP, UpdatePriority::Transition);
    let (stats, mut mutations) = dom.render_concurrent_with(Mutations::default()).await;

    assert_eq!(stats.generation, 1);
    assert_eq!(stats.priority, UpdatePriority::Transition);
    assert_eq!(stats.commit_count, 1);
    assert!(!mutations.edits.is_empty());
}

#[tokio::test]
async fn concurrent_render_commits_each_work_unit() {
    let mut dom = VirtualDom::new(app);
    dom.rebuild();

    dom.mark_dirty_with_priority(ScopeId::APP, UpdatePriority::SyncInput);
    let (stats, mut mutations) = dom.render_concurrent_with(Mutations::default()).await;

    assert_eq!(stats.priority, UpdatePriority::SyncInput);
    assert_eq!(stats.work_count, 1);
    assert_eq!(stats.commit_count, 1);
    assert_eq!(stats.yield_count, 1);
    assert!(!mutations.edits.is_empty());
}

#[test]
fn render_immediate_drains_deferred_child_prop_work() {
    #[component]
    fn ImmediateChild(round: u32) -> Element {
        IMMEDIATE_CHILD_RENDERS.with_borrow_mut(|renders| renders.push(round));
        rsx! { span { "{round}" } }
    }

    fn immediate_app() -> Element {
        let round = use_signal(|| 0);
        IMMEDIATE_PARENT_SIGNAL.with_borrow_mut(|slot| *slot = Some(round));

        rsx! {
            ImmediateChild { round: round() }
        }
    }

    IMMEDIATE_PARENT_SIGNAL.with_borrow_mut(|slot| *slot = None);
    IMMEDIATE_CHILD_RENDERS.with_borrow_mut(Vec::clear);

    let mut dom = VirtualDom::new(immediate_app);
    dom.rebuild();
    IMMEDIATE_CHILD_RENDERS.with_borrow_mut(Vec::clear);

    {
        let _runtime = RuntimeGuard::new(dom.runtime());
        IMMEDIATE_PARENT_SIGNAL.with_borrow(|slot| {
            let mut round = slot.expect("parent signal should be registered");
            round += 1;
        });
    }

    let mut mutations = dom.render_immediate_to_vec();

    assert_eq!(IMMEDIATE_CHILD_RENDERS.with_borrow(Clone::clone), vec![1]);
    assert!(
        mutations
            .edits
            .iter()
            .any(|mutation| matches!(mutation, Mutation::SetText { value, .. } if value == "1"))
    );
}

#[tokio::test]
async fn scheduler_commits_before_new_urgent_work() {
    fn child_a() -> Element {
        let count = use_signal(|| 0);
        CHILD_A_SIGNAL.with_borrow_mut(|slot| *slot = Some(count));
        record_render_priority();
        let generation = dioxus_core::generation();
        rsx! { div { "a {generation} {count}" } }
    }

    fn child_b() -> Element {
        record_render_priority();
        let generation = dioxus_core::generation();
        rsx! { div { "b {generation}" } }
    }

    fn preemption_app() -> Element {
        rsx! {
            child_a {}
            child_b {}
        }
    }

    CHILD_A_SIGNAL.with_borrow_mut(|slot| *slot = None);

    let mut dom = VirtualDom::new(preemption_app);
    dom.rebuild();
    let runtime = dom.runtime();

    dom.mark_dirty_with_priority(ScopeId(4), UpdatePriority::Transition);
    dom.mark_dirty_with_priority(ScopeId(5), UpdatePriority::Transition);

    reset_priority_trace();
    dom.insert_render_target(RenderTargetId::ROOT, Mutations::default());
    {
        let mut fut = pin!(dom.render_concurrent());
        // Advance one work unit. With both scopes dirty at Transition, the first
        // child renders at Transition; we then inject a SyncInput-priority update
        // and expect the scheduler to commit and serve the urgent work next.
        assert!(matches!(step(&mut fut), Poll::Pending));
        CHILD_A_SIGNAL.with_borrow(|slot| {
            let mut signal = slot.expect("child signal should be registered");
            let _runtime = RuntimeGuard::new(runtime.clone());
            dioxus_core::with_update_priority(UpdatePriority::SyncInput, || {
                signal += 1;
            });
        });
        loop {
            if let Poll::Ready(_) = step(&mut fut) {
                break;
            }
        }
    }

    let trace = take_priority_trace();
    assert_eq!(
        trace.first().map(|(_, p)| *p),
        Some(UpdatePriority::Transition),
        "first work unit should render at Transition: {trace:?}"
    );
    assert!(
        trace
            .iter()
            .any(|(_, p)| *p == UpdatePriority::SyncInput),
        "urgent SyncInput work should preempt the transition: {trace:?}"
    );
    let mutations = dom
        .take_render_target::<Mutations>(RenderTargetId::ROOT)
        .expect("ROOT writer was registered before render_concurrent");
    assert!(
        !mutations.edits.is_empty(),
        "renderer must see committed mutations"
    );
}

#[tokio::test]
async fn higher_priority_scope_render_preserves_lower_priority_lane() {
    fn priority_app() -> Element {
        record_render_priority();
        let generation = dioxus_core::generation();
        rsx! { div { "{generation}" } }
    }

    let mut dom = VirtualDom::new(priority_app);
    dom.rebuild();

    dom.mark_dirty_with_priority(ScopeId::APP, UpdatePriority::Transition);
    dom.mark_dirty_with_priority(ScopeId::APP, UpdatePriority::ContinuousInput);

    reset_priority_trace();
    let (_, mut mutations) = dom.render_concurrent_with(Mutations::default()).await;

    let trace = take_priority_trace();
    assert_eq!(
        trace,
        vec![
            (ScopeId::APP, UpdatePriority::ContinuousInput),
            (ScopeId::APP, UpdatePriority::Transition),
        ],
        "the same scope should re-render once at each priority lane"
    );
}

#[tokio::test]
async fn dirty_parent_runs_before_more_urgent_child() {
    #[allow(non_snake_case)]
    fn Child() -> Element {
        record_render_priority();
        rsx! { div { "child" } }
    }

    fn parent_app() -> Element {
        record_render_priority();
        rsx! { Child {} }
    }

    let mut dom = VirtualDom::new(parent_app);
    dom.rebuild();

    dom.mark_dirty_with_priority(ScopeId::APP, UpdatePriority::Transition);
    dom.mark_dirty_with_priority(ScopeId(4), UpdatePriority::SyncInput);

    reset_priority_trace();
    let (_, mut mutations) = dom.render_concurrent_with(Mutations::default()).await;

    let trace = take_priority_trace();
    assert_eq!(
        trace.first(),
        Some(&(ScopeId::APP, UpdatePriority::Transition)),
        "the dirty parent must render before any urgent child work: {trace:?}"
    );
    assert!(
        trace
            .iter()
            .any(|entry| *entry == (ScopeId(4), UpdatePriority::SyncInput)),
        "the urgent child must still render: {trace:?}"
    );
}

#[tokio::test]
async fn memoized_dirty_child_is_not_promoted_by_parent_lane() {
    #[component]
    fn PropChild(id: usize, round: u32) -> Element {
        record_render_priority();
        rsx! { div { "{id}:{round}" } }
    }

    fn parent_app() -> Element {
        record_render_priority();
        let round = use_signal(|| 0);
        let tick = use_signal(|| 0);
        PARENT_ROUND_SIGNAL.with_borrow_mut(|slot| *slot = Some(round));
        PARENT_TICK_SIGNAL.with_borrow_mut(|slot| *slot = Some(tick));

        rsx! {
            div { "{tick}" }
            for id in 0..8 {
                PropChild { key: "{id}", id, round: round() }
            }
        }
    }

    PARENT_ROUND_SIGNAL.with_borrow_mut(|slot| *slot = None);
    PARENT_TICK_SIGNAL.with_borrow_mut(|slot| *slot = None);

    let mut dom = VirtualDom::new(parent_app);
    dom.rebuild();
    let runtime = dom.runtime();

    PARENT_ROUND_SIGNAL.with_borrow(|slot| {
        let mut round = slot.expect("parent round signal should be registered");
        let _runtime = RuntimeGuard::new(runtime.clone());
        dioxus_core::with_update_priority(UpdatePriority::Transition, || {
            round += 1;
        });
    });

    reset_priority_trace();
    dom.insert_render_target(RenderTargetId::ROOT, Mutations::default());
    let mut fut = pin!(dom.render_concurrent());

    // First step: parent renders at Transition, queueing memoized child prop
    // diffs at the Transition lane. Now inject a ContinuousInput tick.
    assert!(matches!(step(&mut fut), Poll::Pending));
    PARENT_TICK_SIGNAL.with_borrow(|slot| {
        let mut tick = slot.expect("parent tick signal should be registered");
        let _runtime = RuntimeGuard::new(runtime.clone());
        dioxus_core::with_update_priority(UpdatePriority::ContinuousInput, || {
            tick += 1;
        });
    });

    // Drain the rest while injecting one more tick after we observe the
    // continuous-input render so the deferred child work has to be resumed
    // by the scheduler after a higher-priority preemption.
    let mut second_tick_done = false;
    loop {
        match step(&mut fut) {
            Poll::Ready(_) => break,
            Poll::Pending => {
                if !second_tick_done {
                    let trace = RENDER_PRIORITY_TRACE.with_borrow(|t| t.clone());
                    if trace
                        .iter()
                        .any(|(_, p)| *p == UpdatePriority::ContinuousInput)
                    {
                        second_tick_done = true;
                        PARENT_TICK_SIGNAL.with_borrow(|slot| {
                            let mut tick = slot.expect("parent tick signal should be registered");
                            let _runtime = RuntimeGuard::new(runtime.clone());
                            dioxus_core::with_update_priority(
                                UpdatePriority::ContinuousInput,
                                || {
                                    tick += 1;
                                },
                            );
                        });
                    }
                }
            }
        }
    }
    drop(fut);

    let trace = take_priority_trace();
    assert_eq!(
        trace.first(),
        Some(&(ScopeId::APP, UpdatePriority::Transition)),
        "the dirty parent runs at its original Transition lane: {trace:?}"
    );
    // After the continuous-input preemption, the deferred PropChild fibers
    // must still be served at Transition rather than being promoted into the
    // ContinuousInput lane just because the parent committed urgently.
    assert!(
        trace
            .iter()
            .skip_while(|(_, p)| *p != UpdatePriority::ContinuousInput)
            .any(|(scope, p)| *scope != ScopeId::APP && *p == UpdatePriority::Transition),
        "memoized children should re-render at Transition: {trace:?}"
    );
}

#[tokio::test]
async fn deferred_child_props_survive_parent_continuous_commit() {
    #[component]
    fn PropChild(round: u32) -> Element {
        record_render_priority();
        rsx! { div { "{round}" } }
    }

    fn parent_app() -> Element {
        record_render_priority();
        let round = use_signal(|| 0);
        let tick = use_signal(|| 0);
        PARENT_ROUND_SIGNAL.with_borrow_mut(|slot| *slot = Some(round));
        PARENT_TICK_SIGNAL.with_borrow_mut(|slot| *slot = Some(tick));

        rsx! {
            div { "{round}" }
            div { "{tick}" }
            PropChild { round: round() }
        }
    }

    PARENT_ROUND_SIGNAL.with_borrow_mut(|slot| *slot = None);
    PARENT_TICK_SIGNAL.with_borrow_mut(|slot| *slot = None);

    let mut dom = VirtualDom::new(parent_app);
    dom.rebuild();
    let runtime = dom.runtime();

    PARENT_TICK_SIGNAL.with_borrow(|slot| {
        let mut tick = slot.expect("parent tick signal should be registered");
        let _runtime = RuntimeGuard::new(runtime.clone());
        dioxus_core::with_update_priority(UpdatePriority::ContinuousInput, || {
            tick += 1;
        });
    });

    PARENT_ROUND_SIGNAL.with_borrow(|slot| {
        let mut round = slot.expect("parent round signal should be registered");
        let _runtime = RuntimeGuard::new(runtime);
        dioxus_core::with_update_priority(UpdatePriority::Transition, || {
            round += 1;
        });
    });

    reset_priority_trace();
    let (_, mut mutations) = dom.render_concurrent_with(Mutations::default()).await;

    let trace = take_priority_trace();
    assert_eq!(
        trace.first(),
        Some(&(ScopeId::APP, UpdatePriority::ContinuousInput)),
        "parent must commit at ContinuousInput first: {trace:?}"
    );
    assert!(
        trace
            .iter()
            .any(|entry| *entry == (ScopeId::APP, UpdatePriority::Transition)),
        "parent must also re-render at Transition: {trace:?}"
    );
    assert!(
        trace
            .iter()
            .any(|(scope, p)| *scope != ScopeId::APP && *p == UpdatePriority::Transition),
        "deferred child must run at Transition: {trace:?}"
    );
}

#[tokio::test]
async fn recursive_transition_tree_progresses_under_continuous_root_updates() {
    const DEPTH: u8 = 4;

    #[component]
    fn Leaf(seconds: u32) -> Element {
        rsx! { span { "leaf:{seconds}" } }
    }

    #[component]
    fn RecursiveTree(depth: u8, seconds: u32) -> Element {
        if depth == 0 {
            return rsx! { Leaf { seconds } };
        }

        rsx! {
            RecursiveTree { depth: depth - 1, seconds }
            RecursiveTree { depth: depth - 1, seconds }
            RecursiveTree { depth: depth - 1, seconds }
        }
    }

    fn recursive_app() -> Element {
        let round = use_signal(|| 0);
        let tick = use_signal(|| 0);
        PARENT_ROUND_SIGNAL.with_borrow_mut(|slot| *slot = Some(round));
        PARENT_TICK_SIGNAL.with_borrow_mut(|slot| *slot = Some(tick));

        rsx! {
            div { "metric:{round}" }
            div { "tick:{tick}" }
            RecursiveTree { depth: DEPTH, seconds: round() }
        }
    }

    fn set_round(runtime: &Rc<dioxus_core::Runtime>, priority: UpdatePriority) {
        PARENT_ROUND_SIGNAL.with_borrow(|slot| {
            let mut round = slot.expect("parent round signal should be registered");
            let _runtime = RuntimeGuard::new(runtime.clone());
            dioxus_core::with_update_priority(priority, || {
                round += 1;
            });
        });
    }

    fn tick(runtime: &Rc<dioxus_core::Runtime>) {
        PARENT_TICK_SIGNAL.with_borrow(|slot| {
            let mut tick = slot.expect("parent tick signal should be registered");
            let _runtime = RuntimeGuard::new(runtime.clone());
            dioxus_core::with_update_priority(UpdatePriority::ContinuousInput, || {
                tick += 1;
            });
        });
    }

    PARENT_ROUND_SIGNAL.with_borrow_mut(|slot| *slot = None);
    PARENT_TICK_SIGNAL.with_borrow_mut(|slot| *slot = None);

    let mut dom = VirtualDom::new(recursive_app);
    dom.rebuild();
    let runtime = dom.runtime();
    let expected_leaf_updates = 3_usize.pow(DEPTH as u32);

    set_round(&runtime, UpdatePriority::Transition);

    dom.insert_render_target(RenderTargetId::ROOT, Mutations::default());
    {
        let mut fut = pin!(dom.render_concurrent());
        let mut ticks = 0;
        loop {
            match step(&mut fut) {
                Poll::Ready(_) => break,
                Poll::Pending => {
                    if ticks < 800 {
                        ticks += 1;
                        tick(&runtime);
                    }
                }
            }
        }
    }

    let mutations = dom
        .take_render_target::<Mutations>(RenderTargetId::ROOT)
        .expect("ROOT writer was registered before render_concurrent");
    let saw_metric = mutations.edits.iter().any(
        |mutation| matches!(mutation, Mutation::SetText { value, .. } if value == "metric:1"),
    );
    let leaf_updates = mutations
        .edits
        .iter()
        .filter(
            |mutation| matches!(mutation, Mutation::SetText { value, .. } if value == "leaf:1"),
        )
        .count();

    assert!(saw_metric, "root transition text should commit");
    assert!(
        leaf_updates >= expected_leaf_updates,
        "all recursive transition leaf text should commit; saw {leaf_updates}/{expected_leaf_updates}"
    );
}

#[tokio::test]
async fn demo_sized_recursive_transition_reaches_visible_leaf_early() {
    const DEPTH: u8 = 7;

    #[component]
    fn Dot(seconds: u32) -> Element {
        rsx! { span { "dot:{seconds}" } }
    }

    #[component]
    fn Triangle(depth: u8, seconds: u32) -> Element {
        if depth == 0 {
            return rsx! { Dot { seconds } };
        }

        rsx! {
            Triangle { depth: depth - 1, seconds }
            Triangle { depth: depth - 1, seconds }
            Triangle { depth: depth - 1, seconds }
        }
    }

    fn triangle_app() -> Element {
        let seconds = use_signal(|| 0);
        let tick = use_signal(|| 0);
        PARENT_ROUND_SIGNAL.with_borrow_mut(|slot| *slot = Some(seconds));
        PARENT_TICK_SIGNAL.with_borrow_mut(|slot| *slot = Some(tick));

        rsx! {
            div { "tick:{tick}" }
            Triangle { depth: DEPTH, seconds: seconds() }
        }
    }

    fn set_seconds(runtime: &Rc<dioxus_core::Runtime>) {
        PARENT_ROUND_SIGNAL.with_borrow(|slot| {
            let mut seconds = slot.expect("seconds signal should be registered");
            let _runtime = RuntimeGuard::new(runtime.clone());
            dioxus_core::with_update_priority(UpdatePriority::Transition, || {
                seconds += 1;
            });
        });
    }

    fn tick(runtime: &Rc<dioxus_core::Runtime>) {
        PARENT_TICK_SIGNAL.with_borrow(|slot| {
            let mut tick = slot.expect("tick signal should be registered");
            let _runtime = RuntimeGuard::new(runtime.clone());
            dioxus_core::with_update_priority(UpdatePriority::ContinuousInput, || {
                tick += 1;
            });
        });
    }

    PARENT_ROUND_SIGNAL.with_borrow_mut(|slot| *slot = None);
    PARENT_TICK_SIGNAL.with_borrow_mut(|slot| *slot = None);

    let mut dom = VirtualDom::new(triangle_app);
    dom.rebuild();
    let runtime = dom.runtime();

    set_seconds(&runtime);

    dom.insert_render_target(RenderTargetId::ROOT, Mutations::default());
    {
        let mut fut = pin!(dom.render_concurrent());
        let mut ticks = 0;
        loop {
            match step(&mut fut) {
                Poll::Ready(_) => break,
                Poll::Pending => {
                    if ticks < 256 {
                        ticks += 1;
                        tick(&runtime);
                    }
                }
            }
        }
    }

    let mutations = dom
        .take_render_target::<Mutations>(RenderTargetId::ROOT)
        .expect("ROOT writer was registered before render_concurrent");
    let saw_leaf_update = mutations.edits.iter().any(
        |mutation| matches!(mutation, Mutation::SetText { value, .. } if value == "dot:1"),
    );
    assert!(
        saw_leaf_update,
        "demo-sized transition should reach visible leaf text"
    );
}

#[tokio::test]
async fn host_yield_lets_continuous_input_preempt_active_transition_lane() {
    #[component]
    fn Leaf(seconds: u32) -> Element {
        record_render_priority();
        rsx! { span { "{seconds}" } }
    }

    #[component]
    fn RecursiveTree(depth: u8, seconds: u32) -> Element {
        record_render_priority();
        if depth == 0 {
            return rsx! { Leaf { seconds } };
        }

        rsx! {
            RecursiveTree { depth: depth - 1, seconds }
            RecursiveTree { depth: depth - 1, seconds }
            RecursiveTree { depth: depth - 1, seconds }
        }
    }

    fn recursive_app() -> Element {
        record_render_priority();
        let round = use_signal(|| 0);
        let tick = use_signal(|| 0);
        PARENT_ROUND_SIGNAL.with_borrow_mut(|slot| *slot = Some(round));
        PARENT_TICK_SIGNAL.with_borrow_mut(|slot| *slot = Some(tick));

        rsx! {
            div { "tick:{tick}" }
            RecursiveTree { depth: 4, seconds: round() }
        }
    }

    fn set_round(runtime: &Rc<dioxus_core::Runtime>) {
        PARENT_ROUND_SIGNAL.with_borrow(|slot| {
            let mut round = slot.expect("parent round signal should be registered");
            let _runtime = RuntimeGuard::new(runtime.clone());
            dioxus_core::with_update_priority(UpdatePriority::Transition, || {
                round += 1;
            });
        });
    }

    fn tick(runtime: &Rc<dioxus_core::Runtime>) {
        PARENT_TICK_SIGNAL.with_borrow(|slot| {
            let mut tick = slot.expect("parent tick signal should be registered");
            let _runtime = RuntimeGuard::new(runtime.clone());
            dioxus_core::with_update_priority(UpdatePriority::ContinuousInput, || {
                tick += 1;
            });
        });
    }

    PARENT_ROUND_SIGNAL.with_borrow_mut(|slot| *slot = None);
    PARENT_TICK_SIGNAL.with_borrow_mut(|slot| *slot = None);

    let mut dom = VirtualDom::new(recursive_app);
    dom.rebuild();
    let runtime = dom.runtime();

    set_round(&runtime);

    reset_priority_trace();
    dom.insert_render_target(RenderTargetId::ROOT, Mutations::default());
    let mut fut = pin!(dom.render_concurrent());
    let mut ticks = 0;
    loop {
        match step(&mut fut) {
            Poll::Ready(_) => break,
            Poll::Pending => {
                if ticks < 32 {
                    ticks += 1;
                    tick(&runtime);
                }
            }
        }
    }
    drop(fut);

    let trace = take_priority_trace();
    let first_transition_idx = trace
        .iter()
        .position(|(_, p)| *p == UpdatePriority::Transition);
    assert!(first_transition_idx.is_some(), "fairness should eventually run transition work: {trace:?}");
    let after_transition = first_transition_idx.unwrap();
    assert!(
        trace[after_transition + 1..]
            .iter()
            .any(|(_, p)| *p == UpdatePriority::ContinuousInput),
        "fresh continuous input should preempt a transition lane after a yield: {trace:?}"
    );
}

#[tokio::test]
async fn child_prop_updates_are_scheduled_as_separate_fibers() {
    #[component]
    fn PropChild(id: usize, round: u32) -> Element {
        rsx! { div { "{id}:{round}" } }
    }

    fn parent_app() -> Element {
        let round = use_signal(|| 0);
        PARENT_ROUND_SIGNAL.with_borrow_mut(|slot| *slot = Some(round));

        rsx! {
            for id in 0..40 {
                PropChild { key: "{id}", id, round: round() }
            }
        }
    }

    PARENT_ROUND_SIGNAL.with_borrow_mut(|slot| *slot = None);

    let mut dom = VirtualDom::new(parent_app);
    dom.rebuild();

    PARENT_ROUND_SIGNAL.with_borrow(|slot| {
        let mut round = slot.expect("parent signal should be registered");
        let _runtime = RuntimeGuard::new(dom.runtime());
        dioxus_core::with_update_priority(UpdatePriority::Transition, || {
            round += 1;
        });
    });

    let (stats, mut mutations) = dom.render_concurrent_with(Mutations::default()).await;

    assert_eq!(stats.priority, UpdatePriority::Transition);
    assert_eq!(stats.work_count, 44);
    assert_eq!(stats.commit_count, 44);
    assert_eq!(stats.yield_count, 44);
    assert!(!mutations.edits.is_empty());
}

#[tokio::test]
async fn concurrent_render_without_work_does_not_commit() {
    let mut dom = VirtualDom::new(app);
    dom.rebuild();

    let (stats, mut mutations) = dom.render_concurrent_with(Mutations::default()).await;
    assert_eq!(stats.generation, 0);
    assert_eq!(stats.priority, UpdatePriority::Idle);
    assert_eq!(stats.commit_count, 0);
    assert!(mutations.edits.is_empty());

    dom.mark_dirty_with_priority(ScopeId::APP, UpdatePriority::Default);
    let stats = dom.render_concurrent_into(&mut mutations).await;
    assert_eq!(stats.generation, 1);
    assert_eq!(stats.commit_count, 1);
    assert!(!mutations.edits.is_empty());

    mutations.edits.clear();
    let stats = dom.render_concurrent_into(&mut mutations).await;
    assert_eq!(stats.generation, 0);
    assert_eq!(stats.commit_count, 0);
    assert!(mutations.edits.is_empty());
}

#[tokio::test]
async fn event_priority_flows_into_concurrent_render() {
    set_event_converter(Box::new(dioxus::html::SerializedHtmlEventConverter));

    fn event_app() -> Element {
        let mut count = use_signal(|| 0);
        rsx! {
            button {
                onclick: move |_| count += 1,
                "{count}"
            }
        }
    }

    let mut dom = VirtualDom::new(event_app);
    let mut mutations = dom.rebuild_to_vec();
    let button = mutations
        .edits
        .iter()
        .find_map(|mutation| match mutation {
            Mutation::NewEventListener { id, .. } => Some(*id),
            _ => None,
        })
        .expect("button should have an event listener");

    dom.runtime().handle_event("click", click_event(), button);

    let (stats, mut mutations) = dom.render_concurrent_with(Mutations::default()).await;
    assert_eq!(stats.priority, UpdatePriority::SyncInput);
    assert!(!mutations.edits.is_empty());
}

#[tokio::test]
async fn urgent_work_preempts_resumed_transition_diff() {
    fn child_a() -> Element {
        let count = use_signal(|| 0);
        CHILD_A_SIGNAL.with_borrow_mut(|slot| *slot = Some(count));
        record_render_priority();
        let generation = dioxus_core::generation();
        rsx! { div { "a {generation} {count}" } }
    }

    fn child_b() -> Element {
        record_render_priority();
        let generation = dioxus_core::generation();
        rsx! { div { "b {generation}" } }
    }

    fn preemption_app() -> Element {
        rsx! {
            child_a {}
            child_b {}
        }
    }

    CHILD_A_SIGNAL.with_borrow_mut(|slot| *slot = None);

    let mut dom = VirtualDom::new(preemption_app);
    dom.rebuild();

    dom.mark_dirty_with_priority(ScopeId(4), UpdatePriority::Transition);
    dom.mark_dirty_with_priority(ScopeId(5), UpdatePriority::Transition);

    let runtime = dom.runtime();
    reset_priority_trace();
    dom.insert_render_target(RenderTargetId::ROOT, Mutations::default());
    let mut fut = pin!(dom.render_concurrent());

    // After the first transition work unit commits, inject the urgent update.
    assert!(matches!(step(&mut fut), Poll::Pending));
    CHILD_A_SIGNAL.with_borrow(|slot| {
        let mut signal = slot.expect("child signal should be registered");
        let _runtime = RuntimeGuard::new(runtime.clone());
        dioxus_core::with_update_priority(UpdatePriority::SyncInput, || {
            signal += 1;
        });
    });

    let stats = drive(&mut fut);
    drop(fut);

    let priorities: Vec<UpdatePriority> = take_priority_trace()
        .into_iter()
        .map(|(_, p)| p)
        .collect();
    assert_eq!(
        priorities,
        vec![
            UpdatePriority::Transition,
            UpdatePriority::SyncInput,
            UpdatePriority::Transition,
        ],
        "the urgent work must commit between the two transition fibers"
    );
    assert_eq!(stats.work_count, 3);
    assert_eq!(stats.commit_count, 3);
    assert_eq!(stats.yield_count, 3);
}

#[tokio::test]
async fn sync_work_commits_before_resuming_lower_priority_work() {
    fn child_a() -> Element {
        let count = use_signal(|| 0);
        CHILD_A_SIGNAL.with_borrow_mut(|slot| *slot = Some(count));
        record_render_priority();
        let generation = dioxus_core::generation();
        rsx! { div { "a {generation} {count}" } }
    }

    fn child_b() -> Element {
        record_render_priority();
        let generation = dioxus_core::generation();
        rsx! { div { "b {generation}" } }
    }

    fn preemption_app() -> Element {
        rsx! {
            child_a {}
            child_b {}
        }
    }

    CHILD_A_SIGNAL.with_borrow_mut(|slot| *slot = None);

    let mut dom = VirtualDom::new(preemption_app);
    dom.rebuild();

    dom.mark_dirty_with_priority(ScopeId(5), UpdatePriority::Transition);

    CHILD_A_SIGNAL.with_borrow(|slot| {
        let mut signal = slot.expect("child signal should be registered");
        let _runtime = RuntimeGuard::new(dom.runtime());
        dioxus_core::with_update_priority(UpdatePriority::SyncInput, || {
            signal += 1;
        });
    });

    reset_priority_trace();
    let (stats, mut mutations) = dom.render_concurrent_with(Mutations::default()).await;

    let priorities: Vec<UpdatePriority> = take_priority_trace()
        .into_iter()
        .map(|(_, p)| p)
        .collect();
    assert_eq!(
        priorities,
        vec![UpdatePriority::SyncInput, UpdatePriority::Transition],
        "sync work must run before resuming the transition lane"
    );
    assert_eq!(stats.work_count, 2);
    assert_eq!(stats.commit_count, 2);
}

#[tokio::test]
async fn large_child_list_prop_update_resumes_without_stale_mounts() {
    #[component]
    fn PropChild(id: usize, round: u32) -> Element {
        rsx! { div { "{id}:{round}" } }
    }

    fn parent_app() -> Element {
        let round = use_signal(|| 0);
        PARENT_ROUND_SIGNAL.with_borrow_mut(|slot| *slot = Some(round));

        rsx! {
            for id in 0..64 {
                PropChild { key: "{id}", id, round: round() }
            }
        }
    }

    PARENT_ROUND_SIGNAL.with_borrow_mut(|slot| *slot = None);

    let mut dom = VirtualDom::new(parent_app);
    dom.rebuild();

    PARENT_ROUND_SIGNAL.with_borrow(|slot| {
        let mut round = slot.expect("parent signal should be registered");
        let _runtime = RuntimeGuard::new(dom.runtime());
        dioxus_core::with_update_priority(UpdatePriority::Transition, || {
            round += 1;
        });
    });

    let (stats, mut mutations) = dom.render_concurrent_with(Mutations::default()).await;

    assert!(stats.work_count > 4);
    assert!(stats.yield_count > 4);
}

#[tokio::test]
async fn work_queued_by_final_commit_is_rendered_before_return() {
    fn child_a() -> Element {
        let count = use_signal(|| 0);
        CHILD_A_SIGNAL.with_borrow_mut(|slot| *slot = Some(count));
        record_render_priority();
        let generation = dioxus_core::generation();
        rsx! { div { "a {generation} {count}" } }
    }

    fn final_commit_app() -> Element {
        rsx! { child_a {} }
    }

    CHILD_A_SIGNAL.with_borrow_mut(|slot| *slot = None);

    let mut dom = VirtualDom::new(final_commit_app);
    dom.rebuild();

    dom.mark_dirty_with_priority(ScopeId(4), UpdatePriority::Transition);

    let runtime = dom.runtime();
    reset_priority_trace();
    dom.insert_render_target(RenderTargetId::ROOT, Mutations::default());
    let mut fut = pin!(dom.render_concurrent());

    // First step renders the transition work. We then queue urgent work; the
    // future must not return until that work has been rendered too.
    assert!(matches!(step(&mut fut), Poll::Pending));
    CHILD_A_SIGNAL.with_borrow(|slot| {
        let mut signal = slot.expect("child signal should be registered");
        let _runtime = RuntimeGuard::new(runtime.clone());
        dioxus_core::with_update_priority(UpdatePriority::SyncInput, || {
            signal += 1;
        });
    });

    let stats = drive(&mut fut);
    drop(fut);

    let priorities: Vec<UpdatePriority> = take_priority_trace()
        .into_iter()
        .map(|(_, p)| p)
        .collect();
    assert_eq!(
        priorities,
        vec![UpdatePriority::Transition, UpdatePriority::SyncInput],
        "the urgent work queued mid-render must complete before the future returns"
    );
    assert_eq!(stats.work_count, 2);
    assert_eq!(stats.commit_count, 2);
}

#[tokio::test]
async fn effects_run_after_final_concurrent_commit() {
    fn effect_app() -> Element {
        let count = use_signal(|| 0);
        EFFECT_SIGNAL.with_borrow_mut(|slot| *slot = Some(count));

        use_effect(move || {
            EFFECT_VALUES.with_borrow_mut(|values| values.push(count()));
        });

        rsx! { div { "{count}" } }
    }

    EFFECT_SIGNAL.with_borrow_mut(|slot| *slot = None);
    EFFECT_VALUES.with_borrow_mut(Vec::clear);

    let mut dom = VirtualDom::new(effect_app);
    dom.rebuild();
    dom.render_immediate();
    EFFECT_VALUES.with_borrow_mut(Vec::clear);

    {
        let _runtime = RuntimeGuard::new(dom.runtime());
        EFFECT_SIGNAL.with_borrow(|slot| {
            let mut count = slot.expect("effect signal should be registered");
            count += 1;
        });
    }

    let (stats, mut mutations) = dom.render_concurrent_with(Mutations::default()).await;

    assert!(stats.commit_count >= 1);
    assert_eq!(EFFECT_VALUES.with_borrow(Clone::clone), vec![1]);
}

#[tokio::test]
async fn effects_wait_for_buffered_commit() {
    /// Tracks whether the renderer has seen any committed mutations. The
    /// effect reads this to verify mutations were flushed before it ran.
    struct CommitObserver(bool);
    impl dioxus_core::WriteMutations for CommitObserver {
        fn append_children(&mut self, _: dioxus_core::ElementId, _: usize) {
            self.0 = true;
            EFFECT_COMMIT_SEEN.set(true);
        }
        fn assign_node_id(&mut self, _: &'static [u8], _: dioxus_core::ElementId) {
            self.0 = true;
            EFFECT_COMMIT_SEEN.set(true);
        }
        fn create_text_node(&mut self, _: &str, _: dioxus_core::ElementId) {
            self.0 = true;
            EFFECT_COMMIT_SEEN.set(true);
        }
        fn load_template(
            &mut self,
            _: dioxus_core::Template,
            _: usize,
            _: dioxus_core::ElementId,
        ) {
            self.0 = true;
            EFFECT_COMMIT_SEEN.set(true);
        }
        fn replace_node_with(&mut self, _: dioxus_core::ElementId, _: usize) {
            self.0 = true;
            EFFECT_COMMIT_SEEN.set(true);
        }
        fn insert_children_at_path(&mut self, _: &'static [u8], _: usize) {
            self.0 = true;
            EFFECT_COMMIT_SEEN.set(true);
        }
        fn insert_nodes_after(&mut self, _: dioxus_core::ElementId, _: usize) {
            self.0 = true;
            EFFECT_COMMIT_SEEN.set(true);
        }
        fn insert_nodes_before(&mut self, _: dioxus_core::ElementId, _: usize) {
            self.0 = true;
            EFFECT_COMMIT_SEEN.set(true);
        }
        fn set_attribute(
            &mut self,
            _: &'static str,
            _: Option<&'static str>,
            _: &dioxus_core::AttributeValue,
            _: dioxus_core::ElementId,
        ) {
            self.0 = true;
            EFFECT_COMMIT_SEEN.set(true);
        }
        fn set_node_text(&mut self, _: &str, _: dioxus_core::ElementId) {
            self.0 = true;
            EFFECT_COMMIT_SEEN.set(true);
        }
        fn create_event_listener(&mut self, _: &'static str, _: dioxus_core::ElementId) {
            self.0 = true;
            EFFECT_COMMIT_SEEN.set(true);
        }
        fn remove_event_listener(&mut self, _: &'static str, _: dioxus_core::ElementId) {
            self.0 = true;
            EFFECT_COMMIT_SEEN.set(true);
        }
        fn remove_node(&mut self, _: dioxus_core::ElementId) {
            self.0 = true;
            EFFECT_COMMIT_SEEN.set(true);
        }
        fn push_root(&mut self, _: dioxus_core::ElementId) {
            self.0 = true;
            EFFECT_COMMIT_SEEN.set(true);
        }
        fn pop_root(&mut self) {
            self.0 = true;
            EFFECT_COMMIT_SEEN.set(true);
        }
        fn commit(&mut self) {}
        fn discard(&mut self) {}
    }

    fn effect_app() -> Element {
        let count = use_signal(|| 0);
        EFFECT_SIGNAL.with_borrow_mut(|slot| *slot = Some(count));

        use_effect(move || {
            let committed = EFFECT_COMMIT_SEEN.get();
            let value = if committed { count() } else { -count() };
            EFFECT_VALUES.with_borrow_mut(|values| values.push(value));
        });

        rsx! { div { "{count}" } }
    }

    EFFECT_COMMIT_SEEN.set(false);
    EFFECT_SIGNAL.with_borrow_mut(|slot| *slot = None);
    EFFECT_VALUES.with_borrow_mut(Vec::clear);

    let mut dom = VirtualDom::new(effect_app);
    dom.rebuild();
    dom.render_immediate();
    EFFECT_VALUES.with_borrow_mut(Vec::clear);
    EFFECT_COMMIT_SEEN.set(false);

    {
        let _runtime = RuntimeGuard::new(dom.runtime());
        EFFECT_SIGNAL.with_borrow(|slot| {
            let mut count = slot.expect("effect signal should be registered");
            count += 1;
        });
    }

    let mut observer = CommitObserver(false);
    dom.render_concurrent_into(&mut observer).await;

    assert!(
        observer.0,
        "renderer should have received committed mutations"
    );
    assert_eq!(
        EFFECT_VALUES.with_borrow(Clone::clone),
        vec![1],
        "effect must observe the post-commit value"
    );
}

#[tokio::test]
async fn dropping_render_concurrent_keeps_renderer_in_sync() {
    #[component]
    fn Row(id: usize, round: u32) -> Element {
        rsx! { div { "{id}:{round}" } }
    }

    fn list_app() -> Element {
        let round = use_signal(|| 0);
        PARENT_ROUND_SIGNAL.with_borrow_mut(|slot| *slot = Some(round));
        rsx! {
            for id in 0..16 {
                Row { key: "{id}", id, round: round() }
            }
        }
    }

    PARENT_ROUND_SIGNAL.with_borrow_mut(|slot| *slot = None);

    let mut dom = VirtualDom::new(list_app);
    dom.rebuild();

    PARENT_ROUND_SIGNAL.with_borrow(|slot| {
        let mut round = slot.expect("parent signal should be registered");
        let _runtime = RuntimeGuard::new(dom.runtime());
        dioxus_core::with_update_priority(UpdatePriority::Transition, || {
            round += 1;
        });
    });

    // Step the future a handful of times, then drop it before it finishes.
    let mut partial = Mutations::default();
    {
        let mut fut = pin!(dom.render_concurrent_into(&mut partial));
        for _ in 0..3 {
            assert!(matches!(step(&mut fut), Poll::Pending));
        }
        // fut drops here — render_concurrent stops cleanly.
    }

    let partial_edits = partial.edits.len();
    assert!(
        partial_edits > 0,
        "the partial run should have committed some work before being cancelled"
    );

    // Resume rendering with a fresh writer. The follow-up call must complete
    // the remaining dirty work, and the union of both writers' mutations
    // should match what a single uninterrupted render would have produced.
    let mut rest = Mutations::default();
    dom.render_concurrent_into(&mut rest).await;

    let mut reference_dom = VirtualDom::new(list_app);
    reference_dom.rebuild();
    PARENT_ROUND_SIGNAL.with_borrow(|slot| {
        let mut round = slot.expect("parent signal should be registered");
        let _runtime = RuntimeGuard::new(reference_dom.runtime());
        dioxus_core::with_update_priority(UpdatePriority::Transition, || {
            round += 1;
        });
    });
    let mut reference = Mutations::default();
    reference_dom.render_concurrent_into(&mut reference).await;

    let resumed_edits = partial.edits.len() + rest.edits.len();
    assert_eq!(
        resumed_edits,
        reference.edits.len(),
        "cancel + resume must produce the same number of mutations as one render"
    );
}
