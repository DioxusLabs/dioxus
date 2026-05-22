use dioxus::prelude::*;
use dioxus_core::{
    Mutation, Mutations, RenderSchedulerDecision, RuntimeGuard, ScopeId, UpdatePriority,
    VirtualDom,
};
use std::cell::{Cell, RefCell};
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
}

#[tokio::test]
async fn concurrent_render_writes_to_mutation_queue() {
    let mut dom = VirtualDom::new(app);
    dom.rebuild(&mut Mutations::default());

    dom.mark_dirty_with_priority(ScopeId::APP, UpdatePriority::Transition);
    let mut mutations = Mutations::default();
    let stats = dom.render_concurrent(&mut mutations).await;

    assert_eq!(stats.generation, 1);
    assert_eq!(stats.priority, UpdatePriority::Transition);
    assert_eq!(stats.commit_count, 1);
    assert!(!mutations.edits.is_empty());
}

#[tokio::test]
async fn concurrent_render_reports_commit_stats() {
    let mut dom = VirtualDom::new(app);
    dom.rebuild(&mut Mutations::default());

    dom.mark_dirty_with_priority(ScopeId::APP, UpdatePriority::SyncInput);

    let mut mutations = Mutations::default();
    let mut committed_priorities = Vec::new();
    let stats = dom
        .render_concurrent_with_scheduler(
            &mut mutations,
            |_, _| RenderSchedulerDecision::Commit,
            |_, render_commit| {
                committed_priorities.push(render_commit.priority);
            },
            |_| std::future::ready(()),
        )
        .await;

    assert_eq!(stats.priority, UpdatePriority::SyncInput);
    assert_eq!(stats.work_count, 1);
    assert_eq!(stats.commit_count, 1);
    assert_eq!(stats.yield_count, 0);
    assert_eq!(committed_priorities, vec![UpdatePriority::SyncInput]);
    assert!(!mutations.edits.is_empty());
}

#[tokio::test]
async fn concurrent_render_scheduler_commits_and_yields_each_checkpoint() {
    let mut dom = VirtualDom::new(app);
    dom.rebuild(&mut Mutations::default());

    dom.mark_dirty_with_priority(ScopeId::APP, UpdatePriority::Default);
    let mut mutations = Mutations::default();
    let mut commit_count = 0;
    let stats = dom
        .render_concurrent_with_scheduler(
            &mut mutations,
            |_, _| RenderSchedulerDecision::CommitAndYield,
            |_, _| {
                commit_count += 1;
            },
            |_| std::future::ready(()),
        )
        .await;

    assert_eq!(stats.priority, UpdatePriority::Default);
    assert_eq!(stats.work_count, 1);
    assert_eq!(stats.commit_count, commit_count);
    assert_eq!(stats.yield_count, 1);
    assert_eq!(commit_count, 1);
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
    dom.rebuild(&mut Mutations::default());
    IMMEDIATE_CHILD_RENDERS.with_borrow_mut(Vec::clear);

    {
        let _runtime = RuntimeGuard::new(dom.runtime());
        IMMEDIATE_PARENT_SIGNAL.with_borrow(|slot| {
            let mut round = slot.expect("parent signal should be registered");
            round += 1;
        });
    }

    let mut mutations = Mutations::default();
    dom.render_immediate(&mut mutations);

    assert_eq!(IMMEDIATE_CHILD_RENDERS.with_borrow(Clone::clone), vec![1]);
    assert!(
        mutations
            .edits
            .iter()
            .any(|mutation| matches!(mutation, Mutation::SetText { value, .. } if value == "1"))
    );
}

#[tokio::test]
async fn scheduler_can_yield_without_committing() {
    let mut dom = VirtualDom::new(app);
    dom.rebuild(&mut Mutations::default());

    dom.mark_dirty_with_priority(ScopeId::APP, UpdatePriority::Default);

    let mut mutations = Mutations::default();
    let mut yielded = false;
    let mut commit_count = 0;
    let stats = dom
        .render_concurrent_with_scheduler(
            &mut mutations,
            |_, _| {
                if yielded {
                    RenderSchedulerDecision::Continue
                } else {
                    yielded = true;
                    RenderSchedulerDecision::Yield
                }
            },
            |_, _| {
                commit_count += 1;
            },
            |_| async {},
        )
        .await;

    assert_eq!(stats.work_count, 1);
    assert_eq!(stats.yield_count, 1);
    assert_eq!(stats.commit_count, 1);
    assert_eq!(commit_count, 1);
    assert!(!mutations.edits.is_empty());
}

#[tokio::test]
async fn scheduler_can_commit_without_yielding() {
    let mut dom = VirtualDom::new(app);
    dom.rebuild(&mut Mutations::default());

    dom.mark_dirty_with_priority(ScopeId::APP, UpdatePriority::Default);

    let mut mutations = Mutations::default();
    let mut commit_count = 0;
    let stats = dom
        .render_concurrent_with_scheduler(
            &mut mutations,
            |_, _| RenderSchedulerDecision::Commit,
            |_, _| {
                commit_count += 1;
            },
            |_| async {},
        )
        .await;

    assert_eq!(stats.work_count, 1);
    assert_eq!(stats.yield_count, 0);
    assert_eq!(stats.commit_count, 1);
    assert_eq!(commit_count, 1);
    assert!(!mutations.edits.is_empty());
}

#[tokio::test]
async fn scheduler_can_commit_and_yield() {
    let mut dom = VirtualDom::new(app);
    dom.rebuild(&mut Mutations::default());

    dom.mark_dirty_with_priority(ScopeId::APP, UpdatePriority::Default);

    let mut mutations = Mutations::default();
    let mut commit_count = 0;
    let stats = dom
        .render_concurrent_with_scheduler(
            &mut mutations,
            |_, _| RenderSchedulerDecision::CommitAndYield,
            |_, _| {
                commit_count += 1;
            },
            |_| async {},
        )
        .await;

    assert_eq!(stats.work_count, 1);
    assert_eq!(stats.yield_count, 1);
    assert_eq!(stats.commit_count, 1);
    assert_eq!(commit_count, 1);
    assert!(!mutations.edits.is_empty());
}

#[tokio::test]
async fn scheduler_reports_buffered_work_at_checkpoint() {
    let mut dom = VirtualDom::new(app);
    dom.rebuild(&mut Mutations::default());
    dom.mark_dirty_with_priority(ScopeId::APP, UpdatePriority::Transition);

    let mut mutations = Mutations::default();
    let mut checkpoints = Vec::new();
    let mut commits = Vec::new();
    let stats = dom
        .render_concurrent_with_scheduler(
            &mut mutations,
            |checkpoint, _| {
                checkpoints.push(checkpoint);
                RenderSchedulerDecision::Continue
            },
            |_, commit| commits.push(commit),
            |_| async {},
        )
        .await;

    let checkpoint = checkpoints.first().expect("expected one work unit");
    assert_eq!(checkpoint.scope, Some(ScopeId::APP));
    assert_eq!(checkpoint.priority, UpdatePriority::Transition);
    assert!(checkpoint.pending_mutations > 0);
    assert_eq!(commits.len(), 1);
    assert_eq!(commits[0].priority, UpdatePriority::Transition);
    assert_eq!(commits[0].mutation_count, checkpoint.pending_mutations);
    assert!(!mutations.edits.is_empty());
    assert_eq!(stats.work_count, 1);
    assert_eq!(stats.commit_count, 1);
}

#[tokio::test]
async fn scheduler_commit_reports_work_since_previous_commit() {
    let mut dom = VirtualDom::new(app);
    dom.rebuild(&mut Mutations::default());

    dom.mark_dirty_with_priority(ScopeId::APP, UpdatePriority::Default);

    let mut mutations = Mutations::default();
    let mut commits = Vec::new();
    dom.render_concurrent_with_scheduler(
        &mut mutations,
        |checkpoint, _| {
            assert_eq!(checkpoint.work_units_since_yield, 1);
            RenderSchedulerDecision::Commit
        },
        |_, commit| commits.push(commit),
        |_| async {},
    )
    .await;
    assert_eq!(commits[0].work_count, 1);

    dom.mark_dirty_with_priority(ScopeId::APP, UpdatePriority::Default);

    commits.clear();
    dom.render_concurrent_with_scheduler(
        &mut mutations,
        |checkpoint, _| {
            assert_eq!(checkpoint.work_units_since_yield, 1);
            RenderSchedulerDecision::Commit
        },
        |_, commit| commits.push(commit),
        |_| async {},
    )
    .await;
    assert_eq!(commits[0].work_count, 1);
}

#[tokio::test]
async fn scheduler_commits_before_new_urgent_work() {
    fn child_a() -> Element {
        let count = use_signal(|| 0);
        CHILD_A_SIGNAL.with_borrow_mut(|slot| *slot = Some(count));
        let generation = dioxus_core::generation();
        rsx! { div { "a {generation} {count}" } }
    }

    fn child_b() -> Element {
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
    dom.rebuild(&mut Mutations::default());
    let runtime = dom.runtime();

    dom.mark_dirty_with_priority(ScopeId(4), UpdatePriority::Transition);
    dom.mark_dirty_with_priority(ScopeId(5), UpdatePriority::Transition);

    let mut mutations = Mutations::default();
    let mut checkpoints = Vec::new();
    let mut commits = Vec::new();
    let mut queued_urgent = false;
    dom.render_concurrent_with_scheduler(
        &mut mutations,
        |checkpoint, _| {
            checkpoints.push(checkpoint);
            if !queued_urgent {
                queued_urgent = true;
                CHILD_A_SIGNAL.with_borrow(|slot| {
                    let mut signal = slot.expect("child signal should be registered");
                    let _runtime = RuntimeGuard::new(runtime.clone());
                    dioxus_core::with_update_priority(UpdatePriority::SyncInput, || {
                        signal += 1;
                    });
                });
            }
            RenderSchedulerDecision::Continue
        },
        |_, commit| commits.push(commit),
        |_| async {},
    )
    .await;

    assert_eq!(checkpoints[0].priority, UpdatePriority::Transition);
    assert_eq!(commits[0].priority, UpdatePriority::Transition);
    assert!(commits[0].mutation_count > 0);
    assert_eq!(checkpoints[1].priority, UpdatePriority::SyncInput);
}

#[tokio::test]
async fn higher_priority_scope_render_preserves_lower_priority_lane() {
    let mut dom = VirtualDom::new(app);
    dom.rebuild(&mut Mutations::default());

    dom.mark_dirty_with_priority(ScopeId::APP, UpdatePriority::Transition);
    dom.mark_dirty_with_priority(ScopeId::APP, UpdatePriority::ContinuousInput);

    let mut mutations = Mutations::default();
    let mut checkpoints = Vec::new();
    dom.render_concurrent_with_scheduler(
        &mut mutations,
        |checkpoint, _| {
            checkpoints.push(checkpoint);
            RenderSchedulerDecision::Commit
        },
        |_, _| {},
        |_| async {},
    )
    .await;

    assert_eq!(checkpoints[0].scope, Some(ScopeId::APP));
    assert_eq!(checkpoints[0].priority, UpdatePriority::ContinuousInput);
    assert_eq!(checkpoints[1].scope, Some(ScopeId::APP));
    assert_eq!(checkpoints[1].priority, UpdatePriority::Transition);
}

#[tokio::test]
async fn dirty_parent_runs_before_more_urgent_child() {
    #[allow(non_snake_case)]
    fn Child() -> Element {
        rsx! { div { "child" } }
    }

    fn parent_app() -> Element {
        rsx! { Child {} }
    }

    let mut dom = VirtualDom::new(parent_app);
    dom.rebuild(&mut Mutations::default());

    dom.mark_dirty_with_priority(ScopeId::APP, UpdatePriority::Transition);
    dom.mark_dirty_with_priority(ScopeId(4), UpdatePriority::SyncInput);

    let mut mutations = Mutations::default();
    let mut checkpoints = Vec::new();
    dom.render_concurrent_with_scheduler(
        &mut mutations,
        |checkpoint, _| {
            checkpoints.push(checkpoint);
            RenderSchedulerDecision::Commit
        },
        |_, _| {},
        |_| async {},
    )
    .await;

    assert_eq!(checkpoints[0].scope, Some(ScopeId::APP));
    assert_eq!(checkpoints[0].priority, UpdatePriority::Transition);
    assert_eq!(checkpoints[1].scope, Some(ScopeId(4)));
    assert_eq!(checkpoints[1].priority, UpdatePriority::SyncInput);
}

#[tokio::test]
async fn memoized_dirty_child_is_not_promoted_by_parent_lane() {
    #[component]
    fn PropChild(id: usize, round: u32) -> Element {
        rsx! { div { "{id}:{round}" } }
    }

    fn parent_app() -> Element {
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
    dom.rebuild(&mut Mutations::default());
    let runtime = dom.runtime();

    PARENT_ROUND_SIGNAL.with_borrow(|slot| {
        let mut round = slot.expect("parent round signal should be registered");
        let _runtime = RuntimeGuard::new(runtime.clone());
        dioxus_core::with_update_priority(UpdatePriority::Transition, || {
            round += 1;
        });
    });

    let mut mutations = Mutations::default();
    let mut checkpoints = Vec::new();
    let mut queued_first_tick = false;
    let mut queued_second_tick = false;
    dom.render_concurrent_with_scheduler(
        &mut mutations,
        |checkpoint, _| {
            checkpoints.push(checkpoint);
            if !queued_first_tick {
                queued_first_tick = true;
                PARENT_TICK_SIGNAL.with_borrow(|slot| {
                    let mut tick = slot.expect("parent tick signal should be registered");
                    let _runtime = RuntimeGuard::new(runtime.clone());
                    dioxus_core::with_update_priority(UpdatePriority::ContinuousInput, || {
                        tick += 1;
                    });
                });
            }
            RenderSchedulerDecision::Continue
        },
        |_, commit| {
            if commit.priority == UpdatePriority::ContinuousInput && !queued_second_tick {
                queued_second_tick = true;
                PARENT_TICK_SIGNAL.with_borrow(|slot| {
                    let mut tick = slot.expect("parent tick signal should be registered");
                    let _runtime = RuntimeGuard::new(runtime.clone());
                    dioxus_core::with_update_priority(UpdatePriority::ContinuousInput, || {
                        tick += 1;
                    });
                });
            }
        },
        |_| async {},
    )
    .await;

    assert_eq!(checkpoints[0].scope, Some(ScopeId::APP));
    assert_eq!(checkpoints[0].priority, UpdatePriority::Transition);
    assert!(
        checkpoints
            .iter()
            .skip_while(|checkpoint| checkpoint.priority != UpdatePriority::ContinuousInput)
            .any(
                |checkpoint| checkpoint.priority == UpdatePriority::Transition
                    && checkpoint.scope != Some(ScopeId::APP)
            )
    );
}

#[tokio::test]
async fn deferred_child_props_survive_parent_continuous_commit() {
    #[component]
    fn PropChild(round: u32) -> Element {
        rsx! { div { "{round}" } }
    }

    fn parent_app() -> Element {
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
    dom.rebuild(&mut Mutations::default());
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

    let mut mutations = Mutations::default();
    let mut checkpoints = Vec::new();
    dom.render_concurrent_with_scheduler(
        &mut mutations,
        |checkpoint, _| {
            checkpoints.push(checkpoint);
            RenderSchedulerDecision::Commit
        },
        |_, _| {},
        |_| async {},
    )
    .await;

    assert_eq!(checkpoints[0].scope, Some(ScopeId::APP));
    assert_eq!(checkpoints[0].priority, UpdatePriority::ContinuousInput);
    assert_eq!(checkpoints[1].scope, Some(ScopeId::APP));
    assert_eq!(checkpoints[1].priority, UpdatePriority::Transition);
    assert!(checkpoints.iter().any(|checkpoint| {
        checkpoint.scope != Some(ScopeId::APP) && checkpoint.priority == UpdatePriority::Transition
    }));
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
    dom.rebuild(&mut Mutations::default());
    let runtime = dom.runtime();
    let expected_leaf_updates = 3_usize.pow(DEPTH as u32);

    set_round(&runtime, UpdatePriority::Transition);

    let mut saw_metric = false;
    let mut leaf_updates = 0;
    let mut trace = Vec::new();
    let mut ticks = 0;

    let mut mutations = Mutations::default();
    dom.render_concurrent_with_scheduler(
        &mut mutations,
        |checkpoint, _| {
            if ticks < 800 {
                ticks += 1;
                tick(&runtime);
            }
            if trace.len() < 160 {
                trace.push(format!(
                    "{:?}:{:?}:pending={}:work={}",
                    checkpoint.priority,
                    checkpoint.scope,
                    checkpoint.pending_mutations,
                    checkpoint.work_units_since_yield
                ));
            }
            if checkpoint.priority <= UpdatePriority::ContinuousInput
                && checkpoint.pending_mutations > 0
            {
                RenderSchedulerDecision::Commit
            } else if checkpoint.work_units_since_yield >= 5 {
                RenderSchedulerDecision::CommitAndYield
            } else {
                RenderSchedulerDecision::Continue
            }
        },
        |mutations, _| {
            saw_metric |= mutations.edits.iter().any(
                |mutation| matches!(mutation, Mutation::SetText { value, .. } if value == "metric:1"),
            );
            leaf_updates += mutations
                .edits
                .iter()
                .filter(
                    |mutation| {
                        matches!(mutation, Mutation::SetText { value, .. } if value == "leaf:1")
                    },
                )
                .count();
            mutations.edits.clear();
        },
        |_| async {},
    )
    .await;

    assert!(saw_metric, "root transition text should commit");
    assert!(
        leaf_updates >= expected_leaf_updates,
        "all recursive transition leaf text should commit; saw {leaf_updates}/{expected_leaf_updates}; trace:\n{}",
        trace.join("\n")
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
    dom.rebuild(&mut Mutations::default());
    let runtime = dom.runtime();

    set_seconds(&runtime);

    let mut transition_work = 0;
    let mut saw_leaf_update = false;
    let mut trace = Vec::new();
    let mut ticks = 0;

    let mut mutations = Mutations::default();
    dom.render_concurrent_with_scheduler(
        &mut mutations,
        |checkpoint, _| {
            if ticks < 256 {
                ticks += 1;
                tick(&runtime);
            }
            if checkpoint.priority == UpdatePriority::Transition {
                transition_work += 1;
            }
            if trace.len() < 160 {
                trace.push(format!(
                    "{:?}:{:?}:pending={}:work={}",
                    checkpoint.priority,
                    checkpoint.scope,
                    checkpoint.pending_mutations,
                    checkpoint.work_units_since_yield
                ));
            }
            if checkpoint.pending_mutations > 0 || checkpoint.work_units_since_yield >= 5 {
                RenderSchedulerDecision::CommitAndYield
            } else {
                RenderSchedulerDecision::Continue
            }
        },
        |mutations, _| {
            saw_leaf_update |= mutations.edits.iter().any(
                |mutation| matches!(mutation, Mutation::SetText { value, .. } if value == "dot:1"),
            );
            mutations.edits.clear();
        },
        |_| async {},
    )
    .await;

    assert!(
        saw_leaf_update,
        "demo-sized transition should reach visible leaf text before the whole breadth frontier; transition work={transition_work}; trace:\n{}",
        trace.join("\n")
    );
}

#[tokio::test]
async fn host_yield_lets_continuous_input_preempt_active_transition_lane() {
    #[component]
    fn Leaf(seconds: u32) -> Element {
        rsx! { span { "{seconds}" } }
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
    dom.rebuild(&mut Mutations::default());
    let runtime = dom.runtime();

    set_round(&runtime);

    let mut saw_transition = false;
    let saw_continuous_after_transition_yield = Cell::new(false);
    let yielded_transition = Cell::new(false);
    let ticks = Cell::new(0);
    let mut mutations = Mutations::default();
    dom.render_concurrent_with_scheduler(
        &mut mutations,
        |checkpoint, _| {
            if ticks.get() < 32 {
                ticks.set(ticks.get() + 1);
                tick(&runtime);
            }
            if yielded_transition.get() && !saw_continuous_after_transition_yield.get() {
                saw_continuous_after_transition_yield
                    .set(checkpoint.priority == UpdatePriority::ContinuousInput);
            }
            if checkpoint.priority == UpdatePriority::Transition && !saw_transition {
                saw_transition = true;
                RenderSchedulerDecision::CommitAndYield
            } else {
                RenderSchedulerDecision::Commit
            }
        },
        |_, _| {},
        |priority| {
            if priority == Some(UpdatePriority::Transition) {
                yielded_transition.set(true);
                if ticks.get() < 32 {
                    ticks.set(ticks.get() + 1);
                    tick(&runtime);
                }
            }
            async {}
        },
    )
    .await;

    assert!(
        saw_transition,
        "fairness should eventually run transition work"
    );
    assert!(
        saw_continuous_after_transition_yield.get(),
        "fresh continuous input should preempt a transition lane after a host yield"
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
    dom.rebuild(&mut Mutations::default());

    PARENT_ROUND_SIGNAL.with_borrow(|slot| {
        let mut round = slot.expect("parent signal should be registered");
        let _runtime = RuntimeGuard::new(dom.runtime());
        dioxus_core::with_update_priority(UpdatePriority::Transition, || {
            round += 1;
        });
    });

    let mut mutations = Mutations::default();
    let stats = dom.render_concurrent(&mut mutations).await;

    assert_eq!(stats.priority, UpdatePriority::Transition);
    assert_eq!(stats.work_count, 44);
    assert_eq!(stats.commit_count, 44);
    assert_eq!(stats.yield_count, 44);
    assert!(!mutations.edits.is_empty());
}

#[tokio::test]
async fn concurrent_render_without_work_does_not_commit() {
    let mut dom = VirtualDom::new(app);
    dom.rebuild(&mut Mutations::default());

    let mut mutations = Mutations::default();
    let stats = dom.render_concurrent(&mut mutations).await;
    assert_eq!(stats.generation, 0);
    assert_eq!(stats.priority, UpdatePriority::Idle);
    assert_eq!(stats.commit_count, 0);
    assert!(mutations.edits.is_empty());

    dom.mark_dirty_with_priority(ScopeId::APP, UpdatePriority::Default);
    let stats = dom.render_concurrent(&mut mutations).await;
    assert_eq!(stats.generation, 1);
    assert_eq!(stats.commit_count, 1);
    assert!(!mutations.edits.is_empty());

    mutations.edits.clear();
    let stats = dom.render_concurrent(&mut mutations).await;
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
    let mut mutations = Mutations::default();
    dom.rebuild(&mut mutations);
    let button = mutations
        .edits
        .iter()
        .find_map(|mutation| match mutation {
            Mutation::NewEventListener { id, .. } => Some(*id),
            _ => None,
        })
        .expect("button should have an event listener");

    dom.runtime().handle_event("click", click_event(), button);

    let mut mutations = Mutations::default();
    let stats = dom.render_concurrent(&mut mutations).await;
    assert_eq!(stats.priority, UpdatePriority::SyncInput);
    assert!(!mutations.edits.is_empty());
}

#[tokio::test]
async fn urgent_work_preempts_resumed_transition_diff() {
    fn child_a() -> Element {
        let count = use_signal(|| 0);
        CHILD_A_SIGNAL.with_borrow_mut(|slot| *slot = Some(count));
        let generation = dioxus_core::generation();
        rsx! { div { "a {generation} {count}" } }
    }

    fn child_b() -> Element {
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
    dom.rebuild(&mut Mutations::default());

    dom.mark_dirty_with_priority(ScopeId(4), UpdatePriority::Transition);
    dom.mark_dirty_with_priority(ScopeId(5), UpdatePriority::Transition);

    let mut applied_priorities = Vec::new();
    let mut queued_urgent_work = false;
    let mut mutations = Mutations::default();
    let stats = dom
        .render_concurrent_with_scheduler(
            &mut mutations,
            |_, _| RenderSchedulerDecision::CommitAndYield,
            |_, render_commit| {
                applied_priorities.push(render_commit.priority);
                if !queued_urgent_work {
                    queued_urgent_work = true;
                    CHILD_A_SIGNAL.with_borrow(|slot| {
                        let mut signal = slot.expect("child signal should be registered");
                        dioxus_core::with_update_priority(UpdatePriority::SyncInput, || {
                            signal += 1;
                        });
                    });
                }
            },
            |_| std::future::ready(()),
        )
        .await;

    assert_eq!(
        applied_priorities,
        vec![
            UpdatePriority::Transition,
            UpdatePriority::SyncInput,
            UpdatePriority::Transition,
        ]
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
        let generation = dioxus_core::generation();
        rsx! { div { "a {generation} {count}" } }
    }

    fn child_b() -> Element {
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
    dom.rebuild(&mut Mutations::default());

    dom.mark_dirty_with_priority(ScopeId(5), UpdatePriority::Transition);

    CHILD_A_SIGNAL.with_borrow(|slot| {
        let mut signal = slot.expect("child signal should be registered");
        let _runtime = RuntimeGuard::new(dom.runtime());
        dioxus_core::with_update_priority(UpdatePriority::SyncInput, || {
            signal += 1;
        });
    });

    let mut mutations = Mutations::default();
    let mut applied_priorities = Vec::new();
    let stats = dom
        .render_concurrent_with_scheduler(
            &mut mutations,
            |_, _| RenderSchedulerDecision::Commit,
            |_, render_commit| {
                applied_priorities.push(render_commit.priority);
            },
            |_| std::future::ready(()),
        )
        .await;

    assert_eq!(
        applied_priorities,
        vec![UpdatePriority::SyncInput, UpdatePriority::Transition]
    );
    assert_eq!(stats.work_count, 2);
    assert_eq!(stats.commit_count, 2);
    assert_eq!(stats.yield_count, 0);
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
    dom.rebuild(&mut Mutations::default());

    PARENT_ROUND_SIGNAL.with_borrow(|slot| {
        let mut round = slot.expect("parent signal should be registered");
        let _runtime = RuntimeGuard::new(dom.runtime());
        dioxus_core::with_update_priority(UpdatePriority::Transition, || {
            round += 1;
        });
    });

    let mut mutations = Mutations::default();
    let stats = dom.render_concurrent(&mut mutations).await;

    assert!(stats.work_count > 4);
    assert!(stats.yield_count > 4);
}

#[tokio::test]
async fn work_queued_by_final_commit_is_rendered_before_return() {
    fn child_a() -> Element {
        let count = use_signal(|| 0);
        CHILD_A_SIGNAL.with_borrow_mut(|slot| *slot = Some(count));
        let generation = dioxus_core::generation();
        rsx! { div { "a {generation} {count}" } }
    }

    fn final_commit_app() -> Element {
        rsx! { child_a {} }
    }

    CHILD_A_SIGNAL.with_borrow_mut(|slot| *slot = None);

    let mut dom = VirtualDom::new(final_commit_app);
    dom.rebuild(&mut Mutations::default());

    dom.mark_dirty_with_priority(ScopeId(4), UpdatePriority::Transition);

    let mut applied_priorities = Vec::new();
    let mut queued_urgent_work = false;
    let mut mutations = Mutations::default();
    let stats = dom
        .render_concurrent_with_scheduler(
            &mut mutations,
            |_, _| RenderSchedulerDecision::Commit,
            |_, render_commit| {
                applied_priorities.push(render_commit.priority);
                if !queued_urgent_work {
                    queued_urgent_work = true;
                    CHILD_A_SIGNAL.with_borrow(|slot| {
                        let mut signal = slot.expect("child signal should be registered");
                        dioxus_core::with_update_priority(UpdatePriority::SyncInput, || {
                            signal += 1;
                        });
                    });
                }
            },
            |_| std::future::ready(()),
        )
        .await;

    assert_eq!(
        applied_priorities,
        vec![UpdatePriority::Transition, UpdatePriority::SyncInput]
    );
    assert_eq!(stats.work_count, 2);
    assert_eq!(stats.commit_count, 2);
    assert_eq!(stats.yield_count, 0);
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
    dom.rebuild(&mut Mutations::default());
    dom.render_immediate(&mut Mutations::default());
    EFFECT_VALUES.with_borrow_mut(Vec::clear);

    {
        let _runtime = RuntimeGuard::new(dom.runtime());
        EFFECT_SIGNAL.with_borrow(|slot| {
            let mut count = slot.expect("effect signal should be registered");
            count += 1;
        });
    }

    let mut mutations = Mutations::default();
    let stats = dom.render_concurrent(&mut mutations).await;

    assert!(stats.commit_count >= 1);
    assert_eq!(EFFECT_VALUES.with_borrow(Clone::clone), vec![1]);
}

#[tokio::test]
async fn effects_wait_for_buffered_commit() {
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
    dom.rebuild(&mut Mutations::default());
    dom.render_immediate(&mut Mutations::default());
    EFFECT_VALUES.with_borrow_mut(Vec::clear);
    EFFECT_COMMIT_SEEN.set(false);

    {
        let _runtime = RuntimeGuard::new(dom.runtime());
        EFFECT_SIGNAL.with_borrow(|slot| {
            let mut count = slot.expect("effect signal should be registered");
            count += 1;
        });
    }

    let mut mutations = Mutations::default();
    let mut commits = 0;
    dom.render_concurrent_with_scheduler(
        &mut mutations,
        |_, _| RenderSchedulerDecision::Continue,
        |_, _| {
            commits += 1;
            EFFECT_COMMIT_SEEN.set(true);
        },
        |_| async {},
    )
    .await;

    assert_eq!(commits, 1);
    assert_eq!(EFFECT_VALUES.with_borrow(Clone::clone), vec![1]);
}
