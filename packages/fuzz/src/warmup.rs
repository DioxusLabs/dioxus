//! Hand-built warmup scenarios that exercise core diff paths the
//! per-input sync replay in [`crate::case::run_case`] cannot reach. Run once
//! per fuzz process so the coverage-instrumented binary records them.

/// Drive the VirtualDom's pending work to completion synchronously.
fn drive_render(dom: &mut dioxus_core::VirtualDom) {
    dom.render_immediate(&mut dioxus_core::Mutations::default());
}

thread_local! {
    /// Shared generation counter for the `warmup_*` scenarios below. Apps read
    /// it via [`warmup_gen`] to pick which variant to render;
    /// [`run_generations`] advances it once per render round.
    static WARMUP_GEN: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
}

/// The current warmup generation: 0 during the initial rebuild, then 1, 2, …
/// for each subsequent render round driven by [`run_generations`].
fn warmup_gen() -> u32 {
    WARMUP_GEN.with(|c| c.get())
}

/// Run a warmup app through `generations` render rounds: reset [`WARMUP_GEN`]
/// to 0, rebuild against a fresh [`RendererOracle`], then for each generation
/// `g` in `1..generations` set `WARMUP_GEN = g`, mark the root scope dirty,
/// and render. Returns the dom and oracle so callers can drive extra custom
/// rounds.
fn run_generations(
    app: fn() -> dioxus_core::Element,
    generations: u32,
) -> (
    dioxus_core::VirtualDom,
    dioxus_renderer_oracle::RendererOracle,
) {
    use dioxus_core::{ScopeId, VirtualDom};
    use dioxus_renderer_oracle::RendererOracle;

    WARMUP_GEN.with(|c| c.set(0));
    let mut dom = VirtualDom::new(app);
    let mut oracle = RendererOracle::new();
    oracle.rebuild(&mut dom);
    for g in 1..generations {
        WARMUP_GEN.with(|c| c.set(g));
        dom.mark_dirty(ScopeId::APP);
        oracle.render(&mut dom);
    }
    (dom, oracle)
}

/// Drive a small unkeyed fragment of identical-component children through a
/// re-render so the batched `queue_component_props_diff` fast path in
/// `diff::iterator::diff_child_pairs` fires (every pair is a same-component,
/// same-render-fn match, exceeding `FRAGMENT_WORK_BATCH`). Also exercises
/// the `Take` iterator monomorphization via a keyed shared-prefix re-render.
fn warmup_batched_component_props_diff() {
    use dioxus::prelude::*;

    #[derive(Clone, PartialEq, Props)]
    struct ItemProps {
        value: u32,
    }

    #[allow(non_snake_case)]
    fn Item(props: ItemProps) -> Element {
        rsx! { span { "{props.value}" } }
    }

    // --- Unkeyed: exercises the slice-iter monomorphization of
    // `diff_child_pairs`.
    fn unkeyed_app() -> Element {
        let g = warmup_gen();
        rsx! {
            for i in 0..20u32 {
                Item { value: i + g }
            }
        }
    }
    run_generations(unkeyed_app, 2);

    // --- Keyed with stable prefix: exercises the `Take<slice iter>`
    // monomorphization of `diff_child_pairs` reached via
    // `diff_shared_prefix` in `diff_keyed_children`. Keep the first
    // `FRAGMENT_WORK_BATCH + 1` keys stable so the shared-prefix walk pumps
    // a same-component batched diff through the fast path.
    fn keyed_app() -> Element {
        let g = warmup_gen();
        rsx! {
            for i in 0..20u32 {
                Item { key: "{i}", value: i + g }
            }
        }
    }
    run_generations(keyed_app, 2);
}

/// Drive a keyed shuffle of >FRAGMENT_WORK_BATCH items so
/// `diff_keyed_middle`'s `collect_splice_mounts` walks survivors that exercise
/// the unchecked mount transfer path on the slice picked by the LIS-based
/// splice.
fn warmup_keyed_reorder() {
    use dioxus::prelude::*;

    #[derive(Clone, PartialEq, Props)]
    struct ItemProps {
        value: u32,
    }

    #[allow(non_snake_case)]
    fn Item(props: ItemProps) -> Element {
        rsx! { span { "{props.value}" } }
    }

    fn keyed_shuffle_app() -> Element {
        let round = warmup_gen();
        // Build a permutation of 0..20 that's the identity on round 0 and
        // shuffled on round 1+. The shuffled half forces `diff_keyed_middle`
        // to splice survivors, walking through `collect_splice_mounts`.
        let order: Vec<u32> = if round == 0 {
            (0..20u32).collect()
        } else {
            (0..20u32).rev().collect()
        };
        rsx! {
            for key in order.iter().copied() {
                Item { key: "{key}", value: key }
            }
        }
    }
    run_generations(keyed_shuffle_app, 2);
}

/// Drive a `SuspenseBoundary` through suspend/resolve transitions so the
/// hidden-subtree state paths fire: vnodes whose mount is unmounted because
/// they live in the suspended branch and were never materialized in the
/// renderer arena.
fn warmup_suspense_hidden_paths() {
    use dioxus::prelude::*;
    use dioxus_core::{SuspenseContext, generation};
    use std::cell::Cell;

    thread_local! {
        static SUSPEND_GEN: Cell<usize> = const { Cell::new(usize::MAX) };
        static SHUFFLE_GEN: Cell<usize> = const { Cell::new(usize::MAX) };
    }

    #[derive(Clone, PartialEq, Props)]
    struct ChildProps {
        value: u32,
    }

    #[component]
    #[allow(non_snake_case)]
    fn SuspendingChild(props: ChildProps) -> Element {
        let g = generation();
        let suspend_at = SUSPEND_GEN.with(|c| c.get());
        if g == suspend_at {
            let task = spawn(async { std::future::pending::<()>().await });
            suspend(task)?;
        }
        rsx! { span { "{props.value}" } }
    }

    // Scenario A: suspend on first render, then re-render so the boundary
    // re-diffs its background children whose mount slots may still be unmounted.
    {
        SUSPEND_GEN.with(|c| c.set(0));
        fn app_a() -> Element {
            rsx! {
                SuspenseBoundary {
                    fallback: |context: SuspenseContext| {
                        let _ = context.with_suspended_mounted_root(|root| root.vnode().template.root_count());
                        rsx! { "loading" }
                    },
                    for i in 0..20u32 {
                        SuspendingChild { key: "{i}", value: i }
                    }
                }
            }
        }
        run_generations(app_a, 4);
    }

    // Scenario B: render normally, then suspend, then re-render with a
    // reversed key order. The keyed-reorder path observes children in the
    // suspended branch with retained mount state, exercising paths that skip
    // children whose DOM never materialized.
    {
        SUSPEND_GEN.with(|c| c.set(1));
        SHUFFLE_GEN.with(|c| c.set(2));
        fn app_b() -> Element {
            let shuffle_at = SHUFFLE_GEN.with(|c| c.get());
            let g = generation();
            let keys: Vec<u32> = if g >= shuffle_at {
                (0..20u32).rev().collect()
            } else {
                (0..20u32).collect()
            };
            rsx! {
                SuspenseBoundary {
                    fallback: |context: SuspenseContext| {
                        let _ = context.with_suspended_mounted_root(|root| root.vnode().template.root_count());
                        rsx! { "loading" }
                    },
                    for key in keys.iter().copied() {
                        SuspendingChild { key: "{key}", value: key }
                    }
                }
            }
        }
        // generation 1: suspend; generation 2: shuffle + still suspending;
        // generation 3: shuffle again.
        run_generations(app_b, 4);
    }
    // Reset for any subsequent warmups.
    SUSPEND_GEN.with(|c| c.set(usize::MAX));
    SHUFFLE_GEN.with(|c| c.set(usize::MAX));
}

fn warmup_empty_fallback_slot_promotion() {
    use dioxus::prelude::*;
    use dioxus_core::generation;

    #[component]
    #[allow(non_snake_case)]
    fn SuspendOnce() -> Element {
        if generation() == 1 {
            let task = spawn(async { std::future::pending::<()>().await });
            suspend(task)?;
        }
        rsx! { span { "ready" } }
    }

    fn app() -> Element {
        rsx! {
            div {
                SuspenseBoundary {
                    fallback: |_| rsx! {},
                    SuspendOnce {}
                }
            }
        }
    }

    run_generations(app, 3);
}

fn warmup_non_root_dynamic_slot_without_adjacent_anchor() {
    use dioxus::prelude::*;
    use dioxus_core::generation;

    fn app() -> Element {
        let g = generation();
        rsx! {
            div {
                if g > 0 {
                    "left"
                }
                span { "middle" }
                if g > 1 {
                    "right"
                }
            }
        }
    }

    run_generations(app, 3);
}

fn warmup_root_dynamic_slot_anchors() {
    use dioxus::prelude::*;
    use dioxus_core::generation;

    fn before_static_root() -> Element {
        let g = generation();
        rsx! {
            if g > 0 {
                "front"
            }
            div { "static" }
        }
    }

    fn before_dynamic_root() -> Element {
        let g = generation();
        rsx! {
            Fragment {
                if g > 0 {
                    "front"
                }
                if g > 1 {
                    "next"
                }
            }
        }
    }

    run_generations(before_static_root, 2);
    run_generations(before_dynamic_root, 3);
}

fn warmup_keyed_fragment_skip_anchors() {
    use dioxus::prelude::*;
    use dioxus_core::generation;

    fn app() -> Element {
        let g = generation();
        let order = if g == 0 {
            [0u32, 1, 2, 3]
        } else {
            [2u32, 0, 3, 1]
        };
        rsx! {
            div {
                for key in order {
                    Fragment {
                        key: "{key}",
                        span { "{key}" }
                    }
                }
            }
        }
    }

    run_generations(app, 3);
}

fn warmup_keyed_fragment_right_edge_anchor() {
    use dioxus::prelude::*;

    fn app() -> Element {
        let order: &[u32] = if warmup_gen() == 0 {
            &[0, 1, 2, 3]
        } else {
            &[3, 0, 1, 2, 4, 5]
        };
        rsx! {
            for key in order.iter().copied() {
                Fragment {
                    key: "{key}",
                    for child in 0..2u32 {
                        span { "{key}:{child}" }
                    }
                }
            }
        }
    }

    run_generations(app, 2);
}

fn warmup_keyed_splice_into_static_slot() {
    use dioxus::prelude::*;

    fn app() -> Element {
        let keys: &[u32] = if warmup_gen() == 0 { &[0] } else { &[0, 1] };
        rsx! {
            div {
                for key in keys.iter().copied() {
                    Fragment {
                        key: "{key}",
                        if key != 0 {
                            span { "{key}" }
                        }
                    }
                }
            }
        }
    }

    run_generations(app, 2);
}

/// Mark a parent scope and all of its descendant scopes dirty at once, then
/// drive a render. Exercises the scheduler diffing an ancestor whose children
/// are also queued, so the descendants are drained as part of the ancestor's
/// pass instead of being re-run afterwards.
fn warmup_deferred_subtree_check() {
    use dioxus::prelude::*;
    use dioxus_core::{ScopeId, VirtualDom, current_scope_id};
    use std::cell::RefCell;

    thread_local! {
        static CHILD_SCOPES: RefCell<Vec<ScopeId>> = const { RefCell::new(Vec::new()) };
    }

    #[derive(Clone, PartialEq, Props)]
    struct ChildProps {
        value: u32,
    }

    #[allow(non_snake_case)]
    fn Child(props: ChildProps) -> Element {
        CHILD_SCOPES.with(|scopes| {
            let id = current_scope_id();
            let mut scopes = scopes.borrow_mut();
            if !scopes.contains(&id) {
                scopes.push(id);
            }
        });
        rsx! { span { "{props.value}" } }
    }

    fn app() -> Element {
        rsx! {
            for i in 0..5u32 {
                Child { value: i }
            }
        }
    }

    let mut dom = VirtualDom::new(app);
    CHILD_SCOPES.with(|scopes| scopes.borrow_mut().clear());
    dom.rebuild_in_place();
    dom.mark_dirty(ScopeId::APP);
    let child_scopes = CHILD_SCOPES.with(|scopes| scopes.borrow().clone());
    for scope in child_scopes {
        dom.mark_dirty(scope);
    }
    drive_render(&mut dom);
}

/// Mix of nested suspense and partial removal: builds a stack of components
/// inside a suspense boundary, then removes/re-orders entries while some
/// remain suspended. The intent is to leave the diff machinery holding a
/// stale `ScopeId` from a dropped sibling so the `get_scope(_)?` and
/// `try_root_node()?` early-returns in
/// `dynamic_node_first_element`/`find_element_at_root_in_target` actually
/// take their `None` branches.
fn warmup_dropped_scope_anchor_lookup() {
    use dioxus::prelude::*;
    use dioxus_core::generation;

    #[derive(Clone, PartialEq, Props)]
    struct InnerProps {
        value: u32,
    }

    #[component]
    #[allow(non_snake_case)]
    fn Suspender(props: InnerProps) -> Element {
        if warmup_gen() == 1 {
            let task = spawn(async { std::future::pending::<()>().await });
            suspend(task)?;
        }
        rsx! { span { "{props.value}" } }
    }

    fn app() -> Element {
        let g = generation();
        let n = match g {
            0 => 10u32,
            1 => 10,
            2 => 4,
            3 => 0,
            _ => 0,
        };
        if n == 0 {
            return rsx! { "empty" };
        }
        rsx! {
            SuspenseBoundary {
                fallback: |_| rsx! { "loading" },
                for i in 0..n {
                    Suspender { key: "{i}", value: i }
                }
            }
        }
    }

    // gen 1: suspend; gen 2: shrink the suspended fragment from 10 to 4
    // (drops 6 suspended child scopes); gen 3: remove the boundary entirely.
    run_generations(app, 4);
}

/// Suspense + removal: render a suspense boundary, suspend its child, then
/// fully remove the boundary so the hidden subtree's vnodes get removed via
/// `remove_node_inner` after being parked in the background. Exercises the
/// mounted-id lookup paths in `dynamic_node_first_element` /
/// `find_element_at_root_in_target` when scopes get dropped mid-diff.
fn warmup_suspense_then_remove() {
    use dioxus::prelude::*;

    #[derive(Clone, PartialEq, Props)]
    struct ChildProps {
        value: u32,
    }

    #[component]
    #[allow(non_snake_case)]
    fn SuspendForever(props: ChildProps) -> Element {
        let task = spawn(async { std::future::pending::<()>().await });
        suspend(task)?;
        rsx! { span { "{props.value}" } }
    }

    fn app() -> Element {
        if warmup_gen() >= 2 {
            // After the remove gen, render nothing — the boundary and its
            // suspended subtree get fully removed.
            return rsx! { "removed" };
        }
        rsx! {
            SuspenseBoundary {
                fallback: |_| rsx! { "loading" },
                for i in 0..10u32 {
                    SuspendForever { key: "{i}", value: i }
                }
            }
        }
    }

    // generation 1: re-render, boundary stays suspended; generation 2:
    // replace the boundary with plain text — removes the suspended subtree,
    // exercising remove_node_inner on unmounted hidden children.
    run_generations(app, 3);
}

/// One-shot warmup that exercises the multi-priority deferred-priority paths in
/// `dioxus_core::diff::component::diff_vcomponent`. The sync `render_immediate`
/// path used by [`run_case`] only ever processes a single priority level at a
/// time, so the `render_deferred_priority`/`deferred_priority_for_subtree`
/// branches are unreachable from corpus inputs alone. Calling this once per
/// fuzz process records coverage of those branches in the fuzz binary.
/// Drive a `Portal` through a target-switch transition so the retarget
/// branch of the portal driver's diff and the surrounding
/// `remove_node_inner` + `create_children_with_parents` machinery fire. The
/// fuzz harness's per-input Portal always uses a single target allocated via
/// `use_hook`, so this branch is otherwise unreachable.
fn warmup_portal_target_switch() {
    use dioxus::prelude::*;
    use dioxus_core::{MultiTargetWriter, Portal, RenderTargetId, ScopeId, VirtualDom};
    use dioxus_renderer_oracle::RendererOracle;
    use std::cell::Cell;

    thread_local! {
        static MODE: Cell<u32> = const { Cell::new(0) };
        static FIRST_TARGET: Cell<RenderTargetId> = const { Cell::new(RenderTargetId::ROOT) };
        static SECOND_TARGET: Cell<RenderTargetId> = const { Cell::new(RenderTargetId::ROOT) };
    }

    fn app() -> Element {
        let mode = MODE.with(|c| c.get());
        let target = match mode {
            0 | 2 => FIRST_TARGET.with(|c| c.get()),
            _ => SECOND_TARGET.with(|c| c.get()),
        };
        rsx! {
            Portal {
                target,
                span { "portal body" }
            }
        }
    }

    let mut dom = VirtualDom::new(app);
    let first = dom.runtime().create_render_target();
    let second = dom.runtime().create_render_target();
    FIRST_TARGET.with(|c| c.set(first));
    SECOND_TARGET.with(|c| c.set(second));
    let mut writer = MultiTargetWriter::<RendererOracle>::new();
    writer.insert(RenderTargetId::ROOT, RendererOracle::new());
    writer.insert(first, RendererOracle::new());
    writer.insert(second, RendererOracle::new());
    dom.rebuild(&mut writer);

    // mode 1: switch from first -> second target, with oracles attached.
    MODE.with(|c| c.set(1));
    dom.mark_dirty(ScopeId::APP);
    dom.render_immediate(&mut writer);

    // mode 2: switch back to first target with NO oracle attached for it.
    // The target router drops mutations for the missing target while the
    // retarget arm still runs its removal and mount logic.
    let _ = writer.take(first);
    let _ = writer.take(second);
    MODE.with(|c| c.set(2));
    dom.mark_dirty(ScopeId::APP);
    dom.render_immediate(&mut writer);

    // mode 3: same props as mode 2 — memoize sees self == new and the
    // `equal` branch of `PortalProps::memoize` fires.
    MODE.with(|c| c.set(2));
    dom.mark_dirty(ScopeId::APP);
    dom.render_immediate(&mut writer);

    // Separate dom: switch to a target that never gets a writer so the
    // missing-writer routing branch fires during a target switch.
    drop(dom);
    let mut dom = VirtualDom::new(app);
    let first = dom.runtime().create_render_target();
    let noop = dom.runtime().create_render_target();
    FIRST_TARGET.with(|c| c.set(first));
    SECOND_TARGET.with(|c| c.set(noop));
    MODE.with(|c| c.set(0));
    let mut writer = MultiTargetWriter::<RendererOracle>::new();
    writer.insert(RenderTargetId::ROOT, RendererOracle::new());
    writer.insert(first, RendererOracle::new());
    dom.rebuild(&mut writer);
    MODE.with(|c| c.set(1));
    dom.mark_dirty(ScopeId::APP);
    dom.render_immediate(&mut writer);
}

/// Mount a scope with a pending effect, then drop it. Exercises the
/// `drop_scope` filter closure that drains `pending_effects` entries for
/// the dropped subtree — unreachable from the fuzz harness because the
/// model never uses `use_effect`.
fn warmup_scope_with_pending_effect() {
    use dioxus::prelude::*;
    use dioxus_core::{ScopeId, current_scope_id, queue_effect};
    use std::cell::Cell;

    thread_local! {
        static CHILD_SCOPE: Cell<Option<ScopeId>> = const { Cell::new(None) };
        static GRANDCHILD_SCOPE: Cell<Option<ScopeId>> = const { Cell::new(None) };
    }

    #[component]
    #[allow(non_snake_case)]
    fn Grandchild() -> Element {
        use_hook(|| {
            GRANDCHILD_SCOPE.with(|c| c.set(Some(current_scope_id())));
        });
        rsx! { em { "grandchild" } }
    }

    #[component]
    #[allow(non_snake_case)]
    fn EffectChild() -> Element {
        use_hook(|| {
            CHILD_SCOPE.with(|c| c.set(Some(current_scope_id())));
        });
        rsx! { span { Grandchild {} } }
    }

    fn app() -> Element {
        if warmup_gen() == 0 {
            rsx! { EffectChild {} }
        } else {
            rsx! { "no child" }
        }
    }

    let (mut dom, mut renderer) = run_generations(app, 1);

    let child_id = CHILD_SCOPE.with(|c| c.get()).expect("child scope captured");
    let grandchild_id = GRANDCHILD_SCOPE
        .with(|c| c.get())
        .expect("grandchild scope captured");

    // Inject pending effects for both the child and the grandchild so the
    // descendant arm of the `drop_scope` filter (id == effect.order.id) and
    // the `is_descendant_of` arm both fire when the parent is unmounted.
    let runtime = dom.runtime();
    runtime.in_scope(child_id, || {
        queue_effect(|| {});
    });
    runtime.in_scope(grandchild_id, || {
        queue_effect(|| {});
    });

    // Removing the child triggers `drop_scope(child)`, which then sees its own
    // and its descendant's pending effects and removes their stale entries.
    WARMUP_GEN.with(|c| c.set(1));
    dom.mark_dirty(ScopeId::APP);
    renderer.render(&mut dom);
}

/// Drive `use_before_render` and `use_after_render` hooks so the pre/post-render
/// closure loops in `run_scope` actually iterate something. The hooks are
/// pushed into the scope's `before_render`/`after_render` lists on the first
/// render, but the loops only see them on subsequent renders — so this warmup
/// captures the child's `ScopeId` on first render and marks the child dirty
/// to force a re-run that actually iterates the hook lists.
fn warmup_before_after_render_hooks() {
    use dioxus::prelude::*;
    use dioxus_core::{ScopeId, current_scope_id, use_after_render, use_before_render};
    use std::cell::Cell;

    thread_local! {
        static HOOKED_SCOPE: Cell<Option<ScopeId>> = const { Cell::new(None) };
    }

    #[component]
    #[allow(non_snake_case)]
    fn HookedChild() -> Element {
        use_before_render(|| {});
        use_after_render(|| {});
        use_hook(|| {
            HOOKED_SCOPE.with(|c| c.set(Some(current_scope_id())));
        });
        rsx! { span { "child" } }
    }

    fn app() -> Element {
        rsx! { HookedChild {} }
    }

    let (mut dom, mut renderer) = run_generations(app, 1);

    if let Some(hooked) = HOOKED_SCOPE.with(|c| c.get()) {
        dom.mark_dirty(hooked);
        renderer.render(&mut dom);
    }
}

/// Drive a component that returns `Err(RenderError::Error(_))` so the error
/// arm in `run_scope`'s `match render_return` and the error arm in
/// `handle_element_return` (which calls `throw_error`) both fire.
fn warmup_throw_error() {
    use dioxus::prelude::*;
    use dioxus_core::{CapturedError, RenderError};

    #[component]
    #[allow(non_snake_case)]
    fn Failing() -> Element {
        Err(RenderError::Error(CapturedError::from_display(
            "expected fuzz error",
        )))
    }

    #[component]
    #[allow(non_snake_case)]
    fn Boundary() -> Element {
        rsx! {
            ErrorBoundary {
                handle_error: |_err: ErrorContext| rsx! { "caught" },
                Failing {}
            }
        }
    }

    run_generations(Boundary, 1);
}

/// Keyed list with a shared left prefix, a fully-replaced middle, and no
/// shared suffix. Drives `diff_keyed_children`'s left-edge splice branch
/// (`right_offset == 0`): the new middle is created anchored against the old
/// left sibling before the old middle is removed.
fn warmup_keyed_left_prefix_splice() {
    use dioxus::prelude::*;

    fn app() -> Element {
        // gen 0: [a, x0, x1, x2]; gen 1: [a, p0, p1, p2].
        // "a" is a shared left prefix, the middle keys are entirely new, and
        // the last keys differ so there is no shared suffix.
        let keys: &[&str] = if warmup_gen() == 0 {
            &["a", "x0", "x1", "x2"]
        } else {
            &["a", "p0", "p1", "p2"]
        };
        rsx! {
            for k in keys.iter().copied() {
                div { key: "{k}", "{k}" }
            }
        }
    }

    run_generations(app, 2);
}

/// A spread adds a dynamic attribute that shadows a static template attribute
/// of the same name, then drops it. Drives the static-template-attribute
/// restore in `remove_attribute_or_restore_static`: the removed dynamic
/// attribute's static value is restored instead of cleared.
fn warmup_static_attribute_restore() {
    use dioxus::prelude::*;
    use dioxus_core::Attribute;

    fn app() -> Element {
        // The template carries a static `class="static"`. The spread shadows
        // it on gen 0, then drops it on gen 1. gen 0 uses a same-namespace
        // (`None`) dynamic attr so the dropped attr restores the static value
        // (the namespace-matches arm of the static lookup); gen 2 uses a
        // *namespaced* dynamic attr so the dropped attr finds the static name
        // but mismatches its namespace (the namespace-mismatch arm).
        let extra: Vec<Attribute> = match warmup_gen() {
            0 => vec![Attribute::new("class", "dynamic", None, false)],
            2 => vec![Attribute::new("class", "dynamic", Some("custom"), false)],
            _ => Vec::new(),
        };
        rsx! {
            div { class: "static", ..extra }
        }
    }

    // gen 1: drop the same-namespace dynamic attr -> restore the static value.
    // gen 2: add a namespaced dynamic attr, gen 3: drop it -> the static
    // lookup finds the name but mismatches the namespace.
    run_generations(app, 4);
}

/// The same spread attribute slot holds a plain value on gen 0 and a listener
/// on gen 1. Drives the `(false, true, Some(_))` arm in
/// `diff_dynamic_attribute`: the old value is explicitly cleared before the
/// listener is installed (installing a listener doesn't overwrite it).
fn warmup_attribute_value_to_listener() {
    use dioxus::prelude::*;
    use dioxus_core::{Attribute, AttributeValue};

    fn app() -> Element {
        let attrs: Vec<Attribute> = if warmup_gen() == 0 {
            vec![Attribute::new("data-x", "value", None, false)]
        } else {
            vec![Attribute::new(
                "data-x",
                AttributeValue::listener(|_: Event<MouseData>| {}),
                None,
                false,
            )]
        };
        rsx! {
            div { ..attrs }
        }
    }

    run_generations(app, 2);
}

/// A keyed list of portals whose body is a single dynamic text node mounted in
/// a *different* render target. Reordering/growing the list makes
/// `push_all_root_nodes` recurse into a portal body whose mount target differs
/// from the list's target, driving the cross-target dynamic-text-root branch.
fn warmup_portal_dynamic_text_root() {
    use dioxus::prelude::*;
    use dioxus_core::{MultiTargetWriter, Portal, RenderTargetId, ScopeId, VirtualDom};
    use dioxus_renderer_oracle::RendererOracle;
    use std::cell::Cell;

    thread_local! {
        static TARGET: Cell<RenderTargetId> = const { Cell::new(RenderTargetId::ROOT) };
    }

    fn app() -> Element {
        let target = TARGET.with(|c| c.get());
        // Reverse a fully-keyed list (shared keys, no shared prefix/suffix) so
        // `diff_keyed_middle` *moves* most entries through its splice. Each
        // moved entry is re-pushed via `push_all_root_nodes`, which recurses
        // into the portal body — a dynamic text root mounted in `target`.
        let keys: &[u32] = if warmup_gen() == 0 {
            &[0, 1, 2, 3]
        } else {
            &[3, 2, 1, 0]
        };
        rsx! {
            for k in keys.iter().copied() {
                Portal { key: "{k}", target, "{k}" }
            }
        }
    }

    WARMUP_GEN.with(|c| c.set(0));
    let mut dom = VirtualDom::new(app);
    let target = dom.runtime().create_render_target();
    TARGET.with(|c| c.set(target));
    let mut writer = MultiTargetWriter::<RendererOracle>::new();
    writer.insert(RenderTargetId::ROOT, RendererOracle::new());
    writer.insert(target, RendererOracle::new());
    dom.rebuild(&mut writer);

    // gen 1: reverse the list, forcing keyed-middle moves whose
    // `push_all_root_nodes` recurses across the portal's target boundary.
    WARMUP_GEN.with(|c| c.set(1));
    dom.mark_dirty(ScopeId::APP);
    dom.render_immediate(&mut writer);
}

/// A keyed list of single-element components that is reordered while some
/// entries are removed. During the reorder the diff resolves anchors against
/// component roots whose scopes may already have been dropped, driving the
/// dropped-scope `?` branch in `find_element_at_root_in_target`'s component arm.
fn warmup_keyed_component_anchor() {
    use dioxus::prelude::*;

    #[derive(Clone, PartialEq, Props)]
    struct ItemProps {
        value: u32,
    }

    #[allow(non_snake_case)]
    fn Item(props: ItemProps) -> Element {
        rsx! { span { "{props.value}" } }
    }

    fn app() -> Element {
        // gen 0: [a, b, c, d, e]; gen 1: [a, e] — the middle three component
        // scopes drop while the list keeps a shared prefix/suffix, so anchors
        // are resolved around components mid-removal.
        let keys: &[u32] = if warmup_gen() == 0 {
            &[0, 1, 2, 3, 4]
        } else {
            &[0, 4]
        };
        rsx! {
            for k in keys.iter().copied() {
                Item { key: "{k}", value: k }
            }
        }
    }

    run_generations(app, 2);
}

pub fn warmup_deferred_priority_paths() {
    warmup_batched_component_props_diff();
    warmup_keyed_reorder();
    warmup_suspense_hidden_paths();
    warmup_empty_fallback_slot_promotion();
    warmup_non_root_dynamic_slot_without_adjacent_anchor();
    warmup_root_dynamic_slot_anchors();
    warmup_keyed_fragment_skip_anchors();
    warmup_keyed_fragment_right_edge_anchor();
    warmup_keyed_splice_into_static_slot();
    warmup_suspense_then_remove();
    warmup_dropped_scope_anchor_lookup();
    warmup_portal_target_switch();
    warmup_scope_with_pending_effect();
    warmup_before_after_render_hooks();
    warmup_throw_error();
    warmup_deferred_subtree_check();
    warmup_keyed_left_prefix_splice();
    warmup_static_attribute_restore();
    warmup_attribute_value_to_listener();
    warmup_portal_dynamic_text_root();
    warmup_keyed_component_anchor();
    use dioxus::prelude::*;
    use dioxus_core::ScopeId;

    #[derive(Clone, PartialEq, Props)]
    struct ItemProps {
        value: u32,
    }

    #[allow(non_snake_case)]
    fn Item(props: ItemProps) -> Element {
        rsx! { span { "{props.value}" } }
    }

    fn app() -> Element {
        let generation = dioxus_core::generation();
        rsx! {
            for i in 0..3u32 {
                Item { value: i + (generation as u32) }
            }
        }
    }

    // Re-render a parent whose children are also dirty, driving the diff for a
    // small fragment of identical components.
    {
        let (mut dom, _oracle) = run_generations(app, 1);
        dom.mark_dirty(ScopeId::APP);
        drive_render(&mut dom);
    }
}
