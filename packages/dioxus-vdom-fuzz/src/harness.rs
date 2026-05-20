use crate::{
    model::*,
    ops::{
        Op, apply_to_model, clear_suspense_ready_tasks, read_model, release_suspense_ready_task,
        selected_registered_ready_suspense_key, with_model, without_suspense_ready_registration,
        TemplateEdit,
    },
    vdom::App,
};
use dioxus_core::{
    AttributeValue, ElementId, Event, ScopeId, Template, VirtualDom, WriteMutations,
};
use dioxus_renderer_oracle::{RendererOracle, SnapshotNode, panic_message};
use std::{any::Any, panic, rc::Rc, sync::Mutex};

// ---------- Harness -------------------------------------------------------------------------

type TargetSnapshots = Vec<SnapshotNode>;

pub(crate) struct Harness {
    vdom: VirtualDom,
    incremental: TargetedRendererOracle,
    pending_app_render: bool,
    pending_fresh_compare: bool,
    strict_renderer_errors: bool,
}

impl Harness {
    pub(crate) fn fresh() -> Self {
        Self::fresh_with_strict_renderer_errors(cfg!(fuzzing))
    }

    #[cfg(test)]
    fn fresh_strict() -> Self {
        Self::fresh_with_strict_renderer_errors(true)
    }

    fn fresh_with_strict_renderer_errors(strict_renderer_errors: bool) -> Self {
        clear_suspense_ready_tasks();
        with_model(|model| *model = Model::initial());
        let mut vdom = VirtualDom::new(App);
        let mut incremental = TargetedRendererOracle::new();
        vdom.rebuild(&mut incremental);
        incremental.assert_stack_clean();
        Self {
            vdom,
            incremental,
            pending_app_render: false,
            pending_fresh_compare: false,
            strict_renderer_errors,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
struct TargetedEventListenerTarget {
    name: &'static str,
    id: ElementId,
}

struct TargetedRendererOracle {
    renderer: RendererOracle,
    last_mutation: Option<String>,
    recent_mutations: Vec<String>,
}

impl TargetedRendererOracle {
    fn new() -> Self {
        Self {
            renderer: RendererOracle::new(),
            last_mutation: None,
            recent_mutations: Vec::new(),
        }
    }

    fn current_renderer(&mut self) -> &mut RendererOracle {
        &mut self.renderer
    }

    fn record_mutation(&mut self, mutation: impl Into<String>) {
        let mutation = mutation.into();
        self.last_mutation = Some(mutation.clone());
        self.recent_mutations.push(mutation);
        if self.recent_mutations.len() > 16 {
            self.recent_mutations.remove(0);
        }
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
        let fresh_snapshot = fresh.snapshot();
        let incremental_snapshot = self.snapshot();
        if incremental_snapshot == fresh_snapshot {
            return Ok(());
        }

        Err(format!(
            "incremental renderer snapshot does not match fresh render\nincremental:\n{incremental_snapshot:#?}\nfresh:\n{fresh_snapshot:#?}"
        ))
    }

    fn snapshot(&self) -> TargetSnapshots {
        self.renderer.snapshot()
    }

    fn historical_event_listener_targets(&self) -> Vec<TargetedEventListenerTarget> {
        self.renderer
            .historical_event_listener_targets()
            .iter()
            .map(|listener| TargetedEventListenerTarget {
                name: listener.name,
                id: listener.id,
            })
            .collect()
    }
}

impl WriteMutations for TargetedRendererOracle {
    fn append_children(&mut self, id: ElementId, m: usize) {
        self.record_mutation(format!("append_children(id: {id:?}, m: {m})"));
        self.current_renderer().append_children(id, m)
    }

    fn assign_node_id(&mut self, path: &'static [u8], id: ElementId) {
        self.record_mutation(format!("assign_node_id(path: {path:?}, id: {id:?})"));
        self.current_renderer().assign_node_id(path, id)
    }

    fn create_placeholder(&mut self, id: ElementId) {
        self.record_mutation(format!("create_placeholder(id: {id:?})"));
        self.current_renderer().create_placeholder(id)
    }

    fn create_text_node(&mut self, value: &str, id: ElementId) {
        self.record_mutation(format!("create_text_node(value: {value:?}, id: {id:?})"));
        self.current_renderer().create_text_node(value, id)
    }

    fn load_template(&mut self, template: Template, index: usize, id: ElementId) {
        self.record_mutation(format!("load_template(index: {index}, id: {id:?})"));
        self.current_renderer().load_template(template, index, id)
    }

    fn replace_node_with(&mut self, id: ElementId, m: usize) {
        self.record_mutation(format!("replace_node_with(id: {id:?}, m: {m})"));
        self.current_renderer().replace_node_with(id, m)
    }

    fn replace_placeholder_with_nodes(&mut self, path: &'static [u8], m: usize) {
        self.record_mutation(format!(
            "replace_placeholder_with_nodes(path: {path:?}, m: {m})"
        ));
        self.current_renderer()
            .replace_placeholder_with_nodes(path, m)
    }

    fn insert_nodes_after(&mut self, id: ElementId, m: usize) {
        self.record_mutation(format!("insert_nodes_after(id: {id:?}, m: {m})"));
        self.current_renderer().insert_nodes_after(id, m)
    }

    fn insert_nodes_before(&mut self, id: ElementId, m: usize) {
        self.record_mutation(format!("insert_nodes_before(id: {id:?}, m: {m})"));
        self.current_renderer().insert_nodes_before(id, m)
    }

    fn set_attribute(
        &mut self,
        name: &'static str,
        ns: Option<&'static str>,
        value: &AttributeValue,
        id: ElementId,
    ) {
        self.record_mutation(format!("set_attribute(name: {name:?}, id: {id:?})"));
        self.current_renderer().set_attribute(name, ns, value, id)
    }

    fn set_node_text(&mut self, value: &str, id: ElementId) {
        self.record_mutation(format!("set_node_text(value: {value:?}, id: {id:?})"));
        self.current_renderer().set_node_text(value, id)
    }

    fn create_event_listener(&mut self, name: &'static str, id: ElementId) {
        self.record_mutation(format!("create_event_listener(name: {name:?}, id: {id:?})"));
        self.current_renderer().create_event_listener(name, id)
    }

    fn remove_event_listener(&mut self, name: &'static str, id: ElementId) {
        self.record_mutation(format!("remove_event_listener(name: {name:?}, id: {id:?})"));
        self.current_renderer().remove_event_listener(name, id)
    }

    fn remove_node(&mut self, id: ElementId) {
        self.record_mutation(format!("remove_node(id: {id:?})"));
        self.current_renderer().remove_node(id)
    }

    fn push_root(&mut self, id: ElementId) {
        self.record_mutation(format!("push_root(id: {id:?})"));
        self.current_renderer().push_root(id)
    }
}

const TRACE_CONTEXT: usize = 6;
const MAX_HTML_CHARS: usize = 240;
static PANIC_HOOK_LOCK: Mutex<()> = Mutex::new(());

fn catch_unwind_silent<F, R>(f: F) -> std::thread::Result<R>
where
    F: FnOnce() -> R,
{
    let _lock = PANIC_HOOK_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let previous_hook = panic::take_hook();
    panic::set_hook(Box::new(|_| {}));
    let result = panic::catch_unwind(panic::AssertUnwindSafe(f));
    panic::set_hook(previous_hook);
    result
}

fn render_model_with_ssr(model: &Model) -> Result<String, String> {
    catch_unwind_silent(|| {
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

fn print_op_window(ops: &[Op], failing_step: usize) {
    let (start, end) = trace_bounds(ops.len(), failing_step);

    println!("operation window:");
    if start > 0 {
        println!("  ... {} earlier ops omitted", start);
    }
    for (index, op) in ops.iter().enumerate().take(end).skip(start) {
        let marker = if index == failing_step { ">>" } else { "  " };
        println!("{marker} {index:03}: {op:?}");
    }
    if end < ops.len() {
        println!("  ... {} later ops omitted", ops.len() - end);
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
    print_op_window(ops, failing_step);
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
        Op::WakeSuspense { suspense } => {
            let Some(key) = read_model().selected_ready_suspense_key(*suspense) else {
                return Ok(());
            };
            apply_to_model(op);
            release_suspense_ready_task(key);
            render_and_assert(state)
        }
        Op::WakeSuspenseNatural { suspense } => {
            let Some(key) = selected_registered_ready_suspense_key(*suspense) else {
                return Ok(());
            };
            with_model(|model| model.resolve_ready_suspense(key));
            release_suspense_ready_task(key);
            let compare_fresh = !state.pending_app_render;
            render_natural_and_assert(state, compare_fresh)
        }
        _ => {
            apply_to_model(op);
            if op_requires_app_render(op) {
                state.pending_app_render = true;
            }
            if op_requires_fresh_compare(op) {
                state.pending_fresh_compare = true;
            }
            Ok(())
        }
    }
}

fn op_requires_app_render(op: &Op) -> bool {
    matches!(
        op,
        Op::Template { .. }
            | Op::Dynamic { .. }
            | Op::DynamicAttrs { .. }
            | Op::Fragment { .. }
            | Op::Suspense { .. }
    )
}

fn op_requires_fresh_compare(op: &Op) -> bool {
    matches!(
        op,
        Op::Template {
            edit: TemplateEdit::Generated { .. },
            ..
        }
    )
}

fn fire_historical_event_listeners(state: &Harness) -> Result<(), String> {
    let targets = state.incremental.historical_event_listener_targets();
    let runtime = state.vdom.runtime();
    let result = catch_unwind_silent(|| {
        for target in targets {
            let event = Event::new(
                Rc::new(String::from("fuzzer stale event")) as Rc<dyn Any>,
                true,
            );
            runtime.handle_event(target.name, event, target.id);
        }
    });

    match result {
        Ok(()) => Ok(()),
        Err(payload) => Err(format!(
            "panic while firing historical event listeners: {}",
            panic_message(&payload)
        )),
    }
}

fn render_once(
    state: &mut Harness,
    mark_app_dirty: bool,
    assert_matches_vdom: bool,
    label: &'static str,
) -> Result<TargetSnapshots, String> {
    fire_historical_event_listeners(state)?;
    if mark_app_dirty {
        state.vdom.mark_dirty(ScopeId::APP);
    }
    let render_result = catch_unwind_silent(|| {
        state.vdom.render_immediate(&mut state.incremental);
        state.incremental.check_stack_clean().map_err(|err| {
            let last_mutation = state
                .incremental
                .last_mutation
                .as_deref()
                .unwrap_or("<none>");
            format!(
                "{err} after {last_mutation}\nrecent mutations:\n  {}",
                state.incremental.recent_mutations.join("\n  ")
            )
        })?;
        let snap = state.incremental.snapshot();
        if assert_matches_vdom {
            state.incremental.check_matches_vdom(&state.vdom)?;
        }
        Ok(snap)
    });

    match render_result {
        Ok(result) => result,
        Err(payload) => {
            let last_mutation = state
                .incremental
                .last_mutation
                .as_deref()
                .unwrap_or("<none>");
            Err(format!(
                "panic in {label} after {last_mutation}: {}",
                panic_message(&payload),
            ))
        }
    }
}

fn render_and_assert(state: &mut Harness) -> Result<(), String> {
    let compare_fresh = state.pending_fresh_compare;
    let result = render_once(state, true, compare_fresh, "incremental render");
    state.pending_app_render = false;
    state.pending_fresh_compare = false;
    render_result_to_fuzz_failure(state, result)
}

fn render_natural_and_assert(state: &mut Harness, compare_fresh: bool) -> Result<(), String> {
    let result = render_once(
        state,
        false,
        compare_fresh && state.pending_fresh_compare,
        "natural incremental render",
    );
    if compare_fresh {
        state.pending_fresh_compare = false;
    }
    render_result_to_fuzz_failure(state, result)
}

fn render_result_to_fuzz_failure(
    state: &Harness,
    result: Result<TargetSnapshots, String>,
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
        ops::{FragmentEdit, IteratorScenario, ListEdit, TemplateEdit, iterator_scenario_ops},
    };

    fn replay_ops(ops: impl IntoIterator<Item = Op>) {
        let mut harness = Harness::fresh_strict();
        for op in ops {
            apply_op(&mut harness, &op).unwrap();
        }
    }

    #[test]
    fn large_template_hash_stress_replay() {
        replay_ops(iterator_scenario_ops(
            IteratorScenario::LargeTemplateHashStress,
            0,
        ));
    }

    #[test]
    fn replacing_root_portal_with_fragment_removes_old_target_subtree() {
        replay_ops([
            Op::Template {
                vnode: 0,
                edit: TemplateEdit::SetNode {
                    node: 0,
                    kind: TemplateNodeKind::Dynamic,
                },
            },
            Op::Dynamic {
                vnode: 0,
                slot: 0,
                kind: DynamicKind::Portal {
                    target: PortalTargetSpec::TargetA,
                },
            },
            Op::Rerender,
            Op::Fragment {
                vnode: 0,
                slot: 0,
                edit: FragmentEdit::Children(ListEdit::Insert {
                    index: 0,
                    item: None,
                }),
            },
            Op::Rerender,
        ]);
    }

    #[test]
    fn keyed_fragment_move_with_noop_portal_child_skips_placeholder_root() {
        replay_ops([
            Op::Template {
                vnode: 0,
                edit: TemplateEdit::SetNode {
                    node: 0,
                    kind: TemplateNodeKind::Dynamic,
                },
            },
            Op::Fragment {
                vnode: 0,
                slot: 0,
                edit: FragmentEdit::Children(ListEdit::Insert {
                    index: 0,
                    item: None,
                }),
            },
            Op::Template {
                vnode: 1,
                edit: TemplateEdit::SetNode {
                    node: 0,
                    kind: TemplateNodeKind::Dynamic,
                },
            },
            Op::Dynamic {
                vnode: 1,
                slot: 0,
                kind: DynamicKind::Portal {
                    target: PortalTargetSpec::Noop,
                },
            },
            Op::Fragment {
                vnode: 0,
                slot: 0,
                edit: FragmentEdit::KeyMode(FragmentKeyMode::Keyed { base: 0 }),
            },
            Op::Fragment {
                vnode: 0,
                slot: 0,
                edit: FragmentEdit::Children(ListEdit::Insert {
                    index: 0,
                    item: None,
                }),
            },
            Op::Rerender,
            Op::Fragment {
                vnode: 0,
                slot: 0,
                edit: FragmentEdit::Children(ListEdit::Move { from: 1, to: 0 }),
            },
            Op::Rerender,
        ]);
    }

    #[test]
    fn domless_root_fragment_child_materializes_before_sibling() {
        replay_ops([
            Op::Template {
                vnode: 0,
                edit: TemplateEdit::SetNode {
                    node: 0,
                    kind: TemplateNodeKind::Dynamic,
                },
            },
            Op::Fragment {
                vnode: 0,
                slot: 0,
                edit: FragmentEdit::Children(ListEdit::Insert {
                    index: 0,
                    item: None,
                }),
            },
            Op::Fragment {
                vnode: 0,
                slot: 0,
                edit: FragmentEdit::Children(ListEdit::Insert {
                    index: 0,
                    item: None,
                }),
            },
            Op::Template {
                vnode: 1,
                edit: TemplateEdit::SetNode {
                    node: 0,
                    kind: TemplateNodeKind::Dynamic,
                },
            },
            Op::Rerender,
            Op::Dynamic {
                vnode: 1,
                slot: 0,
                kind: DynamicKind::Text(0),
            },
            Op::Rerender,
        ]);
    }

    #[test]
    fn replacing_root_portal_with_static_text_uses_root_anchor() {
        replay_ops([
            Op::Template {
                vnode: 0,
                edit: TemplateEdit::SetNode {
                    node: 0,
                    kind: TemplateNodeKind::Dynamic,
                },
            },
            Op::Dynamic {
                vnode: 0,
                slot: 0,
                kind: DynamicKind::Portal {
                    target: PortalTargetSpec::TargetA,
                },
            },
            Op::Rerender,
            Op::Template {
                vnode: 0,
                edit: TemplateEdit::SetNode {
                    node: 0,
                    kind: TemplateNodeKind::Text(0),
                },
            },
            Op::Rerender,
        ]);
    }

    #[test]
    fn stale_event_after_listener_removal_is_noop() {
        let ops = [
            Op::Template {
                vnode: 0,
                edit: TemplateEdit::Attrs {
                    element: 0,
                    edit: ListEdit::Insert {
                        index: 0,
                        item: TemplateAttrSpec::Dynamic,
                    },
                },
            },
            Op::DynamicAttrs {
                vnode: 0,
                slot: 0,
                edit: ListEdit::Insert {
                    index: 0,
                    item: AttrSpec {
                        name: 0,
                        namespace: None,
                        value: AttrValueSpec::Listener,
                        volatile: false,
                    },
                },
            },
            Op::Rerender,
            Op::DynamicAttrs {
                vnode: 0,
                slot: 0,
                edit: ListEdit::Remove { index: 0 },
            },
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
            Op::Template {
                vnode: 0,
                edit: TemplateEdit::Attrs {
                    element: 0,
                    edit: ListEdit::Insert {
                        index: 0,
                        item: TemplateAttrSpec::Dynamic,
                    },
                },
            },
            Op::DynamicAttrs {
                vnode: 0,
                slot: 0,
                edit: ListEdit::Insert {
                    index: 0,
                    item: AttrSpec {
                        name: 0,
                        namespace: None,
                        value: AttrValueSpec::Listener,
                        volatile: false,
                    },
                },
            },
            Op::Rerender,
            Op::Template {
                vnode: 0,
                edit: TemplateEdit::SetNode {
                    node: 0,
                    kind: TemplateNodeKind::Text(0),
                },
            },
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
            Op::Template {
                vnode: 0,
                edit: TemplateEdit::SetNode {
                    node: 0,
                    kind: TemplateNodeKind::Dynamic,
                },
            },
            Op::Dynamic {
                vnode: 0,
                slot: 0,
                kind: DynamicKind::Suspense {
                    mode: SuspenseMode::Resolved,
                },
            },
            Op::Template {
                vnode: 3,
                edit: TemplateEdit::SetNode {
                    node: 0,
                    kind: TemplateNodeKind::Dynamic,
                },
            },
            Op::Dynamic {
                vnode: 7,
                slot: 0,
                kind: DynamicKind::Suspense {
                    mode: SuspenseMode::Ready,
                },
            },
            Op::Rerender,
            Op::Suspense {
                suspense: 0,
                mode: SuspenseMode::Pending,
            },
            Op::Template {
                vnode: 7,
                edit: TemplateEdit::Roots {
                    edit: ListEdit::Insert {
                        index: 0,
                        item: TemplateNodeKind::Dynamic,
                    },
                },
            },
            Op::Rerender,
            Op::Suspense {
                suspense: 0,
                mode: SuspenseMode::Resolved,
            },
            Op::WakeSuspense { suspense: 0 },
        ];

        let mut harness = Harness::fresh();
        for op in ops {
            apply_op(&mut harness, &op).unwrap();
        }
    }

    #[test]
    fn suspense_wake_after_parent_root_insert_does_not_duplicate_promoted_children() {
        let ops = [
            Op::Template {
                vnode: 0,
                edit: TemplateEdit::SetNode {
                    node: 0,
                    kind: TemplateNodeKind::Dynamic,
                },
            },
            Op::Dynamic {
                vnode: 0,
                slot: 0,
                kind: DynamicKind::Suspense {
                    mode: SuspenseMode::Resolved,
                },
            },
            Op::Template {
                vnode: 3,
                edit: TemplateEdit::SetNode {
                    node: 0,
                    kind: TemplateNodeKind::Dynamic,
                },
            },
            Op::Dynamic {
                vnode: 7,
                slot: 0,
                kind: DynamicKind::Suspense {
                    mode: SuspenseMode::Ready,
                },
            },
            Op::Rerender,
            Op::Suspense {
                suspense: 0,
                mode: SuspenseMode::Pending,
            },
            Op::Template {
                vnode: 7,
                edit: TemplateEdit::Roots {
                    edit: ListEdit::Insert {
                        index: 0,
                        item: TemplateNodeKind::Dynamic,
                    },
                },
            },
            Op::Rerender,
            Op::Suspense {
                suspense: 0,
                mode: SuspenseMode::Resolved,
            },
            Op::Rerender,
            Op::Template {
                vnode: 0,
                edit: TemplateEdit::Roots {
                    edit: ListEdit::Insert {
                        index: 0,
                        item: TemplateNodeKind::Dynamic,
                    },
                },
            },
            Op::WakeSuspense { suspense: 0 },
        ];

        let mut harness = Harness::fresh();
        for op in ops {
            apply_op(&mut harness, &op).unwrap();
        }
    }

    #[test]
    fn nested_suspense_wake_after_parent_attr_and_child_edit_does_not_duplicate_children() {
        let ops = [
            Op::Template {
                vnode: 0,
                edit: TemplateEdit::Roots {
                    edit: ListEdit::Insert {
                        index: 0,
                        item: TemplateNodeKind::Dynamic,
                    },
                },
            },
            Op::Dynamic {
                vnode: 0,
                slot: 0,
                kind: DynamicKind::Suspense {
                    mode: SuspenseMode::Resolved,
                },
            },
            Op::Template {
                vnode: 3,
                edit: TemplateEdit::SetNode {
                    node: 0,
                    kind: TemplateNodeKind::Dynamic,
                },
            },
            Op::Dynamic {
                vnode: 7,
                slot: 0,
                kind: DynamicKind::Suspense {
                    mode: SuspenseMode::Ready,
                },
            },
            Op::Rerender,
            Op::Suspense {
                suspense: 0,
                mode: SuspenseMode::Ready,
            },
            Op::Rerender,
            Op::Template {
                vnode: 0,
                edit: TemplateEdit::Attrs {
                    element: 0,
                    edit: ListEdit::Insert {
                        index: 0,
                        item: TemplateAttrSpec::Dynamic,
                    },
                },
            },
            Op::WakeSuspense { suspense: 0 },
            Op::Template {
                vnode: 0,
                edit: TemplateEdit::Children {
                    element: 0,
                    edit: ListEdit::Insert {
                        index: 0,
                        item: TemplateNodeKind::Dynamic,
                    },
                },
            },
            Op::Rerender,
            Op::WakeSuspense { suspense: 0 },
        ];

        let mut harness = Harness::fresh();
        for op in ops {
            apply_op(&mut harness, &op).unwrap();
        }
    }

    #[test]
    fn natural_wake_unmounted_ready_suspense_is_noop() {
        let ops = [
            Op::Template {
                vnode: 3,
                edit: TemplateEdit::Children {
                    element: 0,
                    edit: ListEdit::Insert {
                        index: 5,
                        item: TemplateNodeKind::Dynamic,
                    },
                },
            },
            Op::Dynamic {
                vnode: 5,
                slot: 2,
                kind: DynamicKind::Suspense {
                    mode: SuspenseMode::Ready,
                },
            },
            Op::WakeSuspenseNatural { suspense: 3 },
        ];

        let mut harness = Harness::fresh();
        for op in ops {
            apply_op(&mut harness, &op).unwrap();
        }
    }

    #[test]
    fn natural_wake_after_unrendered_parent_edit_does_not_compare_fresh_model() {
        let ops = [
            Op::Template {
                vnode: 2,
                edit: TemplateEdit::Roots {
                    edit: ListEdit::Insert {
                        index: 4,
                        item: TemplateNodeKind::Dynamic,
                    },
                },
            },
            Op::Dynamic {
                vnode: 6,
                slot: 4,
                kind: DynamicKind::Suspense {
                    mode: SuspenseMode::Ready,
                },
            },
            Op::Rerender,
            Op::Template {
                vnode: 2,
                edit: TemplateEdit::Roots {
                    edit: ListEdit::Insert {
                        index: 5,
                        item: TemplateNodeKind::Text(110),
                    },
                },
            },
            Op::WakeSuspenseNatural { suspense: 0 },
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
            Op::Template {
                vnode: 0,
                edit: TemplateEdit::SetNode {
                    node: 0,
                    kind: TemplateNodeKind::Dynamic,
                },
            },
            Op::Dynamic {
                vnode: 0,
                slot: 0,
                kind: DynamicKind::Suspense {
                    mode: SuspenseMode::Resolved,
                },
            },
            Op::Template {
                vnode: 3,
                edit: TemplateEdit::SetNode {
                    node: 0,
                    kind: TemplateNodeKind::Dynamic,
                },
            },
            Op::Dynamic {
                vnode: 7,
                slot: 0,
                kind: DynamicKind::Suspense {
                    mode: SuspenseMode::Ready,
                },
            },
            Op::SuspenseWakeMutation {
                suspense: 1,
                mutation: WakeMutationSpec::PrependStaticRoot { tag: 42 },
            },
            Op::Rerender,
            Op::Suspense {
                suspense: 0,
                mode: SuspenseMode::Ready,
            },
            Op::Rerender,
            Op::WakeSuspenseNatural { suspense: 1 },
            Op::WakeSuspenseNatural { suspense: 0 },
        ];

        let mut harness = Harness::fresh();
        for op in ops {
            apply_op(&mut harness, &op).unwrap();
        }
    }

    #[test]
    fn nested_suspense_wake_with_prepended_root_does_not_use_cleared_mount_id() {
        let ops = [
            Op::Template {
                vnode: 0,
                edit: TemplateEdit::SetNode {
                    node: 0,
                    kind: TemplateNodeKind::Dynamic,
                },
            },
            Op::Dynamic {
                vnode: 0,
                slot: 0,
                kind: DynamicKind::Suspense {
                    mode: SuspenseMode::Ready,
                },
            },
            Op::Rerender,
            Op::Template {
                vnode: 1,
                edit: TemplateEdit::SetNode {
                    node: 0,
                    kind: TemplateNodeKind::Dynamic,
                },
            },
            Op::Dynamic {
                vnode: 1,
                slot: 0,
                kind: DynamicKind::Suspense {
                    mode: SuspenseMode::Ready,
                },
            },
            Op::WakeSuspense { suspense: 0 },
            Op::SuspenseWakeMutation {
                suspense: 1,
                mutation: WakeMutationSpec::PrependStaticRoot { tag: 0 },
            },
            Op::Rerender,
            Op::WakeSuspense { suspense: 0 },
        ];

        let mut harness = Harness::fresh_strict();
        for op in ops {
            apply_op(&mut harness, &op).unwrap();
        }
    }

    #[test]
    fn removing_suspended_empty_fragment_does_not_reclaim_live_fallback_id() {
        let ops = [
            Op::Template {
                vnode: 223,
                edit: TemplateEdit::SetNode {
                    node: 0,
                    kind: TemplateNodeKind::Dynamic,
                },
            },
            Op::Rerender,
            Op::Dynamic {
                vnode: 109,
                slot: 103,
                kind: DynamicKind::Suspense {
                    mode: SuspenseMode::Ready,
                },
            },
            Op::Rerender,
            Op::Rerender,
            Op::WakeSuspenseNatural { suspense: 34 },
            Op::Suspense {
                suspense: 22,
                mode: SuspenseMode::Pending,
            },
            Op::Rerender,
            Op::Rerender,
            Op::Fragment {
                vnode: 0,
                slot: 0,
                edit: FragmentEdit::Children(ListEdit::Insert {
                    index: 1,
                    item: None,
                }),
            },
            Op::Rerender,
            Op::Fragment {
                vnode: 0,
                slot: 0,
                edit: FragmentEdit::Children(ListEdit::Insert {
                    index: 2,
                    item: None,
                }),
            },
            Op::Rerender,
            Op::Dynamic {
                vnode: 0,
                slot: 0,
                kind: DynamicKind::Empty,
            },
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
            Op::Template {
                vnode: 0,
                edit: TemplateEdit::Roots {
                    edit: ListEdit::Insert {
                        index: 0,
                        item: TemplateNodeKind::Dynamic,
                    },
                },
            },
            Op::Template {
                vnode: 0,
                edit: TemplateEdit::Roots {
                    edit: ListEdit::Insert {
                        index: 0,
                        item: TemplateNodeKind::Dynamic,
                    },
                },
            },
            Op::Template {
                vnode: 0,
                edit: TemplateEdit::Roots {
                    edit: ListEdit::Remove { index: 0 },
                },
            },
            Op::Template {
                vnode: 0,
                edit: TemplateEdit::SetNode {
                    node: 5,
                    kind: TemplateNodeKind::Text(36),
                },
            },
            Op::Template {
                vnode: 0,
                edit: TemplateEdit::SetNode {
                    node: 0,
                    kind: TemplateNodeKind::Element {
                        tag: 0,
                        namespace: None,
                    },
                },
            },
            Op::Rerender,
            Op::Template {
                vnode: 0,
                edit: TemplateEdit::Roots {
                    edit: ListEdit::Remove { index: 1 },
                },
            },
            Op::Template {
                vnode: 0,
                edit: TemplateEdit::Children {
                    element: 0,
                    edit: ListEdit::Insert {
                        index: 0,
                        item: TemplateNodeKind::Text(36),
                    },
                },
            },
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
            Op::Template {
                vnode: 0,
                edit: TemplateEdit::Attrs {
                    element: 0,
                    edit: ListEdit::Insert {
                        index: 0,
                        item: TemplateAttrSpec::Dynamic,
                    },
                },
            },
            Op::Template {
                vnode: 0,
                edit: TemplateEdit::Attrs {
                    element: 0,
                    edit: ListEdit::Insert {
                        index: 0,
                        item: TemplateAttrSpec::Dynamic,
                    },
                },
            },
            Op::DynamicAttrs {
                vnode: 0,
                slot: 7,
                edit: ListEdit::Insert {
                    index: 0,
                    item: AttrSpec {
                        name: 0,
                        namespace: None,
                        value: AttrValueSpec::Int(0),
                        volatile: false,
                    },
                },
            },
            Op::DynamicAttrs {
                vnode: 0,
                slot: 0,
                edit: ListEdit::Insert {
                    index: 0,
                    item: AttrSpec {
                        name: 0,
                        namespace: None,
                        value: AttrValueSpec::None,
                        volatile: true,
                    },
                },
            },
            Op::Rerender,
        ];

        let mut harness = Harness::fresh();
        for op in ops {
            apply_op(&mut harness, &op).unwrap();
        }
    }

    #[test]
    fn root_dynamic_suspense_then_static_text_survives_no_change_rerender() {
        let ops = [
            Op::Template {
                vnode: 0,
                edit: TemplateEdit::SetNode {
                    node: 0,
                    kind: TemplateNodeKind::Dynamic,
                },
            },
            Op::Dynamic {
                vnode: 206,
                slot: 3,
                kind: DynamicKind::Suspense {
                    mode: SuspenseMode::Resolved,
                },
            },
            Op::Template {
                vnode: 5,
                edit: TemplateEdit::SetNode {
                    node: 2,
                    kind: TemplateNodeKind::Dynamic,
                },
            },
            Op::Rerender,
            Op::Template {
                vnode: 0,
                edit: TemplateEdit::SetNode {
                    node: 3,
                    kind: TemplateNodeKind::Text(0),
                },
            },
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
            Op::Template {
                vnode: 0,
                edit: TemplateEdit::Children {
                    element: 7,
                    edit: ListEdit::Insert {
                        index: 16,
                        item: TemplateNodeKind::Text(68),
                    },
                },
            },
            Op::Template {
                vnode: 5,
                edit: TemplateEdit::Roots {
                    edit: ListEdit::Insert {
                        index: 1,
                        item: TemplateNodeKind::Text(24),
                    },
                },
            },
            Op::Template {
                vnode: 1,
                edit: TemplateEdit::SetNode {
                    node: 143,
                    kind: TemplateNodeKind::Dynamic,
                },
            },
            Op::Template {
                vnode: 3,
                edit: TemplateEdit::Children {
                    element: 3,
                    edit: ListEdit::Insert {
                        index: 6,
                        item: TemplateNodeKind::Element {
                            tag: 66,
                            namespace: None,
                        },
                    },
                },
            },
            Op::Dynamic {
                vnode: 4,
                slot: 4,
                kind: DynamicKind::Suspense {
                    mode: SuspenseMode::Ready,
                },
            },
            Op::Template {
                vnode: 7,
                edit: TemplateEdit::SetNode {
                    node: 7,
                    kind: TemplateNodeKind::Dynamic,
                },
            },
            Op::Template {
                vnode: 88,
                edit: TemplateEdit::SetNode {
                    node: 6,
                    kind: TemplateNodeKind::Dynamic,
                },
            },
            Op::Template {
                vnode: 0,
                edit: TemplateEdit::Children {
                    element: 1,
                    edit: ListEdit::Insert {
                        index: 5,
                        item: TemplateNodeKind::Dynamic,
                    },
                },
            },
            Op::Dynamic {
                vnode: 4,
                slot: 2,
                kind: DynamicKind::ComponentB,
            },
            Op::WakeSuspense { suspense: 120 },
            Op::Dynamic {
                vnode: 1,
                slot: 5,
                kind: DynamicKind::Suspense {
                    mode: SuspenseMode::Ready,
                },
            },
            Op::Template {
                vnode: 6,
                edit: TemplateEdit::SetNode {
                    node: 7,
                    kind: TemplateNodeKind::Dynamic,
                },
            },
            Op::WakeSuspense { suspense: 4 },
            Op::Template {
                vnode: 5,
                edit: TemplateEdit::SetNode {
                    node: 7,
                    kind: TemplateNodeKind::Element {
                        tag: 0,
                        namespace: Some(0),
                    },
                },
            },
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
            Op::Template {
                vnode: 183,
                edit: TemplateEdit::Roots {
                    edit: ListEdit::Insert {
                        index: 0,
                        item: TemplateNodeKind::Dynamic,
                    },
                },
            },
            Op::Dynamic {
                vnode: 0,
                slot: 1,
                kind: DynamicKind::Suspense {
                    mode: SuspenseMode::Pending,
                },
            },
            Op::Template {
                vnode: 7,
                edit: TemplateEdit::Roots {
                    edit: ListEdit::Insert {
                        index: 1,
                        item: TemplateNodeKind::Dynamic,
                    },
                },
            },
            Op::Suspense {
                suspense: 4,
                mode: SuspenseMode::Resolved,
            },
            Op::Dynamic {
                vnode: 3,
                slot: 2,
                kind: DynamicKind::Suspense {
                    mode: SuspenseMode::Ready,
                },
            },
            Op::Rerender,
            Op::Suspense {
                suspense: 0,
                mode: SuspenseMode::Ready,
            },
            Op::Rerender,
            Op::Suspense {
                suspense: 1,
                mode: SuspenseMode::Resolved,
            },
            Op::WakeSuspense { suspense: 2 },
        ];

        let mut harness = Harness::fresh();
        for op in ops {
            apply_op(&mut harness, &op).unwrap();
        }
    }

    #[test]
    fn keyed_fragment_moves_nested_child_after_component_insert() {
        let ops = [
            Op::Template {
                vnode: 0,
                edit: TemplateEdit::SetNode {
                    node: 0,
                    kind: TemplateNodeKind::Dynamic,
                },
            },
            Op::Fragment {
                vnode: 0,
                slot: 0,
                edit: FragmentEdit::Children(ListEdit::Insert {
                    index: 0,
                    item: None,
                }),
            },
            Op::Fragment {
                vnode: 0,
                slot: 0,
                edit: FragmentEdit::Children(ListEdit::Insert {
                    index: 0,
                    item: None,
                }),
            },
            Op::Fragment {
                vnode: 0,
                slot: 0,
                edit: FragmentEdit::Children(ListEdit::Insert {
                    index: 0,
                    item: None,
                }),
            },
            Op::Fragment {
                vnode: 0,
                slot: 0,
                edit: FragmentEdit::KeyMode(FragmentKeyMode::Keyed { base: 0 }),
            },
            Op::Fragment {
                vnode: 0,
                slot: 0,
                edit: FragmentEdit::Children(ListEdit::Insert {
                    index: 0,
                    item: None,
                }),
            },
            Op::Template {
                vnode: 6,
                edit: TemplateEdit::SetNode {
                    node: 0,
                    kind: TemplateNodeKind::Dynamic,
                },
            },
            Op::Template {
                vnode: 7,
                edit: TemplateEdit::Children {
                    element: 0,
                    edit: ListEdit::Insert {
                        index: 0,
                        item: TemplateNodeKind::Dynamic,
                    },
                },
            },
            Op::Fragment {
                vnode: 0,
                slot: 0,
                edit: FragmentEdit::Children(ListEdit::Insert {
                    index: 0,
                    item: None,
                }),
            },
            Op::Fragment {
                vnode: 177,
                slot: 0,
                edit: FragmentEdit::Children(ListEdit::Insert {
                    index: 0,
                    item: None,
                }),
            },
            Op::Rerender,
            Op::Dynamic {
                vnode: 2,
                slot: 0,
                kind: DynamicKind::ComponentA,
            },
            Op::Fragment {
                vnode: 0,
                slot: 0,
                edit: FragmentEdit::Children(ListEdit::Move { from: 3, to: 2 }),
            },
            Op::Rerender,
        ];

        let mut harness = Harness::fresh();
        for op in ops {
            apply_op(&mut harness, &op).unwrap();
        }
    }

    #[test]
    fn keyed_fragment_remove_after_domless_child_move_keeps_parent_links() {
        let ops = [
            Op::Template {
                vnode: 0,
                edit: TemplateEdit::SetNode {
                    node: 0,
                    kind: TemplateNodeKind::Dynamic,
                },
            },
            Op::Fragment {
                vnode: 0,
                slot: 0,
                edit: FragmentEdit::Children(ListEdit::Insert {
                    index: 0,
                    item: None,
                }),
            },
            Op::Fragment {
                vnode: 0,
                slot: 0,
                edit: FragmentEdit::Children(ListEdit::Insert {
                    index: 0,
                    item: None,
                }),
            },
            Op::Fragment {
                vnode: 0,
                slot: 0,
                edit: FragmentEdit::Children(ListEdit::Insert {
                    index: 0,
                    item: None,
                }),
            },
            Op::Fragment {
                vnode: 0,
                slot: 0,
                edit: FragmentEdit::KeyMode(FragmentKeyMode::Keyed { base: 0 }),
            },
            Op::Template {
                vnode: 6,
                edit: TemplateEdit::SetNode {
                    node: 0,
                    kind: TemplateNodeKind::Dynamic,
                },
            },
            Op::Fragment {
                vnode: 0,
                slot: 0,
                edit: FragmentEdit::Children(ListEdit::Insert {
                    index: 0,
                    item: None,
                }),
            },
            Op::Rerender,
            Op::Fragment {
                vnode: 0,
                slot: 0,
                edit: FragmentEdit::Children(ListEdit::Move { from: 3, to: 2 }),
            },
            Op::Fragment {
                vnode: 0,
                slot: 0,
                edit: FragmentEdit::Children(ListEdit::Remove { index: 0 }),
            },
            Op::Rerender,
        ];

        let mut harness = Harness::fresh();
        for op in ops {
            apply_op(&mut harness, &op).unwrap();
        }
    }

    #[test]
    fn iterator_scenarios_replay() {
        for scenario in IteratorScenario::ALL {
            replay_ops(iterator_scenario_ops(scenario, 0));
        }
    }
}
