use crate::{
    lifecycle::{self, LifecycleKey, LifecycleRole, LifecycleRun, LifecycleSnapshot},
    model::*,
    ops::{
        DynamicEdit, ModelEdit, Op, VNodeEdit, WakeMode, apply_to_model,
        clear_suspense_ready_tasks, read_model, release_suspense_ready_task,
        selected_registered_ready_suspense_key, with_model, without_suspense_ready_registration,
    },
    vdom::App,
};
use dioxus_core::{
    AttributeValue, ElementId, Event, ScopeId, Template, VirtualDom, WriteMutations,
};
use dioxus_renderer_oracle::{EventListenerTarget, RendererOracle, SnapshotNode, panic_message};
use std::{any::Any, collections::BTreeSet, fmt, panic, rc::Rc};

// ---------- Harness -------------------------------------------------------------------------

type TargetSnapshots = Vec<SnapshotNode>;

pub(crate) struct Harness {
    vdom: VirtualDom,
    incremental: TargetedRendererOracle,
    pending_app_render: bool,
    pending_fresh_compare: bool,
    strict_renderer_errors: bool,
    strict_lifecycle_errors: bool,
}

impl Harness {
    pub(crate) fn fresh() -> Self {
        Self::fresh_with_strict_options(cfg!(fuzzing), cfg!(fuzzing))
    }

    #[cfg(test)]
    fn fresh_strict() -> Self {
        Self::fresh_with_strict_options(true, false)
    }

    #[cfg(test)]
    fn fresh_strict_lifecycle() -> Self {
        Self::fresh_with_strict_options(true, true)
    }

    fn fresh_with_strict_options(
        strict_renderer_errors: bool,
        strict_lifecycle_errors: bool,
    ) -> Self {
        clear_suspense_ready_tasks();
        lifecycle::reset_all();
        with_model(|model| *model = Model::initial());
        let mut vdom = VirtualDom::new(App);
        let mut incremental = TargetedRendererOracle::new();
        lifecycle::with_run(LifecycleRun::Incremental, || vdom.rebuild(&mut incremental));
        incremental.assert_stack_clean();
        let state = Self {
            vdom,
            incremental,
            pending_app_render: false,
            pending_fresh_compare: false,
            strict_renderer_errors,
            strict_lifecycle_errors,
        };
        if strict_lifecycle_errors {
            check_lifecycle_matches_fresh().unwrap();
        }
        state
    }
}

struct TargetedRendererOracle {
    renderer: RendererOracle,
    last_mutation: Option<MutationTrace>,
    recent_mutations: [Option<MutationTrace>; RECENT_MUTATION_LIMIT],
    recent_mutation_start: usize,
    recent_mutation_len: usize,
}

const RECENT_MUTATION_LIMIT: usize = 16;

#[derive(Copy, Clone, Debug)]
enum MutationTrace {
    AppendChildren { id: ElementId, m: usize },
    AssignNodeId { path: &'static [u8], id: ElementId },
    CreatePlaceholder { id: ElementId },
    CreateTextNode { len: usize, id: ElementId },
    LoadTemplate { index: usize, id: ElementId },
    ReplaceNodeWith { id: ElementId, m: usize },
    ReplacePlaceholderWithNodes { path: &'static [u8], m: usize },
    InsertNodesAfter { id: ElementId, m: usize },
    InsertNodesBefore { id: ElementId, m: usize },
    SetAttribute { name: &'static str, id: ElementId },
    SetNodeText { len: usize, id: ElementId },
    CreateEventListener { name: &'static str, id: ElementId },
    RemoveEventListener { name: &'static str, id: ElementId },
    RemoveNode { id: ElementId },
    PushRoot { id: ElementId },
}

impl fmt::Display for MutationTrace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AppendChildren { id, m } => {
                write!(f, "append_children(id: {id:?}, m: {m})")
            }
            Self::AssignNodeId { path, id } => {
                write!(f, "assign_node_id(path: {path:?}, id: {id:?})")
            }
            Self::CreatePlaceholder { id } => write!(f, "create_placeholder(id: {id:?})"),
            Self::CreateTextNode { len, id } => {
                write!(f, "create_text_node(len: {len}, id: {id:?})")
            }
            Self::LoadTemplate { index, id } => {
                write!(f, "load_template(index: {index}, id: {id:?})")
            }
            Self::ReplaceNodeWith { id, m } => {
                write!(f, "replace_node_with(id: {id:?}, m: {m})")
            }
            Self::ReplacePlaceholderWithNodes { path, m } => {
                write!(f, "replace_placeholder_with_nodes(path: {path:?}, m: {m})")
            }
            Self::InsertNodesAfter { id, m } => {
                write!(f, "insert_nodes_after(id: {id:?}, m: {m})")
            }
            Self::InsertNodesBefore { id, m } => {
                write!(f, "insert_nodes_before(id: {id:?}, m: {m})")
            }
            Self::SetAttribute { name, id } => {
                write!(f, "set_attribute(name: {name:?}, id: {id:?})")
            }
            Self::SetNodeText { len, id } => {
                write!(f, "set_node_text(len: {len}, id: {id:?})")
            }
            Self::CreateEventListener { name, id } => {
                write!(f, "create_event_listener(name: {name:?}, id: {id:?})")
            }
            Self::RemoveEventListener { name, id } => {
                write!(f, "remove_event_listener(name: {name:?}, id: {id:?})")
            }
            Self::RemoveNode { id } => write!(f, "remove_node(id: {id:?})"),
            Self::PushRoot { id } => write!(f, "push_root(id: {id:?})"),
        }
    }
}

impl TargetedRendererOracle {
    fn new() -> Self {
        Self {
            renderer: RendererOracle::new(),
            last_mutation: None,
            recent_mutations: [None; RECENT_MUTATION_LIMIT],
            recent_mutation_start: 0,
            recent_mutation_len: 0,
        }
    }

    fn current_renderer(&mut self) -> &mut RendererOracle {
        &mut self.renderer
    }

    fn record_mutation(&mut self, mutation: MutationTrace) {
        self.last_mutation = Some(mutation);
        if self.recent_mutation_len < RECENT_MUTATION_LIMIT {
            let index =
                (self.recent_mutation_start + self.recent_mutation_len) % RECENT_MUTATION_LIMIT;
            self.recent_mutations[index] = Some(mutation);
            self.recent_mutation_len += 1;
        } else {
            self.recent_mutations[self.recent_mutation_start] = Some(mutation);
            self.recent_mutation_start = (self.recent_mutation_start + 1) % RECENT_MUTATION_LIMIT;
        }
    }

    fn recent_mutations_text(&self) -> String {
        let mut out = String::new();
        for offset in 0..self.recent_mutation_len {
            let index = (self.recent_mutation_start + offset) % RECENT_MUTATION_LIMIT;
            if let Some(mutation) = self.recent_mutations[index] {
                if !out.is_empty() {
                    out.push_str("\n  ");
                }
                out.push_str(&mutation.to_string());
            }
        }
        out
    }

    fn assert_stack_clean(&self) {
        if let Err(error) = self.check_stack_clean() {
            panic!("{error}");
        }
    }

    fn check_stack_clean(&self) -> Result<(), String> {
        self.renderer.check_stack_clean()
    }

    fn check_matches_vdom(&self, _vdom: &VirtualDom) -> Result<(), String> {
        let mut fresh_vdom = VirtualDom::new(App);
        let mut fresh = RendererOracle::new();
        without_suspense_ready_registration(|| fresh_vdom.rebuild(&mut fresh));
        fresh.check_stack_clean()?;
        if self.renderer.snapshot_eq(&fresh) {
            return Ok(());
        }

        let fresh_snapshot = fresh.snapshot();
        let incremental_snapshot = self.snapshot();
        Err(format!(
            "incremental renderer snapshot does not match fresh render\nincremental:\n{incremental_snapshot:#?}\nfresh:\n{fresh_snapshot:#?}"
        ))
    }

    fn snapshot(&self) -> TargetSnapshots {
        self.renderer.snapshot()
    }

    fn historical_event_listener_targets(&self) -> &[EventListenerTarget] {
        self.renderer.historical_event_listener_targets()
    }
}

impl WriteMutations for TargetedRendererOracle {
    fn append_children(&mut self, id: ElementId, m: usize) {
        self.record_mutation(MutationTrace::AppendChildren { id, m });
        self.current_renderer().append_children(id, m)
    }

    fn assign_node_id(&mut self, path: &'static [u8], id: ElementId) {
        self.record_mutation(MutationTrace::AssignNodeId { path, id });
        self.current_renderer().assign_node_id(path, id)
    }

    fn create_placeholder(&mut self, id: ElementId) {
        self.record_mutation(MutationTrace::CreatePlaceholder { id });
        self.current_renderer().create_placeholder(id)
    }

    fn create_text_node(&mut self, value: &str, id: ElementId) {
        self.record_mutation(MutationTrace::CreateTextNode {
            len: value.len(),
            id,
        });
        self.current_renderer().create_text_node(value, id)
    }

    fn load_template(&mut self, template: Template, index: usize, id: ElementId) {
        self.record_mutation(MutationTrace::LoadTemplate { index, id });
        self.current_renderer().load_template(template, index, id)
    }

    fn replace_node_with(&mut self, id: ElementId, m: usize) {
        self.record_mutation(MutationTrace::ReplaceNodeWith { id, m });
        self.current_renderer().replace_node_with(id, m)
    }

    fn replace_placeholder_with_nodes(&mut self, path: &'static [u8], m: usize) {
        self.record_mutation(MutationTrace::ReplacePlaceholderWithNodes { path, m });
        self.current_renderer()
            .replace_placeholder_with_nodes(path, m)
    }

    fn insert_nodes_after(&mut self, id: ElementId, m: usize) {
        self.record_mutation(MutationTrace::InsertNodesAfter { id, m });
        self.current_renderer().insert_nodes_after(id, m)
    }

    fn insert_nodes_before(&mut self, id: ElementId, m: usize) {
        self.record_mutation(MutationTrace::InsertNodesBefore { id, m });
        self.current_renderer().insert_nodes_before(id, m)
    }

    fn set_attribute(
        &mut self,
        name: &'static str,
        ns: Option<&'static str>,
        value: &AttributeValue,
        id: ElementId,
    ) {
        self.record_mutation(MutationTrace::SetAttribute { name, id });
        self.current_renderer().set_attribute(name, ns, value, id)
    }

    fn set_node_text(&mut self, value: &str, id: ElementId) {
        self.record_mutation(MutationTrace::SetNodeText {
            len: value.len(),
            id,
        });
        self.current_renderer().set_node_text(value, id)
    }

    fn create_event_listener(&mut self, name: &'static str, id: ElementId) {
        self.record_mutation(MutationTrace::CreateEventListener { name, id });
        self.current_renderer().create_event_listener(name, id)
    }

    fn remove_event_listener(&mut self, name: &'static str, id: ElementId) {
        self.record_mutation(MutationTrace::RemoveEventListener { name, id });
        self.current_renderer().remove_event_listener(name, id)
    }

    fn remove_node(&mut self, id: ElementId) {
        self.record_mutation(MutationTrace::RemoveNode { id });
        self.current_renderer().remove_node(id)
    }

    fn push_root(&mut self, id: ElementId) {
        self.record_mutation(MutationTrace::PushRoot { id });
        self.current_renderer().push_root(id)
    }
}

const TRACE_CONTEXT: usize = 6;
const MAX_HTML_CHARS: usize = 240;

fn catch_unwind_result<F, R>(f: F) -> std::thread::Result<R>
where
    F: FnOnce() -> R,
{
    panic::catch_unwind(panic::AssertUnwindSafe(f))
}

fn render_model_with_ssr(model: &Model) -> Result<String, String> {
    catch_unwind_result(|| {
        without_suspense_ready_registration(|| {
            with_model(|global| *global = model.clone());
            let mut vdom = VirtualDom::new(App);
            vdom.rebuild_in_place();
            dioxus_ssr::render(&vdom)
        })
    })
    .map_err(|payload| format!("panic in SSR render: {}", panic_message(&payload)))
}

fn print_html_line(label: &str, rendered: &Result<String, String>) {
    match rendered {
        Ok(html) => println!("    {label:<7} {}", truncate_html(html)),
        Err(err) => println!("    {label:<7} <{err}>"),
    }
}

fn truncate_html(html: &str) -> String {
    if html.chars().count() <= MAX_HTML_CHARS {
        return html.to_string();
    }

    let mut truncated = html.chars().take(MAX_HTML_CHARS).collect::<String>();
    truncated.push_str("...");
    truncated
}

fn first_line(text: &str) -> &str {
    text.lines().next().unwrap_or(text)
}

fn print_indented(text: &str, indent: &str) {
    for line in text.lines() {
        println!("{indent}{line}");
    }
}

fn print_op_list(ops: &[Op], failing_step: usize) {
    println!("operations:");
    for (index, op) in ops.iter().enumerate() {
        let marker = if index == failing_step { ">>" } else { "  " };
        println!("{marker} {index:03}: {op:?}");
    }
}

fn trace_bounds(ops_len: usize, failing_step: usize) -> (usize, usize) {
    if ops_len <= TRACE_CONTEXT * 4 {
        return (0, ops_len);
    }

    (
        failing_step.saturating_sub(TRACE_CONTEXT),
        (failing_step + TRACE_CONTEXT + 1).min(ops_len),
    )
}

pub(crate) fn print_ssr_diff_trace(ops: &[Op], failing_step: usize, minimized_error: &str) {
    let panic_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));

    println!();
    println!("dioxus-vdom-fuzz failure");
    println!("decoded operations: {}", ops.len());
    println!("reported failing step: {failing_step}");
    println!("summary: {}", first_line(minimized_error));
    println!();
    print_op_list(ops, failing_step);
    println!();
    println!("ssr replay around failing step:");

    let mut state = Harness::fresh();
    let mut current_model = Model::initial();
    let mut current_html = render_model_with_ssr(&current_model);
    let (trace_start, trace_end) = trace_bounds(ops.len(), failing_step);

    if trace_start == 0 {
        println!("  initial");
        print_html_line("html:", &current_html);
    } else {
        println!("  replaying first {trace_start} steps without logging");
    }

    let mut reproduced_error = None;
    for (index, op) in ops.iter().enumerate() {
        with_model(|global| *global = current_model.clone());
        let should_log = index >= trace_start && index < trace_end;

        if should_log {
            println!();
            println!("  step {index}");
            println!("    op:     {op:?}");
            print_html_line("before:", &current_html);
        }

        match apply_op(&mut state, op) {
            Ok(()) => {
                let next_model = read_model();
                let next_html = render_model_with_ssr(&next_model);
                if should_log {
                    print_html_line("after:", &next_html);
                    println!("    status: ok");
                }
                current_model = next_model;
                current_html = next_html;
            }
            Err(err) => {
                let next_model = read_model();
                let next_html = render_model_with_ssr(&next_model);
                print_html_line("after:", &next_html);
                println!("    error:  {}", first_line(&err));
                println!();
                println!("full oracle error:");
                print_indented(&err, "  ");
                reproduced_error = Some(err);
                break;
            }
        }
    }

    if reproduced_error.is_none() {
        println!();
        println!("  replay completed without reproducing the minimized error:");
        println!("    {minimized_error}");
    }
    std::panic::set_hook(panic_hook);
}

pub(crate) fn apply_step(state: &mut Harness, op: &Op) -> Result<(), String> {
    apply_op(state, op)
}

fn apply_op(state: &mut Harness, op: &Op) -> Result<(), String> {
    match op {
        Op::Rerender => render_and_assert(state),
        Op::WakeSuspense {
            suspense,
            mode: WakeMode::Harness,
        } => {
            let Some(key) = read_model().selected_ready_suspense_key(*suspense) else {
                return Ok(());
            };
            apply_to_model(op);
            update_pending_fresh_compare(state, op);
            release_suspense_ready_task(key);
            render_and_assert(state)
        }
        Op::WakeSuspense {
            suspense,
            mode: WakeMode::Natural,
        } => {
            let Some(key) = selected_registered_ready_suspense_key(*suspense) else {
                return Ok(());
            };
            with_model(|model| model.resolve_ready_suspense(key));
            update_pending_fresh_compare(state, op);
            release_suspense_ready_task(key);
            let compare_fresh = !state.pending_app_render;
            render_natural_and_assert(state, compare_fresh)
        }
        _ => {
            apply_to_model(op);
            if op_requires_app_render(op) {
                state.pending_app_render = true;
            }
            update_pending_fresh_compare(state, op);
            Ok(())
        }
    }
}

fn op_requires_app_render(op: &Op) -> bool {
    matches!(
        op,
        Op::Mutate(ModelEdit::VNode { .. }) | Op::Mutate(ModelEdit::Suspense { .. })
    )
}

fn update_pending_fresh_compare(state: &mut Harness, op: &Op) {
    if op_blocks_fresh_compare(op) {
        state.pending_fresh_compare = false;
    } else if op_requires_fresh_compare(op) {
        state.pending_fresh_compare = true;
    }
}

fn op_requires_fresh_compare(op: &Op) -> bool {
    match op {
        Op::Mutate(ModelEdit::VNode { edit, .. }) => !vnode_edit_blocks_fresh_compare(edit),
        Op::Rerender | Op::WakeSuspense { .. } | Op::Mutate(ModelEdit::Suspense { .. }) => false,
    }
}

fn op_blocks_fresh_compare(op: &Op) -> bool {
    // Suspense transitions can legitimately leave the incremental renderer on
    // fallback output while a fresh rebuild observes the updated model.
    match op {
        Op::WakeSuspense { .. } | Op::Mutate(ModelEdit::Suspense { .. }) => true,
        Op::Mutate(ModelEdit::VNode { edit, .. }) => vnode_edit_blocks_fresh_compare(edit),
        Op::Rerender => false,
    }
}

fn vnode_edit_blocks_fresh_compare(edit: &VNodeEdit) -> bool {
    matches!(
        edit,
        VNodeEdit::DynamicSlot {
            edit: DynamicEdit::SetKind(DynamicKind::Suspense { .. }),
            ..
        }
    )
}

fn fire_historical_event_listeners(state: &Harness) -> Result<(), String> {
    let targets = state.incremental.historical_event_listener_targets();
    if targets.is_empty() {
        return Ok(());
    }

    let runtime = state.vdom.runtime();
    for target in targets {
        let event = Event::new(
            Rc::new(String::from("fuzzer stale event")) as Rc<dyn Any>,
            true,
        );
        runtime.handle_event(target.name, event, target.id);
    }
    Ok(())
}

fn render_once(
    state: &mut Harness,
    mark_app_dirty: bool,
    assert_matches_vdom: bool,
    assert_lifecycle_matches_fresh: bool,
) -> Result<(), String> {
    fire_historical_event_listeners(state)?;
    if mark_app_dirty {
        state.vdom.mark_dirty(ScopeId::APP);
    }
    lifecycle::with_run(LifecycleRun::Incremental, || {
        state.vdom.render_immediate(&mut state.incremental)
    });
    state.incremental.check_stack_clean().map_err(|err| {
        let last_mutation = state
            .incremental
            .last_mutation
            .map_or_else(|| "<none>".to_string(), |mutation| mutation.to_string());
        let recent_mutations = state.incremental.recent_mutations_text();
        format!("{err} after {last_mutation}\nrecent mutations:\n  {recent_mutations}")
    })?;
    if assert_matches_vdom {
        state.incremental.check_matches_vdom(&state.vdom)?;
    }
    if assert_lifecycle_matches_fresh {
        check_lifecycle_matches_fresh().map_err(|err| {
            let last_mutation = state
                .incremental
                .last_mutation
                .map_or_else(|| "<none>".to_string(), |mutation| mutation.to_string());
            let recent_mutations = state.incremental.recent_mutations_text();
            format!("{err} after {last_mutation}\nrecent mutations:\n  {recent_mutations}")
        })?;
    }
    Ok(())
}

fn render_and_assert(state: &mut Harness) -> Result<(), String> {
    let compare_fresh = state.pending_fresh_compare;
    let compare_lifecycle = state.strict_lifecycle_errors;
    let result = render_once(state, true, compare_fresh, compare_lifecycle);
    state.pending_app_render = false;
    state.pending_fresh_compare = false;
    render_result_to_fuzz_failure(state, result)
}

fn render_natural_and_assert(state: &mut Harness, compare_fresh: bool) -> Result<(), String> {
    // Natural suspense wakes can observe an intermediate render pass where a
    // dirty boundary is processed before the released task is polled. The
    // renderer output must still match, but lifecycle state may not settle
    // until a later queued pass.
    let compare_lifecycle = false;
    let result = render_once(
        state,
        false,
        compare_fresh && state.pending_fresh_compare,
        compare_lifecycle,
    );
    if compare_fresh {
        state.pending_fresh_compare = false;
    }
    render_result_to_fuzz_failure(state, result)
}

fn check_lifecycle_matches_fresh() -> Result<(), String> {
    lifecycle::reset_run(LifecycleRun::Fresh);
    let mut fresh_vdom = VirtualDom::new(App);
    let mut fresh_renderer = RendererOracle::new();
    without_suspense_ready_registration(|| {
        lifecycle::with_run(LifecycleRun::Fresh, || {
            fresh_vdom.rebuild(&mut fresh_renderer)
        });
    });
    fresh_renderer.check_stack_clean()?;

    let incremental = lifecycle::snapshot(LifecycleRun::Incremental);
    let fresh = lifecycle::snapshot(LifecycleRun::Fresh);
    let model = expected_model_lifecycle_snapshot();
    if lifecycle_is_within_expected_bounds(&incremental, &fresh, &model) {
        return Ok(());
    }

    let retaining_suspense_ids = retaining_suspense_ids(&incremental, &fresh, &model);
    let retained_suspended = lifecycle::snapshot_with_suspense_ancestor(
        LifecycleRun::Incremental,
        &retaining_suspense_ids,
    );
    let model_suspended = model_lifecycle_with_suspense_ancestor_snapshot(&retaining_suspense_ids);
    Err(lifecycle_mismatch_error(
        &incremental,
        &fresh,
        &model,
        &retained_suspended,
        &model_suspended,
    ))
}

fn lifecycle_is_within_expected_bounds(
    incremental: &LifecycleSnapshot,
    fresh: &LifecycleSnapshot,
    model: &LifecycleSnapshot,
) -> bool {
    let retaining_suspense_ids = retaining_suspense_ids(incremental, fresh, model);
    let retained_suspended_subtree_lifecycle = lifecycle::snapshot_with_suspense_ancestor(
        LifecycleRun::Incremental,
        &retaining_suspense_ids,
    );
    let model_suspended_subtree_lifecycle =
        model_lifecycle_with_suspense_ancestor_snapshot(&retaining_suspense_ids);
    let has_all_visible_fresh_components = fresh
        .iter()
        .filter(|(key, _)| lifecycle_role_is_strict(**key))
        .all(|(key, count)| incremental.get(key).copied().unwrap_or(0) >= *count);
    let has_no_components_outside_the_model = incremental
        .iter()
        .filter(|(key, _)| lifecycle_role_is_strict(**key))
        .all(|(key, count)| {
            let model_count = model.get(key).copied().unwrap_or(0);
            let retained_suspended_count = retained_suspended_subtree_lifecycle
                .get(key)
                .copied()
                .unwrap_or(0);
            let model_suspended_count = model_suspended_subtree_lifecycle
                .get(key)
                .copied()
                .unwrap_or(0);
            let retained_extra_count =
                retained_suspended_count.saturating_sub(model_suspended_count);
            *count <= model_count + retained_extra_count
        });
    has_all_visible_fresh_components && has_no_components_outside_the_model
}

fn lifecycle_role_is_strict(key: LifecycleKey) -> bool {
    // Suspense helper components can overlap while core moves work between
    // visible and suspended trees. The strict oracle targets generated app
    // components, where a live key outside the model means stale state.
    matches!(
        key.role,
        LifecycleRole::ComponentA | LifecycleRole::ComponentB
    )
}

fn expected_model_lifecycle_snapshot() -> LifecycleSnapshot {
    let model = read_model();
    let mut out = LifecycleSnapshot::new();
    collect_vnode_lifecycle(&model.root, &mut out);
    out
}

fn retaining_suspense_ids(
    incremental: &LifecycleSnapshot,
    fresh: &LifecycleSnapshot,
    model: &LifecycleSnapshot,
) -> BTreeSet<u64> {
    let current_model = read_model();
    let mut out = BTreeSet::new();
    // Core suspense can retain previous child state while a reused boundary
    // moves between fallback and resolved output, even if the model suspense is
    // currently resolved. Bound retained extras by current boundary ancestry.
    collect_current_suspense_ids(&current_model.root, &mut out);

    for (key, count) in incremental {
        if key.role != LifecycleRole::SuspenseChild {
            continue;
        }

        let fresh_count = fresh.get(key).copied().unwrap_or(0);
        let model_count = model.get(key).copied().unwrap_or(0);
        if (fresh_count > 0 || model_count > 0) && *count > fresh_count.max(model_count) {
            out.insert(key.id);
        }
    }

    out
}

fn model_lifecycle_with_suspense_ancestor_snapshot(
    suspense_ids: &BTreeSet<u64>,
) -> LifecycleSnapshot {
    let model = read_model();
    let mut out = LifecycleSnapshot::new();
    collect_model_lifecycle_with_suspense_ancestor(&model.root, false, suspense_ids, &mut out);
    out
}

fn collect_current_suspense_ids(vnode: &VNodeSpec, out: &mut BTreeSet<u64>) {
    for dynamic in &vnode.dynamics {
        collect_dynamic_current_suspense_ids(dynamic, out);
    }
}

fn collect_dynamic_current_suspense_ids(dynamic: &DynamicSpec, out: &mut BTreeSet<u64>) {
    match dynamic {
        DynamicSpec::Fragment(nodes) => {
            for node in nodes {
                collect_current_suspense_ids(node, out);
            }
        }
        DynamicSpec::ComponentA(component) | DynamicSpec::ComponentB(component) => {
            collect_current_suspense_ids(&component.child, out);
        }
        DynamicSpec::Suspense(spec) => {
            out.insert(spec.id);
            collect_current_suspense_ids(&spec.child, out);
        }
        DynamicSpec::Empty | DynamicSpec::Text(_) | DynamicSpec::Placeholder => {}
    }
}

fn collect_model_lifecycle_with_suspense_ancestor(
    vnode: &VNodeSpec,
    within_retaining_suspense: bool,
    suspense_ids: &BTreeSet<u64>,
    out: &mut LifecycleSnapshot,
) {
    for dynamic in &vnode.dynamics {
        collect_model_dynamic_lifecycle_with_suspense_ancestor(
            dynamic,
            within_retaining_suspense,
            suspense_ids,
            out,
        );
    }
}

fn collect_model_dynamic_lifecycle_with_suspense_ancestor(
    dynamic: &DynamicSpec,
    within_retaining_suspense: bool,
    suspense_ids: &BTreeSet<u64>,
    out: &mut LifecycleSnapshot,
) {
    match dynamic {
        DynamicSpec::Fragment(nodes) => {
            for node in nodes {
                collect_model_lifecycle_with_suspense_ancestor(
                    node,
                    within_retaining_suspense,
                    suspense_ids,
                    out,
                );
            }
        }
        DynamicSpec::ComponentA(component) => {
            if within_retaining_suspense {
                add_lifecycle_key(out, LifecycleRole::ComponentA, component.id);
            }
            collect_model_lifecycle_with_suspense_ancestor(
                &component.child,
                within_retaining_suspense,
                suspense_ids,
                out,
            );
        }
        DynamicSpec::ComponentB(component) => {
            if within_retaining_suspense {
                add_lifecycle_key(out, LifecycleRole::ComponentB, component.id);
            }
            collect_model_lifecycle_with_suspense_ancestor(
                &component.child,
                within_retaining_suspense,
                suspense_ids,
                out,
            );
        }
        DynamicSpec::Suspense(spec) => {
            collect_model_lifecycle_with_suspense_ancestor(
                &spec.child,
                within_retaining_suspense || suspense_ids.contains(&spec.id),
                suspense_ids,
                out,
            );
        }
        DynamicSpec::Empty | DynamicSpec::Text(_) | DynamicSpec::Placeholder => {}
    }
}

fn collect_vnode_lifecycle(vnode: &VNodeSpec, out: &mut LifecycleSnapshot) {
    for dynamic in &vnode.dynamics {
        collect_dynamic_lifecycle(dynamic, out);
    }
}

fn collect_dynamic_lifecycle(dynamic: &DynamicSpec, out: &mut LifecycleSnapshot) {
    match dynamic {
        DynamicSpec::Fragment(nodes) => {
            for node in nodes {
                collect_vnode_lifecycle(node, out);
            }
        }
        DynamicSpec::ComponentA(component) => {
            add_lifecycle_key(out, LifecycleRole::ComponentA, component.id);
            collect_vnode_lifecycle(&component.child, out);
        }
        DynamicSpec::ComponentB(component) => {
            add_lifecycle_key(out, LifecycleRole::ComponentB, component.id);
            collect_vnode_lifecycle(&component.child, out);
        }
        DynamicSpec::Suspense(spec) => {
            add_lifecycle_key(out, LifecycleRole::SuspenseBoundary, spec.id);
            add_lifecycle_key(out, LifecycleRole::SuspenseChild, spec.id);
            collect_vnode_lifecycle(&spec.child, out);
        }
        DynamicSpec::Empty | DynamicSpec::Text(_) | DynamicSpec::Placeholder => {}
    }
}

fn add_lifecycle_key(out: &mut LifecycleSnapshot, role: LifecycleRole, id: u64) {
    *out.entry(LifecycleKey { role, id }).or_insert(0) += 1;
}

fn lifecycle_mismatch_error(
    incremental: &LifecycleSnapshot,
    fresh: &LifecycleSnapshot,
    model: &LifecycleSnapshot,
    retained_suspended: &LifecycleSnapshot,
    model_suspended: &LifecycleSnapshot,
) -> String {
    format!(
        "incremental component lifecycle set is outside fresh/model bounds\nincremental:\n{incremental:#?}\nvisible fresh:\n{fresh:#?}\nmodel upper bound:\n{model:#?}\nretained suspended incremental:\n{retained_suspended:#?}\nmodel suspended subtree:\n{model_suspended:#?}"
    )
}

fn render_result_to_fuzz_failure(
    state: &Harness,
    result: Result<(), String>,
) -> Result<(), String> {
    if state.strict_renderer_errors {
        result.map(|_| ())
    } else {
        let _ = result;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        model::{
            AttrSpec, AttrValueSpec, DynamicKind, FragmentKeyMode, SuspenseMode, TemplateAttrSpec,
            TemplateNodeKind, WakeMutationSpec,
        },
        ops::{FragmentEdit, ListEdit, TemplateEdit},
    };

    fn replay_ops(ops: impl IntoIterator<Item = Op>) {
        let mut harness = Harness::fresh_strict();
        for op in ops {
            apply_op(&mut harness, &op).unwrap();
        }
    }

    fn replay_ops_with_lifecycle(ops: impl IntoIterator<Item = Op>) {
        let mut harness = Harness::fresh_strict_lifecycle();
        for op in ops {
            apply_op(&mut harness, &op).unwrap();
        }
    }

    fn set_pending_suspense_model() {
        with_model(|model| *model = Model::initial());
        apply_to_model(&Op::template(
            0,
            TemplateEdit::SetNode {
                node: 0,
                kind: TemplateNodeKind::Dynamic,
            },
        ));
        apply_to_model(&Op::dynamic(
            0,
            0,
            DynamicKind::Suspense {
                mode: SuspenseMode::Pending,
            },
        ));
    }

    #[test]
    fn vnode_mutation_arms_fresh_render_compare() {
        let mut harness = Harness::fresh_strict();

        apply_op(
            &mut harness,
            &Op::template(
                0,
                TemplateEdit::SetNode {
                    node: 0,
                    kind: TemplateNodeKind::Dynamic,
                },
            ),
        )
        .unwrap();

        assert!(harness.pending_app_render);
        assert!(harness.pending_fresh_compare);

        apply_op(&mut harness, &Op::Rerender).unwrap();

        assert!(!harness.pending_app_render);
        assert!(!harness.pending_fresh_compare);
    }

    #[test]
    fn suspense_slot_mutation_disarms_fresh_render_compare() {
        let mut harness = Harness::fresh_strict();

        apply_op(
            &mut harness,
            &Op::template(
                0,
                TemplateEdit::SetNode {
                    node: 0,
                    kind: TemplateNodeKind::Dynamic,
                },
            ),
        )
        .unwrap();
        apply_op(
            &mut harness,
            &Op::dynamic(
                0,
                0,
                DynamicKind::Suspense {
                    mode: SuspenseMode::Ready,
                },
            ),
        )
        .unwrap();

        assert!(harness.pending_app_render);
        assert!(!harness.pending_fresh_compare);
    }

    #[test]
    fn resolved_suspense_with_edited_child_matches_fresh_render() {
        replay_ops([
            Op::template(
                0,
                TemplateEdit::SetNode {
                    node: 0,
                    kind: TemplateNodeKind::Dynamic,
                },
            ),
            Op::dynamic(
                0,
                0,
                DynamicKind::Suspense {
                    mode: SuspenseMode::Ready,
                },
            ),
            Op::Rerender,
            Op::suspense(240, SuspenseMode::Resolved),
            Op::dynamic(1, 51, DynamicKind::ComponentA),
            Op::Rerender,
        ]);
    }

    #[test]
    fn lifecycle_oracle_rejects_stale_component_outside_unresolved_suspense() {
        lifecycle::reset_all();
        set_pending_suspense_model();

        let stale_key = LifecycleKey {
            role: LifecycleRole::ComponentA,
            id: 99,
        };
        let incremental = LifecycleSnapshot::from([(stale_key, 1)]);
        let fresh = LifecycleSnapshot::new();
        let model = expected_model_lifecycle_snapshot();

        assert!(!lifecycle_is_within_expected_bounds(
            &incremental,
            &fresh,
            &model
        ));
    }

    #[test]
    fn lifecycle_oracle_allows_stale_component_inside_unresolved_suspense() {
        lifecycle::reset_all();
        set_pending_suspense_model();

        let _guard = lifecycle::with_run(LifecycleRun::Incremental, || {
            lifecycle::track(LifecycleRole::ComponentA, 99, &[0])
        });
        let incremental = lifecycle::snapshot(LifecycleRun::Incremental);
        let fresh = LifecycleSnapshot::new();
        let model = expected_model_lifecycle_snapshot();

        assert!(lifecycle_is_within_expected_bounds(
            &incremental,
            &fresh,
            &model
        ));
    }

    // Regression test for a panic in `SuspenseContext::remove_suspended_task` when
    // a nested suspense boundary was unmounted while a child task was still suspended.
    // The boundary scope was dropped before the task cleanup ran, so `needs_update`
    // unwrapped a `None` scope state.
    #[test]
    fn unmounting_nested_pending_suspense_does_not_panic_on_drop() {
        replay_ops([
            Op::template(
                0,
                TemplateEdit::SetNode {
                    node: 0,
                    kind: TemplateNodeKind::Dynamic,
                },
            ),
            Op::dynamic(
                0,
                0,
                DynamicKind::Suspense {
                    mode: SuspenseMode::Resolved,
                },
            ),
            Op::template(
                1,
                TemplateEdit::SetNode {
                    node: 0,
                    kind: TemplateNodeKind::Dynamic,
                },
            ),
            Op::dynamic(
                1,
                0,
                DynamicKind::Suspense {
                    mode: SuspenseMode::Ready,
                },
            ),
            Op::Rerender,
            Op::suspense(0, SuspenseMode::Pending),
            Op::dynamic(1, 0, DynamicKind::Placeholder),
            Op::Rerender,
            Op::suspense(0, SuspenseMode::Resolved),
            Op::Rerender,
        ]);
    }

    #[test]
    fn replacing_root_component_with_fragment_removes_old_subtree() {
        replay_ops([
            Op::template(
                0,
                TemplateEdit::SetNode {
                    node: 0,
                    kind: TemplateNodeKind::Dynamic,
                },
            ),
            Op::dynamic(0, 0, DynamicKind::ComponentA),
            Op::Rerender,
            Op::fragment(
                0,
                0,
                FragmentEdit::Children(ListEdit::Insert {
                    index: 0,
                    item: None,
                }),
            ),
            Op::Rerender,
        ]);
    }

    #[test]
    fn keyed_fragment_move_with_component_child_skips_placeholder_root() {
        replay_ops([
            Op::template(
                0,
                TemplateEdit::SetNode {
                    node: 0,
                    kind: TemplateNodeKind::Dynamic,
                },
            ),
            Op::fragment(
                0,
                0,
                FragmentEdit::Children(ListEdit::Insert {
                    index: 0,
                    item: None,
                }),
            ),
            Op::template(
                1,
                TemplateEdit::SetNode {
                    node: 0,
                    kind: TemplateNodeKind::Dynamic,
                },
            ),
            Op::dynamic(1, 0, DynamicKind::ComponentA),
            Op::fragment(
                0,
                0,
                FragmentEdit::KeyMode(FragmentKeyMode::Keyed { base: 0 }),
            ),
            Op::fragment(
                0,
                0,
                FragmentEdit::Children(ListEdit::Insert {
                    index: 0,
                    item: None,
                }),
            ),
            Op::Rerender,
            Op::fragment(
                0,
                0,
                FragmentEdit::Children(ListEdit::Move { from: 1, to: 0 }),
            ),
            Op::Rerender,
        ]);
    }

    #[test]
    fn hidden_suspense_diff_drops_removed_generated_component() {
        replay_ops_with_lifecycle([
            Op::template(
                0,
                TemplateEdit::SetNode {
                    node: 0,
                    kind: TemplateNodeKind::Dynamic,
                },
            ),
            Op::dynamic(
                0,
                0,
                DynamicKind::Suspense {
                    mode: SuspenseMode::Resolved,
                },
            ),
            Op::template(
                1,
                TemplateEdit::Children {
                    element: 0,
                    edit: ListEdit::Insert {
                        index: 0,
                        item: TemplateNodeKind::Dynamic,
                    },
                },
            ),
            Op::template(
                1,
                TemplateEdit::Children {
                    element: 0,
                    edit: ListEdit::Insert {
                        index: 1,
                        item: TemplateNodeKind::Dynamic,
                    },
                },
            ),
            Op::dynamic(
                1,
                0,
                DynamicKind::Suspense {
                    mode: SuspenseMode::Pending,
                },
            ),
            Op::dynamic(1, 1, DynamicKind::ComponentA),
            Op::Rerender,
            Op::template(
                1,
                TemplateEdit::Children {
                    element: 0,
                    edit: ListEdit::Remove { index: 1 },
                },
            ),
            Op::Rerender,
        ]);
    }

    #[test]
    fn reused_component_scope_updates_lifecycle_identity() {
        replay_ops_with_lifecycle([
            Op::template(
                0,
                TemplateEdit::SetNode {
                    node: 51,
                    kind: TemplateNodeKind::Dynamic,
                },
            ),
            Op::dynamic(0, 0, DynamicKind::ComponentA),
            Op::Rerender,
            Op::Rerender,
            Op::Rerender,
            Op::dynamic(98, 73, DynamicKind::Empty),
            Op::dynamic(0, 0, DynamicKind::ComponentA),
            Op::Rerender,
        ]);
    }

    #[test]
    fn pending_parent_may_retain_rendered_nested_suspense_lifecycle() {
        replay_ops_with_lifecycle([
            Op::template(
                0,
                TemplateEdit::SetNode {
                    node: 0,
                    kind: TemplateNodeKind::Dynamic,
                },
            ),
            Op::dynamic(
                195,
                186,
                DynamicKind::Suspense {
                    mode: SuspenseMode::Ready,
                },
            ),
            Op::Rerender,
            Op::Rerender,
            Op::Rerender,
            Op::Rerender,
            Op::template(
                1,
                TemplateEdit::SetNode {
                    node: 207,
                    kind: TemplateNodeKind::Dynamic,
                },
            ),
            Op::Rerender,
            Op::dynamic(
                39,
                114,
                DynamicKind::Suspense {
                    mode: SuspenseMode::Pending,
                },
            ),
            Op::Rerender,
            Op::wake_suspense(4),
            Op::Rerender,
            Op::wake_suspense_natural(210),
            Op::Rerender,
            Op::suspense(0, SuspenseMode::Pending),
            Op::Rerender,
        ]);
    }

    #[test]
    fn suspense_child_helper_overlap_does_not_fail_lifecycle_oracle() {
        replay_ops_with_lifecycle([
            Op::template(
                0,
                TemplateEdit::SetNode {
                    node: 0,
                    kind: TemplateNodeKind::Dynamic,
                },
            ),
            Op::Rerender,
            Op::dynamic(
                195,
                186,
                DynamicKind::Suspense {
                    mode: SuspenseMode::Resolved,
                },
            ),
            Op::Rerender,
            Op::Rerender,
            Op::Rerender,
            Op::Rerender,
            Op::template(
                1,
                TemplateEdit::SetNode {
                    node: 207,
                    kind: TemplateNodeKind::Dynamic,
                },
            ),
            Op::Rerender,
            Op::Rerender,
            Op::dynamic(
                1,
                0,
                DynamicKind::Suspense {
                    mode: SuspenseMode::Pending,
                },
            ),
            Op::wake_suspense(130),
            Op::wake_suspense_natural(167),
            Op::Rerender,
            Op::suspense(245, SuspenseMode::Ready),
            Op::Rerender,
            Op::suspense(0, SuspenseMode::Pending),
            Op::Rerender,
        ]);
    }

    #[test]
    fn resolving_parent_reuses_mounted_nested_suspense_children() {
        replay_ops_with_lifecycle([
            Op::template(
                50,
                TemplateEdit::SetNode {
                    node: 196,
                    kind: TemplateNodeKind::Dynamic,
                },
            ),
            Op::dynamic(109, 211, DynamicKind::ComponentB),
            Op::template(
                1,
                TemplateEdit::SetNode {
                    node: 0,
                    kind: TemplateNodeKind::Dynamic,
                },
            ),
            Op::dynamic(
                15,
                170,
                DynamicKind::Suspense {
                    mode: SuspenseMode::Pending,
                },
            ),
            Op::template(
                2,
                TemplateEdit::SetNode {
                    node: 0,
                    kind: TemplateNodeKind::Dynamic,
                },
            ),
            Op::dynamic(
                2,
                0,
                DynamicKind::Suspense {
                    mode: SuspenseMode::Resolved,
                },
            ),
            Op::template(
                47,
                TemplateEdit::Roots {
                    edit: ListEdit::Insert {
                        index: 20,
                        item: TemplateNodeKind::Dynamic,
                    },
                },
            ),
            Op::Rerender,
            Op::dynamic(3, 0, DynamicKind::ComponentB),
            Op::suspense(124, SuspenseMode::Resolved),
            Op::Rerender,
            Op::suspense(23, SuspenseMode::Ready),
            Op::wake_suspense(50),
        ]);
    }

    #[test]
    fn hidden_template_replace_drops_unmounted_component_state() {
        replay_ops_with_lifecycle([
            Op::template(
                0,
                TemplateEdit::SetNode {
                    node: 0,
                    kind: TemplateNodeKind::Dynamic,
                },
            ),
            Op::dynamic(
                0,
                0,
                DynamicKind::Suspense {
                    mode: SuspenseMode::Pending,
                },
            ),
            Op::template(
                1,
                TemplateEdit::SetNode {
                    node: 0,
                    kind: TemplateNodeKind::Dynamic,
                },
            ),
            Op::template(
                1,
                TemplateEdit::Roots {
                    edit: ListEdit::Insert {
                        index: 16,
                        item: TemplateNodeKind::Text(88),
                    },
                },
            ),
            Op::dynamic(1, 0, DynamicKind::ComponentB),
            Op::Rerender,
            Op::suspense(0, SuspenseMode::Ready),
            Op::Rerender,
            Op::suspense_wake_mutation(0, WakeMutationSpec::PrependStaticRoot { tag: 127 }),
            Op::Rerender,
            Op::wake_suspense(0),
            Op::suspense_wake_mutation(0, WakeMutationSpec::None),
            Op::Rerender,
        ]);
    }

    #[test]
    fn suspended_component_may_retain_previous_generated_child() {
        replay_ops_with_lifecycle([
            Op::template(
                0,
                TemplateEdit::SetNode {
                    node: 0,
                    kind: TemplateNodeKind::Dynamic,
                },
            ),
            Op::dynamic(
                0,
                0,
                DynamicKind::Suspense {
                    mode: SuspenseMode::Ready,
                },
            ),
            Op::template(
                1,
                TemplateEdit::SetNode {
                    node: 0,
                    kind: TemplateNodeKind::Dynamic,
                },
            ),
            Op::dynamic(1, 0, DynamicKind::ComponentA),
            Op::Rerender,
            Op::wake_suspense(0),
            Op::dynamic(
                1,
                0,
                DynamicKind::Suspense {
                    mode: SuspenseMode::Ready,
                },
            ),
            Op::suspense(0, SuspenseMode::Ready),
            Op::Rerender,
        ]);
    }

    #[test]
    fn nested_ready_rewake_may_retain_current_generated_child() {
        replay_ops_with_lifecycle([
            Op::template(
                50,
                TemplateEdit::SetNode {
                    node: 189,
                    kind: TemplateNodeKind::Dynamic,
                },
            ),
            Op::dynamic(
                0,
                0,
                DynamicKind::Suspense {
                    mode: SuspenseMode::Ready,
                },
            ),
            Op::template(
                1,
                TemplateEdit::SetNode {
                    node: 0,
                    kind: TemplateNodeKind::Dynamic,
                },
            ),
            Op::dynamic(
                15,
                170,
                DynamicKind::Suspense {
                    mode: SuspenseMode::Ready,
                },
            ),
            Op::template(
                2,
                TemplateEdit::SetNode {
                    node: 0,
                    kind: TemplateNodeKind::Dynamic,
                },
            ),
            Op::dynamic(2, 0, DynamicKind::ComponentA),
            Op::suspense(83, SuspenseMode::Pending),
            Op::wake_suspense(0),
            Op::Rerender,
            Op::suspense(204, SuspenseMode::Ready),
            Op::Rerender,
            Op::wake_suspense(2),
            Op::suspense(31, SuspenseMode::Ready),
            Op::Rerender,
            Op::Rerender,
            Op::suspense(2, SuspenseMode::Ready),
            Op::wake_suspense_natural(0),
            Op::Rerender,
            Op::wake_suspense(50),
        ]);
    }

    #[test]
    fn suspending_updated_child_drops_previous_generated_output() {
        replay_ops_with_lifecycle([
            Op::template(
                50,
                TemplateEdit::SetNode {
                    node: 84,
                    kind: TemplateNodeKind::Dynamic,
                },
            ),
            Op::dynamic(
                0,
                0,
                DynamicKind::Suspense {
                    mode: SuspenseMode::Ready,
                },
            ),
            Op::template(
                1,
                TemplateEdit::SetNode {
                    node: 0,
                    kind: TemplateNodeKind::Dynamic,
                },
            ),
            Op::Rerender,
            Op::dynamic(1, 0, DynamicKind::ComponentB),
            Op::Rerender,
            Op::wake_suspense_natural(164),
            Op::dynamic(0, 0, DynamicKind::ComponentB),
            Op::dynamic(
                0,
                0,
                DynamicKind::Suspense {
                    mode: SuspenseMode::Ready,
                },
            ),
            Op::Rerender,
        ]);
    }

    #[test]
    fn stale_suspended_output_reclaim_is_idempotent() {
        replay_ops_with_lifecycle([
            Op::template(
                50,
                TemplateEdit::SetNode {
                    node: 2,
                    kind: TemplateNodeKind::Dynamic,
                },
            ),
            Op::Rerender,
            Op::Rerender,
            Op::wake_suspense_natural(104),
            Op::dynamic(
                0,
                0,
                DynamicKind::Suspense {
                    mode: SuspenseMode::Ready,
                },
            ),
            Op::template(
                1,
                TemplateEdit::SetNode {
                    node: 0,
                    kind: TemplateNodeKind::Dynamic,
                },
            ),
            Op::wake_suspense(94),
            Op::Rerender,
            Op::suspense(50, SuspenseMode::Ready),
            Op::Rerender,
            Op::wake_suspense_natural(120),
            Op::template(
                3,
                TemplateEdit::Roots {
                    edit: ListEdit::Remove { index: 3 },
                },
            ),
            Op::dynamic(2, 0, DynamicKind::Text(7)),
            Op::Rerender,
        ]);
    }

    #[test]
    fn anchor_only_root_fragment_child_materializes_before_sibling() {
        replay_ops([
            Op::template(
                0,
                TemplateEdit::SetNode {
                    node: 0,
                    kind: TemplateNodeKind::Dynamic,
                },
            ),
            Op::fragment(
                0,
                0,
                FragmentEdit::Children(ListEdit::Insert {
                    index: 0,
                    item: None,
                }),
            ),
            Op::fragment(
                0,
                0,
                FragmentEdit::Children(ListEdit::Insert {
                    index: 0,
                    item: None,
                }),
            ),
            Op::template(
                1,
                TemplateEdit::SetNode {
                    node: 0,
                    kind: TemplateNodeKind::Dynamic,
                },
            ),
            Op::Rerender,
            Op::dynamic(1, 0, DynamicKind::Text(0)),
            Op::Rerender,
        ]);
    }

    #[test]
    fn replacing_root_component_with_static_text_uses_root_anchor() {
        replay_ops([
            Op::template(
                0,
                TemplateEdit::SetNode {
                    node: 0,
                    kind: TemplateNodeKind::Dynamic,
                },
            ),
            Op::dynamic(0, 0, DynamicKind::ComponentA),
            Op::Rerender,
            Op::template(
                0,
                TemplateEdit::SetNode {
                    node: 0,
                    kind: TemplateNodeKind::Text(0),
                },
            ),
            Op::Rerender,
        ]);
    }

    #[test]
    fn stale_event_after_listener_removal_is_noop() {
        let ops = [
            Op::template(
                0,
                TemplateEdit::Attrs {
                    element: 0,
                    edit: ListEdit::Insert {
                        index: 0,
                        item: TemplateAttrSpec::Dynamic,
                    },
                },
            ),
            Op::dynamic_attrs(
                0,
                0,
                ListEdit::Insert {
                    index: 0,
                    item: AttrSpec {
                        name: 0,
                        namespace: None,
                        value: AttrValueSpec::Listener,
                        volatile: false,
                    },
                },
            ),
            Op::Rerender,
            Op::dynamic_attrs(0, 0, ListEdit::Remove { index: 0 }),
            Op::Rerender,
            Op::Rerender,
        ];

        let mut harness = Harness::fresh_strict();
        for op in ops {
            apply_op(&mut harness, &op).unwrap();
        }
        assert_eq!(
            harness
                .incremental
                .historical_event_listener_targets()
                .len(),
            1
        );
        fire_historical_event_listeners(&harness).unwrap();
    }

    #[test]
    fn stale_event_after_listener_element_removal_is_noop() {
        let ops = [
            Op::template(
                0,
                TemplateEdit::Attrs {
                    element: 0,
                    edit: ListEdit::Insert {
                        index: 0,
                        item: TemplateAttrSpec::Dynamic,
                    },
                },
            ),
            Op::dynamic_attrs(
                0,
                0,
                ListEdit::Insert {
                    index: 0,
                    item: AttrSpec {
                        name: 0,
                        namespace: None,
                        value: AttrValueSpec::Listener,
                        volatile: false,
                    },
                },
            ),
            Op::Rerender,
            Op::template(
                0,
                TemplateEdit::SetNode {
                    node: 0,
                    kind: TemplateNodeKind::Text(0),
                },
            ),
            Op::Rerender,
            Op::Rerender,
        ];

        let mut harness = Harness::fresh();
        for op in ops {
            apply_op(&mut harness, &op).unwrap();
        }
        assert_eq!(
            harness
                .incremental
                .historical_event_listener_targets()
                .len(),
            1
        );
        fire_historical_event_listeners(&harness).unwrap();
    }

    #[test]
    fn suspense_replay_does_not_duplicate_promoted_children() {
        let ops = [
            Op::template(
                0,
                TemplateEdit::SetNode {
                    node: 0,
                    kind: TemplateNodeKind::Dynamic,
                },
            ),
            Op::dynamic(
                0,
                0,
                DynamicKind::Suspense {
                    mode: SuspenseMode::Resolved,
                },
            ),
            Op::template(
                3,
                TemplateEdit::SetNode {
                    node: 0,
                    kind: TemplateNodeKind::Dynamic,
                },
            ),
            Op::dynamic(
                7,
                0,
                DynamicKind::Suspense {
                    mode: SuspenseMode::Ready,
                },
            ),
            Op::Rerender,
            Op::suspense(0, SuspenseMode::Pending),
            Op::template(
                7,
                TemplateEdit::Roots {
                    edit: ListEdit::Insert {
                        index: 0,
                        item: TemplateNodeKind::Dynamic,
                    },
                },
            ),
            Op::Rerender,
            Op::suspense(0, SuspenseMode::Resolved),
            Op::wake_suspense(0),
        ];

        let mut harness = Harness::fresh();
        for op in ops {
            apply_op(&mut harness, &op).unwrap();
        }
    }

    #[test]
    fn suspense_wake_after_parent_root_insert_does_not_duplicate_promoted_children() {
        let ops = [
            Op::template(
                0,
                TemplateEdit::SetNode {
                    node: 0,
                    kind: TemplateNodeKind::Dynamic,
                },
            ),
            Op::dynamic(
                0,
                0,
                DynamicKind::Suspense {
                    mode: SuspenseMode::Resolved,
                },
            ),
            Op::template(
                3,
                TemplateEdit::SetNode {
                    node: 0,
                    kind: TemplateNodeKind::Dynamic,
                },
            ),
            Op::dynamic(
                7,
                0,
                DynamicKind::Suspense {
                    mode: SuspenseMode::Ready,
                },
            ),
            Op::Rerender,
            Op::suspense(0, SuspenseMode::Pending),
            Op::template(
                7,
                TemplateEdit::Roots {
                    edit: ListEdit::Insert {
                        index: 0,
                        item: TemplateNodeKind::Dynamic,
                    },
                },
            ),
            Op::Rerender,
            Op::suspense(0, SuspenseMode::Resolved),
            Op::Rerender,
            Op::template(
                0,
                TemplateEdit::Roots {
                    edit: ListEdit::Insert {
                        index: 0,
                        item: TemplateNodeKind::Dynamic,
                    },
                },
            ),
            Op::wake_suspense(0),
        ];

        let mut harness = Harness::fresh();
        for op in ops {
            apply_op(&mut harness, &op).unwrap();
        }
    }

    #[test]
    fn nested_suspense_wake_after_parent_attr_and_child_edit_does_not_duplicate_children() {
        let ops = [
            Op::template(
                0,
                TemplateEdit::Roots {
                    edit: ListEdit::Insert {
                        index: 0,
                        item: TemplateNodeKind::Dynamic,
                    },
                },
            ),
            Op::dynamic(
                0,
                0,
                DynamicKind::Suspense {
                    mode: SuspenseMode::Resolved,
                },
            ),
            Op::template(
                3,
                TemplateEdit::SetNode {
                    node: 0,
                    kind: TemplateNodeKind::Dynamic,
                },
            ),
            Op::dynamic(
                7,
                0,
                DynamicKind::Suspense {
                    mode: SuspenseMode::Ready,
                },
            ),
            Op::Rerender,
            Op::suspense(0, SuspenseMode::Ready),
            Op::Rerender,
            Op::template(
                0,
                TemplateEdit::Attrs {
                    element: 0,
                    edit: ListEdit::Insert {
                        index: 0,
                        item: TemplateAttrSpec::Dynamic,
                    },
                },
            ),
            Op::wake_suspense(0),
            Op::template(
                0,
                TemplateEdit::Children {
                    element: 0,
                    edit: ListEdit::Insert {
                        index: 0,
                        item: TemplateNodeKind::Dynamic,
                    },
                },
            ),
            Op::Rerender,
            Op::wake_suspense(0),
        ];

        let mut harness = Harness::fresh();
        for op in ops {
            apply_op(&mut harness, &op).unwrap();
        }
    }

    #[test]
    fn natural_wake_unmounted_ready_suspense_is_noop() {
        let ops = [
            Op::template(
                3,
                TemplateEdit::Children {
                    element: 0,
                    edit: ListEdit::Insert {
                        index: 5,
                        item: TemplateNodeKind::Dynamic,
                    },
                },
            ),
            Op::dynamic(
                5,
                2,
                DynamicKind::Suspense {
                    mode: SuspenseMode::Ready,
                },
            ),
            Op::wake_suspense_natural(3),
        ];

        let mut harness = Harness::fresh();
        for op in ops {
            apply_op(&mut harness, &op).unwrap();
        }
    }

    #[test]
    fn natural_wake_after_unrendered_parent_edit_does_not_compare_fresh_model() {
        let ops = [
            Op::template(
                2,
                TemplateEdit::Roots {
                    edit: ListEdit::Insert {
                        index: 4,
                        item: TemplateNodeKind::Dynamic,
                    },
                },
            ),
            Op::dynamic(
                6,
                4,
                DynamicKind::Suspense {
                    mode: SuspenseMode::Ready,
                },
            ),
            Op::Rerender,
            Op::template(
                2,
                TemplateEdit::Roots {
                    edit: ListEdit::Insert {
                        index: 5,
                        item: TemplateNodeKind::Text(110),
                    },
                },
            ),
            Op::wake_suspense_natural(0),
            Op::Rerender,
        ];

        let mut harness = Harness::fresh();
        for op in ops {
            apply_op(&mut harness, &op).unwrap();
        }
    }

    #[test]
    fn natural_wake_nested_suspense_applies_hidden_wake_mutation() {
        let ops = [
            Op::template(
                0,
                TemplateEdit::SetNode {
                    node: 0,
                    kind: TemplateNodeKind::Dynamic,
                },
            ),
            Op::dynamic(
                0,
                0,
                DynamicKind::Suspense {
                    mode: SuspenseMode::Resolved,
                },
            ),
            Op::template(
                3,
                TemplateEdit::SetNode {
                    node: 0,
                    kind: TemplateNodeKind::Dynamic,
                },
            ),
            Op::dynamic(
                7,
                0,
                DynamicKind::Suspense {
                    mode: SuspenseMode::Ready,
                },
            ),
            Op::suspense_wake_mutation(1, WakeMutationSpec::PrependStaticRoot { tag: 42 }),
            Op::Rerender,
            Op::suspense(0, SuspenseMode::Ready),
            Op::Rerender,
            Op::wake_suspense_natural(1),
            Op::wake_suspense_natural(0),
        ];

        let mut harness = Harness::fresh();
        for op in ops {
            apply_op(&mut harness, &op).unwrap();
        }
    }

    #[test]
    fn nested_suspense_wake_with_prepended_root_does_not_use_cleared_mount_id() {
        let ops = [
            Op::template(
                0,
                TemplateEdit::SetNode {
                    node: 0,
                    kind: TemplateNodeKind::Dynamic,
                },
            ),
            Op::dynamic(
                0,
                0,
                DynamicKind::Suspense {
                    mode: SuspenseMode::Ready,
                },
            ),
            Op::Rerender,
            Op::template(
                1,
                TemplateEdit::SetNode {
                    node: 0,
                    kind: TemplateNodeKind::Dynamic,
                },
            ),
            Op::dynamic(
                1,
                0,
                DynamicKind::Suspense {
                    mode: SuspenseMode::Ready,
                },
            ),
            Op::wake_suspense(0),
            Op::suspense_wake_mutation(1, WakeMutationSpec::PrependStaticRoot { tag: 0 }),
            Op::Rerender,
            Op::wake_suspense(0),
        ];

        let mut harness = Harness::fresh_strict();
        for op in ops {
            apply_op(&mut harness, &op).unwrap();
        }
    }

    #[test]
    fn removing_suspended_empty_fragment_does_not_reclaim_live_fallback_id() {
        let ops = [
            Op::template(
                223,
                TemplateEdit::SetNode {
                    node: 0,
                    kind: TemplateNodeKind::Dynamic,
                },
            ),
            Op::Rerender,
            Op::dynamic(
                109,
                103,
                DynamicKind::Suspense {
                    mode: SuspenseMode::Ready,
                },
            ),
            Op::Rerender,
            Op::Rerender,
            Op::wake_suspense_natural(34),
            Op::suspense(22, SuspenseMode::Pending),
            Op::Rerender,
            Op::Rerender,
            Op::fragment(
                0,
                0,
                FragmentEdit::Children(ListEdit::Insert {
                    index: 1,
                    item: None,
                }),
            ),
            Op::Rerender,
            Op::fragment(
                0,
                0,
                FragmentEdit::Children(ListEdit::Insert {
                    index: 2,
                    item: None,
                }),
            ),
            Op::Rerender,
            Op::dynamic(0, 0, DynamicKind::Empty),
            Op::Rerender,
        ];

        let mut harness = Harness::fresh_strict();
        for op in ops {
            apply_op(&mut harness, &op).unwrap();
        }
    }

    #[test]
    fn template_hash_distinguishes_root_sibling_from_nested_child() {
        let ops = [
            Op::template(
                0,
                TemplateEdit::Roots {
                    edit: ListEdit::Insert {
                        index: 0,
                        item: TemplateNodeKind::Dynamic,
                    },
                },
            ),
            Op::template(
                0,
                TemplateEdit::Roots {
                    edit: ListEdit::Insert {
                        index: 0,
                        item: TemplateNodeKind::Dynamic,
                    },
                },
            ),
            Op::template(
                0,
                TemplateEdit::Roots {
                    edit: ListEdit::Remove { index: 0 },
                },
            ),
            Op::template(
                0,
                TemplateEdit::SetNode {
                    node: 5,
                    kind: TemplateNodeKind::Text(36),
                },
            ),
            Op::template(
                0,
                TemplateEdit::SetNode {
                    node: 0,
                    kind: TemplateNodeKind::Element {
                        tag: 0,
                        namespace: None,
                    },
                },
            ),
            Op::Rerender,
            Op::template(
                0,
                TemplateEdit::Roots {
                    edit: ListEdit::Remove { index: 1 },
                },
            ),
            Op::template(
                0,
                TemplateEdit::Children {
                    element: 0,
                    edit: ListEdit::Insert {
                        index: 0,
                        item: TemplateNodeKind::Text(36),
                    },
                },
            ),
            Op::Rerender,
        ];

        let mut harness = Harness::fresh();
        for op in ops {
            apply_op(&mut harness, &op).unwrap();
        }
    }

    #[test]
    fn dynamic_attribute_shadowing_survives_no_change_rerender() {
        let ops = [
            Op::template(
                0,
                TemplateEdit::Attrs {
                    element: 0,
                    edit: ListEdit::Insert {
                        index: 0,
                        item: TemplateAttrSpec::Dynamic,
                    },
                },
            ),
            Op::template(
                0,
                TemplateEdit::Attrs {
                    element: 0,
                    edit: ListEdit::Insert {
                        index: 0,
                        item: TemplateAttrSpec::Dynamic,
                    },
                },
            ),
            Op::dynamic_attrs(
                0,
                7,
                ListEdit::Insert {
                    index: 0,
                    item: AttrSpec {
                        name: 0,
                        namespace: None,
                        value: AttrValueSpec::Int(0),
                        volatile: false,
                    },
                },
            ),
            Op::dynamic_attrs(
                0,
                0,
                ListEdit::Insert {
                    index: 0,
                    item: AttrSpec {
                        name: 0,
                        namespace: None,
                        value: AttrValueSpec::None,
                        volatile: true,
                    },
                },
            ),
            Op::Rerender,
        ];

        let mut harness = Harness::fresh();
        for op in ops {
            apply_op(&mut harness, &op).unwrap();
        }
    }

    #[test]
    fn removing_none_dynamic_attr_restores_static_template_attr() {
        replay_ops([
            Op::template(
                0,
                TemplateEdit::Attrs {
                    element: 0,
                    edit: ListEdit::Insert {
                        index: 0,
                        item: TemplateAttrSpec::Static {
                            name: 209,
                            value: 0,
                            namespace: None,
                        },
                    },
                },
            ),
            Op::template(
                195,
                TemplateEdit::Attrs {
                    element: 0,
                    edit: ListEdit::Insert {
                        index: 0,
                        item: TemplateAttrSpec::Dynamic,
                    },
                },
            ),
            Op::dynamic_attrs(
                108,
                137,
                ListEdit::Insert {
                    index: 142,
                    item: AttrSpec {
                        name: 209,
                        namespace: None,
                        value: AttrValueSpec::None,
                        volatile: true,
                    },
                },
            ),
            Op::Rerender,
            Op::dynamic_attrs(0, 185, ListEdit::Remove { index: 2 }),
            Op::Rerender,
        ]);
    }

    #[test]
    fn dynamic_attr_namespace_change_removes_old_namespace() {
        replay_ops([
            Op::template(
                0,
                TemplateEdit::Attrs {
                    element: 0,
                    edit: ListEdit::Insert {
                        index: 0,
                        item: TemplateAttrSpec::Dynamic,
                    },
                },
            ),
            Op::dynamic_attrs(
                0,
                0,
                ListEdit::Insert {
                    index: 0,
                    item: AttrSpec {
                        name: 49,
                        namespace: None,
                        value: AttrValueSpec::Float(0),
                        volatile: false,
                    },
                },
            ),
            Op::Rerender,
            Op::dynamic_attrs(
                0,
                0,
                ListEdit::Insert {
                    index: 0,
                    item: AttrSpec {
                        name: 49,
                        namespace: Some(122),
                        value: AttrValueSpec::Text(48),
                        volatile: false,
                    },
                },
            ),
            Op::Rerender,
        ]);
    }

    #[test]
    fn later_dynamic_attr_slot_shadows_earlier_slot() {
        replay_ops([
            Op::template(
                0,
                TemplateEdit::Attrs {
                    element: 0,
                    edit: ListEdit::Insert {
                        index: 0,
                        item: TemplateAttrSpec::Dynamic,
                    },
                },
            ),
            Op::dynamic_attrs(
                0,
                0,
                ListEdit::Insert {
                    index: 0,
                    item: AttrSpec {
                        name: 0,
                        namespace: None,
                        value: AttrValueSpec::Text(50),
                        volatile: false,
                    },
                },
            ),
            Op::Rerender,
            Op::dynamic_attrs(0, 0, ListEdit::Remove { index: 0 }),
            Op::Rerender,
            Op::template(
                0,
                TemplateEdit::Attrs {
                    element: 0,
                    edit: ListEdit::Insert {
                        index: 1,
                        item: TemplateAttrSpec::Dynamic,
                    },
                },
            ),
            Op::Rerender,
            Op::dynamic_attrs(
                0,
                1,
                ListEdit::Insert {
                    index: 0,
                    item: AttrSpec {
                        name: 0,
                        namespace: None,
                        value: AttrValueSpec::Text(195),
                        volatile: false,
                    },
                },
            ),
            Op::Rerender,
            Op::dynamic_attrs(
                0,
                0,
                ListEdit::Insert {
                    index: 0,
                    item: AttrSpec {
                        name: 0,
                        namespace: None,
                        value: AttrValueSpec::Any(229),
                        volatile: true,
                    },
                },
            ),
            Op::Rerender,
        ]);
    }

    #[test]
    fn later_none_dynamic_attr_slot_shadows_earlier_slot() {
        replay_ops([
            Op::template(
                0,
                TemplateEdit::Attrs {
                    element: 0,
                    edit: ListEdit::Insert {
                        index: 0,
                        item: TemplateAttrSpec::Dynamic,
                    },
                },
            ),
            Op::template(
                0,
                TemplateEdit::Attrs {
                    element: 0,
                    edit: ListEdit::Insert {
                        index: 0,
                        item: TemplateAttrSpec::Dynamic,
                    },
                },
            ),
            Op::dynamic_attrs(
                0,
                67,
                ListEdit::Insert {
                    index: 5,
                    item: AttrSpec {
                        name: 0,
                        namespace: None,
                        value: AttrValueSpec::None,
                        volatile: false,
                    },
                },
            ),
            Op::Rerender,
            Op::dynamic_attrs(
                0,
                0,
                ListEdit::Insert {
                    index: 0,
                    item: AttrSpec {
                        name: 0,
                        namespace: None,
                        value: AttrValueSpec::Int(114),
                        volatile: false,
                    },
                },
            ),
            Op::Rerender,
        ]);
    }

    #[test]
    fn root_dynamic_suspense_then_static_text_survives_no_change_rerender() {
        let ops = [
            Op::template(
                0,
                TemplateEdit::SetNode {
                    node: 0,
                    kind: TemplateNodeKind::Dynamic,
                },
            ),
            Op::dynamic(
                206,
                3,
                DynamicKind::Suspense {
                    mode: SuspenseMode::Resolved,
                },
            ),
            Op::template(
                5,
                TemplateEdit::SetNode {
                    node: 2,
                    kind: TemplateNodeKind::Dynamic,
                },
            ),
            Op::Rerender,
            Op::template(
                0,
                TemplateEdit::SetNode {
                    node: 3,
                    kind: TemplateNodeKind::Text(0),
                },
            ),
            Op::Rerender,
        ];

        let mut harness = Harness::fresh();
        for op in ops {
            apply_op(&mut harness, &op).unwrap();
        }
    }

    #[test]
    fn nested_suspense_slot_static_child_survives_no_change_rerender() {
        let ops = [
            Op::template(
                0,
                TemplateEdit::Children {
                    element: 7,
                    edit: ListEdit::Insert {
                        index: 16,
                        item: TemplateNodeKind::Text(68),
                    },
                },
            ),
            Op::template(
                5,
                TemplateEdit::Roots {
                    edit: ListEdit::Insert {
                        index: 1,
                        item: TemplateNodeKind::Text(24),
                    },
                },
            ),
            Op::template(
                1,
                TemplateEdit::SetNode {
                    node: 143,
                    kind: TemplateNodeKind::Dynamic,
                },
            ),
            Op::template(
                3,
                TemplateEdit::Children {
                    element: 3,
                    edit: ListEdit::Insert {
                        index: 6,
                        item: TemplateNodeKind::Element {
                            tag: 66,
                            namespace: None,
                        },
                    },
                },
            ),
            Op::dynamic(
                4,
                4,
                DynamicKind::Suspense {
                    mode: SuspenseMode::Ready,
                },
            ),
            Op::template(
                7,
                TemplateEdit::SetNode {
                    node: 7,
                    kind: TemplateNodeKind::Dynamic,
                },
            ),
            Op::template(
                88,
                TemplateEdit::SetNode {
                    node: 6,
                    kind: TemplateNodeKind::Dynamic,
                },
            ),
            Op::template(
                0,
                TemplateEdit::Children {
                    element: 1,
                    edit: ListEdit::Insert {
                        index: 5,
                        item: TemplateNodeKind::Dynamic,
                    },
                },
            ),
            Op::dynamic(4, 2, DynamicKind::ComponentB),
            Op::wake_suspense(120),
            Op::dynamic(
                1,
                5,
                DynamicKind::Suspense {
                    mode: SuspenseMode::Ready,
                },
            ),
            Op::template(
                6,
                TemplateEdit::SetNode {
                    node: 7,
                    kind: TemplateNodeKind::Dynamic,
                },
            ),
            Op::wake_suspense(4),
            Op::template(
                5,
                TemplateEdit::SetNode {
                    node: 7,
                    kind: TemplateNodeKind::Element {
                        tag: 0,
                        namespace: Some(0),
                    },
                },
            ),
            Op::Rerender,
        ];

        let mut harness = Harness::fresh();
        for op in ops {
            apply_op(&mut harness, &op).unwrap();
        }
    }

    #[test]
    fn nested_suspense_wake_replaces_inner_fallback_root() {
        let ops = [
            Op::template(
                183,
                TemplateEdit::Roots {
                    edit: ListEdit::Insert {
                        index: 0,
                        item: TemplateNodeKind::Dynamic,
                    },
                },
            ),
            Op::dynamic(
                0,
                1,
                DynamicKind::Suspense {
                    mode: SuspenseMode::Pending,
                },
            ),
            Op::template(
                7,
                TemplateEdit::Roots {
                    edit: ListEdit::Insert {
                        index: 1,
                        item: TemplateNodeKind::Dynamic,
                    },
                },
            ),
            Op::suspense(4, SuspenseMode::Resolved),
            Op::dynamic(
                3,
                2,
                DynamicKind::Suspense {
                    mode: SuspenseMode::Ready,
                },
            ),
            Op::Rerender,
            Op::suspense(0, SuspenseMode::Ready),
            Op::Rerender,
            Op::suspense(1, SuspenseMode::Resolved),
            Op::wake_suspense(2),
        ];

        let mut harness = Harness::fresh();
        for op in ops {
            apply_op(&mut harness, &op).unwrap();
        }
    }

    #[test]
    fn nested_ready_wake_while_parent_enters_suspense_keeps_renderer_stack_balanced() {
        let ops = [
            Op::template(
                0,
                TemplateEdit::Roots {
                    edit: ListEdit::Insert {
                        index: 68,
                        item: TemplateNodeKind::Text(94),
                    },
                },
            ),
            Op::template(
                50,
                TemplateEdit::SetNode {
                    node: 189,
                    kind: TemplateNodeKind::Dynamic,
                },
            ),
            Op::dynamic(
                0,
                0,
                DynamicKind::Suspense {
                    mode: SuspenseMode::Ready,
                },
            ),
            Op::template(
                1,
                TemplateEdit::SetNode {
                    node: 0,
                    kind: TemplateNodeKind::Dynamic,
                },
            ),
            Op::dynamic(
                15,
                170,
                DynamicKind::Suspense {
                    mode: SuspenseMode::Resolved,
                },
            ),
            Op::template(
                2,
                TemplateEdit::Roots {
                    edit: ListEdit::Insert {
                        index: 0,
                        item: TemplateNodeKind::Dynamic,
                    },
                },
            ),
            Op::wake_suspense(6),
            Op::dynamic(2, 0, DynamicKind::ComponentB),
            Op::Rerender,
            Op::template(
                2,
                TemplateEdit::Roots {
                    edit: ListEdit::Remove { index: 97 },
                },
            ),
            Op::suspense(31, SuspenseMode::Ready),
            Op::Rerender,
            Op::suspense(240, SuspenseMode::Ready),
            Op::wake_suspense(197),
        ];

        let mut harness = Harness::fresh_strict();
        for op in ops {
            apply_op(&mut harness, &op).unwrap();
        }
    }

    #[test]
    fn keyed_fragment_moves_nested_child_after_component_insert() {
        let ops = [
            Op::template(
                0,
                TemplateEdit::SetNode {
                    node: 0,
                    kind: TemplateNodeKind::Dynamic,
                },
            ),
            Op::fragment(
                0,
                0,
                FragmentEdit::Children(ListEdit::Insert {
                    index: 0,
                    item: None,
                }),
            ),
            Op::fragment(
                0,
                0,
                FragmentEdit::Children(ListEdit::Insert {
                    index: 0,
                    item: None,
                }),
            ),
            Op::fragment(
                0,
                0,
                FragmentEdit::Children(ListEdit::Insert {
                    index: 0,
                    item: None,
                }),
            ),
            Op::fragment(
                0,
                0,
                FragmentEdit::KeyMode(FragmentKeyMode::Keyed { base: 0 }),
            ),
            Op::fragment(
                0,
                0,
                FragmentEdit::Children(ListEdit::Insert {
                    index: 0,
                    item: None,
                }),
            ),
            Op::template(
                6,
                TemplateEdit::SetNode {
                    node: 0,
                    kind: TemplateNodeKind::Dynamic,
                },
            ),
            Op::template(
                7,
                TemplateEdit::Children {
                    element: 0,
                    edit: ListEdit::Insert {
                        index: 0,
                        item: TemplateNodeKind::Dynamic,
                    },
                },
            ),
            Op::fragment(
                0,
                0,
                FragmentEdit::Children(ListEdit::Insert {
                    index: 0,
                    item: None,
                }),
            ),
            Op::fragment(
                177,
                0,
                FragmentEdit::Children(ListEdit::Insert {
                    index: 0,
                    item: None,
                }),
            ),
            Op::Rerender,
            Op::dynamic(2, 0, DynamicKind::ComponentA),
            Op::fragment(
                0,
                0,
                FragmentEdit::Children(ListEdit::Move { from: 3, to: 2 }),
            ),
            Op::Rerender,
        ];

        let mut harness = Harness::fresh();
        for op in ops {
            apply_op(&mut harness, &op).unwrap();
        }
    }

    #[test]
    fn keyed_fragment_remove_after_anchor_only_child_move_keeps_parent_links() {
        let ops = [
            Op::template(
                0,
                TemplateEdit::SetNode {
                    node: 0,
                    kind: TemplateNodeKind::Dynamic,
                },
            ),
            Op::fragment(
                0,
                0,
                FragmentEdit::Children(ListEdit::Insert {
                    index: 0,
                    item: None,
                }),
            ),
            Op::fragment(
                0,
                0,
                FragmentEdit::Children(ListEdit::Insert {
                    index: 0,
                    item: None,
                }),
            ),
            Op::fragment(
                0,
                0,
                FragmentEdit::Children(ListEdit::Insert {
                    index: 0,
                    item: None,
                }),
            ),
            Op::fragment(
                0,
                0,
                FragmentEdit::KeyMode(FragmentKeyMode::Keyed { base: 0 }),
            ),
            Op::template(
                6,
                TemplateEdit::SetNode {
                    node: 0,
                    kind: TemplateNodeKind::Dynamic,
                },
            ),
            Op::fragment(
                0,
                0,
                FragmentEdit::Children(ListEdit::Insert {
                    index: 0,
                    item: None,
                }),
            ),
            Op::Rerender,
            Op::fragment(
                0,
                0,
                FragmentEdit::Children(ListEdit::Move { from: 3, to: 2 }),
            ),
            Op::fragment(0, 0, FragmentEdit::Children(ListEdit::Remove { index: 0 })),
            Op::Rerender,
        ];

        let mut harness = Harness::fresh();
        for op in ops {
            apply_op(&mut harness, &op).unwrap();
        }
    }
}
