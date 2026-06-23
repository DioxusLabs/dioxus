use crate::diagnostics::panic_message;
use crate::{
    context::HarnessContext,
    lifecycle::{LifecycleKey, LifecycleRole, LifecycleRun, LifecycleSnapshot},
    model::*,
    ops::{EventBehaviorSpec, Op},
    vdom::App,
};
use dioxus_core::{AttributeValue, ElementId, Event, ScopeId, VirtualDom, WriteMutations};
use dioxus_renderer_oracle::{EventListenerTarget, RendererOracle, SnapshotNode};
use std::{
    any::Any,
    cell::RefCell,
    collections::BTreeSet,
    fmt,
    future::Future,
    panic,
    rc::Rc,
    task::{Context, Poll, Waker},
};

type TargetSnapshots = Vec<SnapshotNode>;

pub(crate) struct Harness {
    vdom: Rc<RefCell<VirtualDom>>,
    incremental: Rc<RefCell<TargetedRendererOracle>>,
    context: HarnessContext,
    strict_renderer_errors: bool,
    strict_lifecycle_errors: bool,
}

#[derive(Clone, Copy)]
struct EventScopeContext;

#[derive(Clone, Copy)]
struct EventRootContext;

struct AnyRootContext;

impl Harness {
    pub(crate) fn fresh() -> Self {
        Self::fresh_with_strict_options(cfg!(fuzzing), cfg!(fuzzing))
    }

    #[cfg(test)]
    pub(crate) fn fresh_strict() -> Self {
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
        let context = HarnessContext::new();
        context.clear_suspense_ready_tasks();
        context.lifecycle.reset_all();
        context.with_model(|model| *model = Model::initial());
        let vdom = Rc::new(RefCell::new(VirtualDom::new_with_props(
            App,
            context.clone(),
        )));
        {
            let mut dom = vdom.borrow_mut();
            inspect_scope_state_accessors(&dom);
            dom.insert_any_root_context(Box::new(AnyRootContext));
            dom.insert_any_root_context(Box::new(AnyRootContext));
        }
        let incremental = Rc::new(RefCell::new(TargetedRendererOracle::new()));
        context.lifecycle.with_run(LifecycleRun::Incremental, || {
            vdom.borrow_mut().rebuild(&mut *incremental.borrow_mut())
        });
        inspect_scope_state_accessors(&vdom.borrow());
        incremental.borrow().assert_stack_clean();
        let state = Self {
            vdom,
            incremental,
            context,
            strict_renderer_errors,
            strict_lifecycle_errors,
        };
        if strict_lifecycle_errors {
            let (_, fresh_lifecycle) = build_fresh_check(&state.context).unwrap();
            check_lifecycle_matches_fresh_snapshot(&state.context, &fresh_lifecycle).unwrap();
        }
        state
    }
}

fn inspect_scope_state_accessors(dom: &VirtualDom) {
    for id in [
        ScopeId::ROOT,
        ScopeId::ROOT_SUSPENSE_BOUNDARY,
        ScopeId::ROOT_ERROR_BOUNDARY,
        ScopeId::APP,
    ] {
        let Some(scope) = dom.get_scope(id) else {
            continue;
        };
        let id = scope.id();
        let _ = format!("{id:?}");
        let _ = scope.height();
        if scope.try_root_node().is_some() {
            let _ = scope.root_node();
        }
    }
}

struct TargetedRendererOracle {
    renderer: RendererOracle,
    historical_event_listener_targets: BTreeSet<EventListenerTarget>,
    last_mutation: Option<MutationTrace>,
    recent_mutations: [Option<MutationTrace>; RECENT_MUTATION_LIMIT],
    recent_mutation_start: usize,
    recent_mutation_len: usize,
}

const RECENT_MUTATION_LIMIT: usize = 64;

#[derive(Clone, Debug)]
enum MutationTrace {
    PushId { id: ElementId },
    SetId { id: ElementId },
    Child { index: usize },
    Pop,
    CreateElement { tag: String },
    CreateText { len: usize },
    Clone,
    AppendChildren { m: usize },
    ReplaceWith { m: usize },
    InsertAfter { m: usize },
    InsertBefore { m: usize },
    SetAttribute { name: String },
    SetText { len: usize },
    AddEventListener { name: String, id: Option<ElementId> },
    RemoveEventListener { name: String, id: Option<ElementId> },
    Remove,
}

impl fmt::Display for MutationTrace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PushId { id } => write!(f, "push_id(id: {id:?})"),
            Self::SetId { id } => write!(f, "set_id(id: {id:?})"),
            Self::Child { index } => write!(f, "child(index: {index})"),
            Self::Pop => write!(f, "pop()"),
            Self::CreateElement { tag } => write!(f, "create_element(tag: {tag:?})"),
            Self::CreateText { len } => write!(f, "create_text(len: {len})"),
            Self::Clone => write!(f, "clone()"),
            Self::AppendChildren { m } => write!(f, "append_children(m: {m})"),
            Self::ReplaceWith { m } => write!(f, "replace_with(m: {m})"),
            Self::InsertAfter { m } => write!(f, "insert_after(m: {m})"),
            Self::InsertBefore { m } => write!(f, "insert_before(m: {m})"),
            Self::SetAttribute { name } => write!(f, "set_attribute(name: {name:?})"),
            Self::SetText { len } => write!(f, "set_text(len: {len})"),
            Self::AddEventListener { name, id } => {
                write!(f, "add_event_listener(name: {name:?}, id: {id:?})")
            }
            Self::RemoveEventListener { name, id } => {
                write!(f, "remove_event_listener(name: {name:?}, id: {id:?})")
            }
            Self::Remove => write!(f, "remove()"),
        }
    }
}

impl TargetedRendererOracle {
    fn new() -> Self {
        Self {
            renderer: RendererOracle::new(),
            historical_event_listener_targets: BTreeSet::new(),
            last_mutation: None,
            recent_mutations: std::array::from_fn(|_| None),
            recent_mutation_start: 0,
            recent_mutation_len: 0,
        }
    }

    fn current_renderer(&mut self) -> &mut RendererOracle {
        &mut self.renderer
    }

    fn record_mutation(&mut self, mutation: MutationTrace) {
        self.last_mutation = Some(mutation.clone());
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
            if let Some(mutation) = self.recent_mutations[index].as_ref() {
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

    fn check_matches_fresh(&self, fresh: &RendererOracle) -> Result<(), String> {
        let fresh_snapshot = fresh.snapshot();
        if self.renderer.snapshot_eq(&fresh_snapshot) {
            return Ok(());
        }

        let incremental_snapshot = self.snapshot();
        let recent_mutations = self.recent_mutations_text();
        Err(format!(
            "incremental renderer snapshot does not match fresh render\nincremental:\n{incremental_snapshot:#?}\nfresh:\n{fresh_snapshot:#?}\nrecent mutations:\n  {recent_mutations}"
        ))
    }

    fn snapshot(&self) -> TargetSnapshots {
        self.renderer.snapshot()
    }

    fn historical_event_listener_targets(&self) -> Vec<EventListenerTarget> {
        self.historical_event_listener_targets
            .iter()
            .cloned()
            .collect()
    }
}

impl WriteMutations for TargetedRendererOracle {
    fn push_id(&mut self, id: ElementId) {
        self.record_mutation(MutationTrace::PushId { id });
        self.current_renderer().push_id(id);
    }

    fn set_id(&mut self, id: ElementId) {
        self.record_mutation(MutationTrace::SetId { id });
        self.current_renderer().set_id(id);
    }

    fn child(&mut self, index: usize) {
        self.record_mutation(MutationTrace::Child { index });
        self.current_renderer().child(index);
    }

    fn pop(&mut self) {
        self.record_mutation(MutationTrace::Pop);
        self.current_renderer().pop();
    }

    fn create_element(&mut self, tag: &str, ns: Option<&str>) {
        self.record_mutation(MutationTrace::CreateElement {
            tag: tag.to_string(),
        });
        self.current_renderer().create_element(tag, ns);
    }

    fn create_text(&mut self, value: &str) {
        self.record_mutation(MutationTrace::CreateText { len: value.len() });
        self.current_renderer().create_text(value);
    }

    fn clone(&mut self) {
        self.record_mutation(MutationTrace::Clone);
        WriteMutations::clone(self.current_renderer());
    }

    fn append_children(&mut self, m: usize) {
        self.record_mutation(MutationTrace::AppendChildren { m });
        self.current_renderer().append_children(m);
    }

    fn replace_with(&mut self, m: usize) {
        self.record_mutation(MutationTrace::ReplaceWith { m });
        self.current_renderer().replace_with(m);
    }

    fn insert_after(&mut self, m: usize) {
        self.record_mutation(MutationTrace::InsertAfter { m });
        self.current_renderer().insert_after(m);
    }

    fn insert_before(&mut self, m: usize) {
        self.record_mutation(MutationTrace::InsertBefore { m });
        self.current_renderer().insert_before(m);
    }

    fn set_attribute(&mut self, name: &str, ns: Option<&str>, value: &AttributeValue) {
        self.record_mutation(MutationTrace::SetAttribute {
            name: name.to_string(),
        });
        self.current_renderer().set_attribute(name, ns, value);
    }

    fn set_text(&mut self, value: &str) {
        self.record_mutation(MutationTrace::SetText { len: value.len() });
        self.current_renderer().set_text(value);
    }

    fn add_event_listener(&mut self, name: &str) {
        let id = self.renderer.current_stack_element_id();
        self.record_mutation(MutationTrace::AddEventListener {
            name: name.to_string(),
            id,
        });
        self.current_renderer().add_event_listener(name);
        if let Some(id) = id {
            self.historical_event_listener_targets
                .insert(EventListenerTarget {
                    name: name.to_string(),
                    id,
                });
        }
    }

    fn remove_event_listener(&mut self, name: &str) {
        let id = self.renderer.current_stack_element_id();
        self.record_mutation(MutationTrace::RemoveEventListener {
            name: name.to_string(),
            id,
        });
        self.current_renderer().remove_event_listener(name);
    }

    fn remove(&mut self) {
        self.record_mutation(MutationTrace::Remove);
        self.current_renderer().remove();
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

fn render_model_with_ssr(context: &HarnessContext, model: &Model) -> Result<String, String> {
    catch_unwind_result(|| {
        context.without_suspense_ready_registration(|| {
            context.with_model(|global| *global = model.clone());
            let mut vdom = VirtualDom::new_with_props(App, context.clone());
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
    println!("fuzz failure");
    println!("decoded operations: {}", ops.len());
    println!("reported failing step: {failing_step}");
    println!("summary: {}", first_line(minimized_error));
    println!();
    print_op_list(ops, failing_step);
    println!();
    println!("ssr replay around failing step:");

    let mut state = Harness::fresh();
    let mut current_model = Model::initial();
    let mut current_html = render_model_with_ssr(&state.context, &current_model);
    let (trace_start, trace_end) = trace_bounds(ops.len(), failing_step);

    if trace_start == 0 {
        println!("  initial");
        print_html_line("html:", &current_html);
    } else {
        println!("  replaying first {trace_start} steps without logging");
    }

    let mut reproduced_error = None;
    for (index, op) in ops.iter().enumerate() {
        state
            .context
            .with_model(|global| *global = current_model.clone());
        let should_log = index >= trace_start && index < trace_end;

        if should_log {
            println!();
            println!("  step {index}");
            println!("    op:     {op:?}");
            print_html_line("before:", &current_html);
        }

        let applied = catch_unwind_result(|| apply_op(&mut state, op)).unwrap_or_else(|payload| {
            Err(format!(
                "panic while replaying operation: {}",
                panic_message(&payload)
            ))
        });

        match applied {
            Ok(()) => {
                let next_model = state.context.read_model();
                let next_html = render_model_with_ssr(&state.context, &next_model);
                if should_log {
                    print_html_line("after:", &next_html);
                    println!("    status: ok");
                }
                current_model = next_model;
                current_html = next_html;
            }
            Err(err) => {
                let next_model = state.context.read_model();
                let next_html = render_model_with_ssr(&state.context, &next_model);
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
        Op::Rerender => render_app_and_assert(state),
        Op::WakeSuspense { suspense } => {
            let Some(key) = state
                .context
                .selected_registered_ready_suspense_key(*suspense)
            else {
                return Ok(());
            };
            state.context.release_suspense_ready_task(key);
            state
                .context
                .with_model(|model| model.wake_ready_suspense(key));
            state.vdom.borrow_mut().mark_dirty(ScopeId::APP);
            render_dirty_and_assert(state)
        }
        Op::FireEvent { target, behavior } => {
            fire_selected_event_listener(state, *target, *behavior)
        }
        Op::RenderDirty => render_dirty_and_assert(state),
        Op::RenderSuspenseDirty => render_suspense_dirty_and_assert(state),
        Op::Mutate(_) => {
            state.context.apply_to_model(op);
            state.vdom.borrow_mut().mark_dirty(ScopeId::APP);
            Ok(())
        }
    }
}

fn fire_historical_event_listeners(state: &Harness) -> Result<(), String> {
    let targets = state
        .incremental
        .borrow()
        .historical_event_listener_targets();
    if targets.is_empty() {
        return Ok(());
    }

    let runtime = state.vdom.borrow().runtime();
    for target in targets {
        let event = Event::new(
            Rc::new(String::from("fuzzer stale event")) as Rc<dyn Any>,
            true,
        );
        runtime.handle_event(&target.name, event, target.id);
    }
    Ok(())
}

fn fire_selected_event_listener(
    state: &mut Harness,
    target_selector: u8,
    behavior: EventBehaviorSpec,
) -> Result<(), String> {
    let targets = state
        .incremental
        .borrow()
        .historical_event_listener_targets();
    if targets.is_empty() {
        return Ok(());
    }

    let target = targets[target_selector as usize % targets.len()].clone();
    let runtime = state.vdom.borrow().runtime();
    let nested_runtime = runtime.clone();
    let nested_targets = targets.clone();
    let events = state.context.events.clone();
    let nested_events = events.clone();
    let listener_driver = Rc::new(move |behavior| match behavior {
        EventBehaviorSpec::Noop => {}
        EventBehaviorSpec::DispatchNestedEvent { target } => {
            let Some(target) = nested_targets.get(target as usize % nested_targets.len()) else {
                return;
            };
            let event = Event::new(
                Rc::new(String::from("fuzzer nested event")) as Rc<dyn Any>,
                true,
            );
            nested_events.with_listener_driver(EventBehaviorSpec::Noop, Rc::new(|_| {}), || {
                nested_runtime.handle_event(&target.name, event, target.id)
            });
        }
        EventBehaviorSpec::ScheduleUpdate => {
            let update = dioxus_core::schedule_update();
            update();
        }
        EventBehaviorSpec::ScheduleUpdateAny => {
            let id = dioxus_core::current_scope_id();
            let update_any = dioxus_core::schedule_update_any();
            update_any(id);
        }
        EventBehaviorSpec::NeedsUpdate => dioxus_core::needs_update(),
        EventBehaviorSpec::NeedsUpdateAny => {
            let id = dioxus_core::current_scope_id();
            dioxus_core::needs_update_any(id);
        }
        EventBehaviorSpec::ContextRoundTrip => {
            let id = dioxus_core::current_scope_id();
            dioxus_core::provide_context(EventScopeContext);
            dioxus_core::provide_context(EventScopeContext);
            let _ = dioxus_core::has_context::<EventScopeContext>();
            let _ = dioxus_core::try_consume_context::<EventScopeContext>();
            let _ = dioxus_core::consume_context::<EventScopeContext>();
            let _ = dioxus_core::consume_context_from_scope::<EventScopeContext>(id);
        }
        EventBehaviorSpec::RootContextRoundTrip => {
            dioxus_core::provide_root_context(EventRootContext);
            dioxus_core::provide_root_context(EventRootContext);
            let _ = dioxus_core::try_consume_context::<EventRootContext>();
            let _ = dioxus_core::consume_context::<EventRootContext>();
            let _ = dioxus_core::consume_context_from_scope::<EventRootContext>(ScopeId::ROOT);
        }
        EventBehaviorSpec::QueueEffect => dioxus_core::queue_effect(|| {}),
        EventBehaviorSpec::SpawnIsomorphic => {
            let _ = dioxus_core::spawn_isomorphic(async {});
        }
    });

    events.with_listener_driver(behavior, listener_driver, || {
        let event = Event::new(
            Rc::new(String::from("fuzzer explicit event")) as Rc<dyn Any>,
            true,
        );
        runtime.handle_event(&target.name, event, target.id);
    });

    if event_behavior_queues_work(behavior) {
        render_dirty_and_assert(state)
    } else {
        Ok(())
    }
}

fn event_behavior_queues_work(behavior: EventBehaviorSpec) -> bool {
    matches!(
        behavior,
        EventBehaviorSpec::ScheduleUpdate
            | EventBehaviorSpec::ScheduleUpdateAny
            | EventBehaviorSpec::NeedsUpdate
            | EventBehaviorSpec::NeedsUpdateAny
            | EventBehaviorSpec::QueueEffect
            | EventBehaviorSpec::SpawnIsomorphic
    )
}

fn render_once(state: &mut Harness, assert_lifecycle_matches_fresh: bool) -> Result<(), String> {
    fire_historical_event_listeners(state)?;
    state
        .context
        .lifecycle
        .with_run(LifecycleRun::Incremental, || {
            state
                .vdom
                .borrow_mut()
                .render_immediate(&mut *state.incremental.borrow_mut())
        });
    check_incremental_state(state, assert_lifecycle_matches_fresh)
}

fn check_incremental_state(
    state: &Harness,
    assert_lifecycle_matches_fresh: bool,
) -> Result<(), String> {
    let incremental = state.incremental.borrow();
    incremental.check_stack_clean().map_err(|err| {
        let last_mutation = incremental
            .last_mutation
            .as_ref()
            .map_or_else(|| "<none>".to_string(), |mutation| mutation.to_string());
        let recent_mutations = incremental.recent_mutations_text();
        format!("{err} after {last_mutation}\nrecent mutations:\n  {recent_mutations}")
    })?;
    let (fresh_renderer, fresh_lifecycle) = build_fresh_check(&state.context)?;
    incremental.check_matches_fresh(&fresh_renderer)?;
    if assert_lifecycle_matches_fresh {
        check_lifecycle_matches_fresh_snapshot(&state.context, &fresh_lifecycle).map_err(
            |err| {
                let last_mutation = incremental
                    .last_mutation
                    .as_ref()
                    .map_or_else(|| "<none>".to_string(), |mutation| mutation.to_string());
                let recent_mutations = incremental.recent_mutations_text();
                format!("{err} after {last_mutation}\nrecent mutations:\n  {recent_mutations}")
            },
        )?;
    }
    Ok(())
}

fn render_app_and_assert(state: &mut Harness) -> Result<(), String> {
    state.vdom.borrow_mut().mark_dirty(ScopeId::APP);
    let compare_lifecycle = state.strict_lifecycle_errors;
    let result = render_once(state, compare_lifecycle);
    render_result_to_fuzz_failure(state, result)
}

fn render_dirty_and_assert(state: &mut Harness) -> Result<(), String> {
    let compare_lifecycle = state.strict_lifecycle_errors;
    let result = render_once(state, compare_lifecycle);
    render_result_to_fuzz_failure(state, result)
}

fn render_suspense_dirty_and_assert(state: &mut Harness) -> Result<(), String> {
    fire_historical_event_listeners(state)?;
    let result = state
        .context
        .lifecycle
        .with_run(LifecycleRun::Incremental, || {
            let mut dom = state.vdom.borrow_mut();
            // Mirror SSR's `rebuild` -> `render_suspense_immediate` ordering:
            // flush any deferred foreground render before resolving suspense.
            // `render_suspense_immediate` only reruns scopes under a suspense
            // boundary, so a foreground scope left dirty here would be skipped
            // and its update lost, diverging from the fresh render.
            dom.render_immediate(&mut *state.incremental.borrow_mut());
            poll_render_suspense_immediate(&mut dom)
        });
    let compare_lifecycle = state.strict_lifecycle_errors;
    let result = result.and_then(|()| check_incremental_state(state, compare_lifecycle));
    render_result_to_fuzz_failure(state, result)
}

fn poll_render_suspense_immediate(dom: &mut VirtualDom) -> Result<(), String> {
    let mut cx = Context::from_waker(Waker::noop());
    let mut future = std::pin::pin!(dom.render_suspense_immediate());
    for _ in 0..4096 {
        match Future::poll(future.as_mut(), &mut cx) {
            Poll::Ready(_) => return Ok(()),
            Poll::Pending => {}
        }
    }
    Err("render_suspense_immediate did not complete after 4096 polls".to_string())
}

fn build_fresh_check(
    context: &HarnessContext,
) -> Result<(RendererOracle, LifecycleSnapshot), String> {
    context.lifecycle.reset_run(LifecycleRun::Fresh);
    let mut fresh_vdom = VirtualDom::new_with_props(App, context.clone());
    let mut renderer = RendererOracle::new();
    context.without_suspense_ready_registration(|| {
        context
            .lifecycle
            .with_run(LifecycleRun::Fresh, || fresh_vdom.rebuild(&mut renderer));
    });
    renderer.check_stack_clean()?;

    Ok((renderer, context.lifecycle.snapshot(LifecycleRun::Fresh)))
}

fn check_lifecycle_matches_fresh_snapshot(
    context: &HarnessContext,
    fresh: &LifecycleSnapshot,
) -> Result<(), String> {
    let incremental = context.lifecycle.snapshot(LifecycleRun::Incremental);
    let model = expected_model_lifecycle_snapshot(context);
    if lifecycle_is_within_expected_bounds(context, &incremental, fresh, &model) {
        return Ok(());
    }

    let retaining_suspense_ids = retaining_suspense_ids(context, &incremental, fresh, &model);
    let retained_suspended = context
        .lifecycle
        .snapshot_with_suspense_ancestor(LifecycleRun::Incremental, &retaining_suspense_ids);
    let model_suspended =
        model_lifecycle_with_suspense_ancestor_snapshot(context, &retaining_suspense_ids);
    Err(lifecycle_mismatch_error(
        &incremental,
        fresh,
        &model,
        &retained_suspended,
        &model_suspended,
    ))
}

fn lifecycle_is_within_expected_bounds(
    context: &HarnessContext,
    incremental: &LifecycleSnapshot,
    fresh: &LifecycleSnapshot,
    model: &LifecycleSnapshot,
) -> bool {
    let retaining_suspense_ids = retaining_suspense_ids(context, incremental, fresh, model);
    let retained_suspended_subtree_lifecycle = context
        .lifecycle
        .snapshot_with_suspense_ancestor(LifecycleRun::Incremental, &retaining_suspense_ids);
    let model_suspended_subtree_lifecycle =
        model_lifecycle_with_suspense_ancestor_snapshot(context, &retaining_suspense_ids);
    let has_all_visible_fresh_components = fresh
        .iter()
        .filter(|(key, _)| lifecycle_role_is_strict(**key))
        .all(|(key, count)| {
            let model_count = model.get(key).copied().unwrap_or(0);
            let required_visible_count = (*count).min(model_count);
            incremental.get(key).copied().unwrap_or(0) >= required_visible_count
        });
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

fn expected_model_lifecycle_snapshot(context: &HarnessContext) -> LifecycleSnapshot {
    let model = context.read_model();
    let mut out = LifecycleSnapshot::new();
    model.root.visit(&mut |visit, _| match visit {
        ModelVisit::Dynamic(DynamicSpec::ComponentA(component)) => {
            add_lifecycle_key(&mut out, LifecycleRole::ComponentA, component.id);
        }
        ModelVisit::Dynamic(DynamicSpec::ComponentB(component)) => {
            add_lifecycle_key(&mut out, LifecycleRole::ComponentB, component.id);
        }
        ModelVisit::Dynamic(DynamicSpec::Suspense(spec)) => {
            add_lifecycle_key(&mut out, LifecycleRole::SuspenseBoundary, spec.id);
            add_lifecycle_key(&mut out, LifecycleRole::SuspenseChild, spec.id);
        }
        _ => {}
    });
    out
}

fn retaining_suspense_ids(
    context: &HarnessContext,
    incremental: &LifecycleSnapshot,
    fresh: &LifecycleSnapshot,
    model: &LifecycleSnapshot,
) -> BTreeSet<u64> {
    let current_model = context.read_model();
    let mut out = BTreeSet::new();
    // Core suspense can retain previous child state while a reused boundary
    // moves between fallback and resolved output, even if the model suspense is
    // currently resolved. Bound retained extras by current boundary ancestry.
    current_model.root.visit(&mut |visit, _| {
        if let ModelVisit::Dynamic(DynamicSpec::Suspense(spec)) = visit {
            out.insert(spec.id);
        }
    });

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

/// Lifecycle keys for generated components that live under one of the given
/// suspense boundary ids, in the current model.
fn model_lifecycle_with_suspense_ancestor_snapshot(
    context: &HarnessContext,
    suspense_ids: &BTreeSet<u64>,
) -> LifecycleSnapshot {
    let model = context.read_model();
    let mut out = LifecycleSnapshot::new();
    model.root.visit(&mut |visit, suspense_ancestors| {
        let (role, id) = match visit {
            ModelVisit::Dynamic(DynamicSpec::ComponentA(component)) => {
                (LifecycleRole::ComponentA, component.id)
            }
            ModelVisit::Dynamic(DynamicSpec::ComponentB(component)) => {
                (LifecycleRole::ComponentB, component.id)
            }
            _ => return,
        };
        if suspense_ancestors
            .iter()
            .any(|ancestor| suspense_ids.contains(ancestor))
        {
            add_lifecycle_key(&mut out, role, id);
        }
    });
    out
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
            TemplateNodeKind, TemplateNodeSpec, WakeMutationSpec,
        },
        ops::{EventBehaviorSpec, FragmentEdit, ListEdit, TemplateEdit},
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

    fn first_suspense_mode_and_wake_count(context: &HarnessContext) -> Option<(SuspenseMode, u8)> {
        let model = context.read_model();
        let DynamicSpec::Suspense(spec) = first_dynamic(&model.root.template.roots)? else {
            return None;
        };
        Some((spec.mode, spec.ready_wake_count))
    }

    fn first_dynamic(nodes: &[TemplateNodeSpec]) -> Option<&DynamicSpec> {
        for node in nodes {
            match node {
                TemplateNodeSpec::Element { children, .. } => {
                    if let Some(dynamic) = first_dynamic(children) {
                        return Some(dynamic);
                    }
                }
                TemplateNodeSpec::Text(_) => {}
                TemplateNodeSpec::Dynamic(dynamic) => return Some(dynamic),
            }
        }
        None
    }

    fn set_pending_suspense_model(context: &HarnessContext) {
        context.with_model(|model| *model = Model::initial());
        context.apply_to_model(&Op::template(
            0,
            TemplateEdit::SetNode {
                node: 0,
                kind: TemplateNodeKind::Dynamic(DynamicKind::Empty),
            },
        ));
        context.apply_to_model(&Op::dynamic(
            0,
            0,
            DynamicKind::Suspense {
                mode: SuspenseMode::Pending,
            },
        ));
    }

    fn set_resolved_suspense_with_component_b_model(context: &HarnessContext) {
        context.with_model(|model| *model = Model::initial());
        context.apply_to_model(&Op::template(
            0,
            TemplateEdit::SetNode {
                node: 0,
                kind: TemplateNodeKind::Dynamic(DynamicKind::Suspense {
                    mode: SuspenseMode::Resolved,
                }),
            },
        ));
        context.apply_to_model(&Op::template(
            1,
            TemplateEdit::Roots {
                edit: ListEdit::Insert {
                    index: 0,
                    item: TemplateNodeKind::Dynamic(DynamicKind::ComponentB),
                },
            },
        ));
    }

    fn mount_listener_ops() -> Vec<Op> {
        vec![
            Op::template(
                0,
                TemplateEdit::Attrs {
                    element: 0,
                    edit: ListEdit::Insert {
                        index: 0,
                        item: TemplateAttrSpec::Dynamic(Vec::new()),
                    },
                },
            ),
            Op::dynamic_attrs(
                0,
                0,
                ListEdit::Insert {
                    index: 0,
                    item: AttrSpec {
                        name: 1,
                        namespace: None,
                        value: AttrValueSpec::Listener,
                        volatile: false,
                    },
                },
            ),
            Op::Rerender,
        ]
    }

    #[test]
    fn vnode_mutation_still_compares_fresh_render() {
        let mut harness = Harness::fresh_strict();

        apply_op(
            &mut harness,
            &Op::template(
                0,
                TemplateEdit::SetNode {
                    node: 0,
                    kind: TemplateNodeKind::Dynamic(DynamicKind::Empty),
                },
            ),
        )
        .unwrap();

        apply_op(&mut harness, &Op::Rerender).unwrap();
    }

    #[test]
    fn explicit_noop_event_fires_listener_without_rendering() {
        let mut harness = Harness::fresh_strict();
        for op in mount_listener_ops() {
            apply_op(&mut harness, &op).unwrap();
        }

        assert_eq!(
            harness
                .incremental
                .borrow()
                .historical_event_listener_targets()
                .len(),
            1
        );
        apply_op(&mut harness, &Op::fire_event(0, EventBehaviorSpec::Noop)).unwrap();
    }

    #[test]
    fn explicit_nested_event_ignores_reentrant_dispatch() {
        let mut harness = Harness::fresh_strict();
        for op in mount_listener_ops() {
            apply_op(&mut harness, &op).unwrap();
        }

        assert_eq!(
            harness
                .incremental
                .borrow()
                .historical_event_listener_targets()
                .len(),
            1
        );
        apply_op(
            &mut harness,
            &Op::fire_event(0, EventBehaviorSpec::DispatchNestedEvent { target: 0 }),
        )
        .unwrap();
    }

    #[test]
    fn suspense_slot_mutation_still_compares_fresh_render() {
        let mut harness = Harness::fresh_strict();

        apply_op(
            &mut harness,
            &Op::template(
                0,
                TemplateEdit::SetNode {
                    node: 0,
                    kind: TemplateNodeKind::Dynamic(DynamicKind::Empty),
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
                    mode: SuspenseMode::Ready { wake_after: 0 },
                },
            ),
        )
        .unwrap();

        apply_op(&mut harness, &Op::Rerender).unwrap();
    }

    #[test]
    fn ready_suspense_resolves_after_configured_real_wakes() {
        let mut harness = Harness::fresh_strict();

        apply_op(
            &mut harness,
            &Op::template(
                0,
                TemplateEdit::SetNode {
                    node: 0,
                    kind: TemplateNodeKind::Dynamic(DynamicKind::Empty),
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
                    mode: SuspenseMode::Ready { wake_after: 1 },
                },
            ),
        )
        .unwrap();
        apply_op(&mut harness, &Op::Rerender).unwrap();

        apply_op(&mut harness, &Op::wake_suspense(0)).unwrap();
        assert!(
            harness
                .context
                .read_model()
                .selected_ready_suspense_key(0)
                .is_some()
        );
        assert_eq!(
            first_suspense_mode_and_wake_count(&harness.context),
            Some((SuspenseMode::Ready { wake_after: 1 }, 1))
        );

        apply_op(&mut harness, &Op::wake_suspense(0)).unwrap();
        assert!(
            harness
                .context
                .read_model()
                .selected_ready_suspense_key(0)
                .is_none()
        );
        assert_eq!(
            first_suspense_mode_and_wake_count(&harness.context),
            Some((SuspenseMode::Resolved, 2))
        );
    }

    #[test]
    fn waking_hidden_nested_suspense_keeps_renderer_stack_balanced() {
        replay_ops([
            Op::template(
                0,
                TemplateEdit::SetNode {
                    node: 0,
                    kind: TemplateNodeKind::Dynamic(DynamicKind::Suspense {
                        mode: SuspenseMode::Resolved,
                    }),
                },
            ),
            Op::template(
                1,
                TemplateEdit::Roots {
                    edit: ListEdit::Insert {
                        index: 0,
                        item: TemplateNodeKind::Dynamic(DynamicKind::Suspense {
                            mode: SuspenseMode::Ready { wake_after: 0 },
                        }),
                    },
                },
            ),
            Op::Rerender,
            Op::suspense(2, SuspenseMode::Pending),
            Op::wake_suspense(4),
        ]);
    }

    #[test]
    fn resolved_suspense_with_edited_child_matches_fresh_render() {
        replay_ops([
            Op::template(
                0,
                TemplateEdit::SetNode {
                    node: 0,
                    kind: TemplateNodeKind::Dynamic(DynamicKind::Empty),
                },
            ),
            Op::dynamic(
                0,
                0,
                DynamicKind::Suspense {
                    mode: SuspenseMode::Ready { wake_after: 0 },
                },
            ),
            Op::Rerender,
            Op::suspense(240, SuspenseMode::Resolved),
            Op::dynamic(1, 51, DynamicKind::ComponentA),
            Op::Rerender,
        ]);
    }

    #[test]
    fn removing_root_after_resolving_nested_suspense_drops_stale_component_state() {
        replay_ops_with_lifecycle([
            Op::template(
                0,
                TemplateEdit::Roots {
                    edit: ListEdit::Insert {
                        index: 1,
                        item: TemplateNodeKind::Dynamic(DynamicKind::Suspense {
                            mode: SuspenseMode::Ready { wake_after: 90 },
                        }),
                    },
                },
            ),
            Op::template(
                123,
                TemplateEdit::SetNode {
                    node: 183,
                    kind: TemplateNodeKind::Dynamic(DynamicKind::Fragment {
                        children: 48,
                        key_base: None,
                    }),
                },
            ),
            Op::Rerender,
            Op::template(
                133,
                TemplateEdit::SetNode {
                    node: 202,
                    kind: TemplateNodeKind::Dynamic(DynamicKind::Suspense {
                        mode: SuspenseMode::Pending,
                    }),
                },
            ),
            Op::template(
                4,
                TemplateEdit::Roots {
                    edit: ListEdit::Insert {
                        index: 1,
                        item: TemplateNodeKind::Dynamic(DynamicKind::ComponentA),
                    },
                },
            ),
            Op::wake_suspense(97),
            Op::template(
                12,
                TemplateEdit::SetNode {
                    node: 0,
                    kind: TemplateNodeKind::Dynamic(DynamicKind::ComponentA),
                },
            ),
            Op::template(
                100,
                TemplateEdit::Roots {
                    edit: ListEdit::Insert {
                        index: 16,
                        item: TemplateNodeKind::Dynamic(DynamicKind::Suspense {
                            mode: SuspenseMode::Pending,
                        }),
                    },
                },
            ),
            Op::wake_suspense(50),
            Op::template(
                11,
                TemplateEdit::SetNode {
                    node: 0,
                    kind: TemplateNodeKind::Dynamic(DynamicKind::ComponentB),
                },
            ),
            Op::wake_suspense(117),
            Op::template(
                45,
                TemplateEdit::SetNode {
                    node: 9,
                    kind: TemplateNodeKind::Dynamic(DynamicKind::Suspense {
                        mode: SuspenseMode::Pending,
                    }),
                },
            ),
            Op::Rerender,
            Op::template(
                0,
                TemplateEdit::Roots {
                    edit: ListEdit::Remove { index: 95 },
                },
            ),
            Op::Rerender,
        ]);
    }

    #[test]
    fn lifecycle_oracle_rejects_stale_component_outside_unresolved_suspense() {
        let context = HarnessContext::new();
        context.lifecycle.reset_all();
        set_pending_suspense_model(&context);

        let stale_key = LifecycleKey {
            role: LifecycleRole::ComponentA,
            id: 99,
        };
        let incremental = LifecycleSnapshot::from([(stale_key, 1)]);
        let fresh = LifecycleSnapshot::new();
        let model = expected_model_lifecycle_snapshot(&context);

        assert!(!lifecycle_is_within_expected_bounds(
            &context,
            &incremental,
            &fresh,
            &model
        ));
    }

    #[test]
    fn lifecycle_oracle_allows_stale_component_inside_unresolved_suspense() {
        let context = HarnessContext::new();
        context.lifecycle.reset_all();
        set_pending_suspense_model(&context);

        let _guard = context.lifecycle.with_run(LifecycleRun::Incremental, || {
            context.lifecycle.track(LifecycleRole::ComponentA, 99, &[0])
        });
        let incremental = context.lifecycle.snapshot(LifecycleRun::Incremental);
        let fresh = LifecycleSnapshot::new();
        let model = expected_model_lifecycle_snapshot(&context);

        assert!(lifecycle_is_within_expected_bounds(
            &context,
            &incremental,
            &fresh,
            &model
        ));
    }

    #[test]
    fn lifecycle_oracle_caps_fresh_suspense_duplicates_at_model_count() {
        let context = HarnessContext::new();
        context.lifecycle.reset_all();
        set_resolved_suspense_with_component_b_model(&context);

        let component = LifecycleKey {
            role: LifecycleRole::ComponentB,
            id: 0,
        };
        let incremental = LifecycleSnapshot::from([(component, 1)]);
        let fresh = LifecycleSnapshot::from([(component, 2)]);
        let model = expected_model_lifecycle_snapshot(&context);

        assert!(lifecycle_is_within_expected_bounds(
            &context,
            &incremental,
            &fresh,
            &model
        ));
    }

    #[test]
    fn replacing_root_component_with_fragment_removes_old_subtree() {
        replay_ops([
            Op::template(
                0,
                TemplateEdit::SetNode {
                    node: 0,
                    kind: TemplateNodeKind::Dynamic(DynamicKind::Empty),
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
                    kind: TemplateNodeKind::Dynamic(DynamicKind::Empty),
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
                    kind: TemplateNodeKind::Dynamic(DynamicKind::Empty),
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
    fn keyed_fragment_splice_uses_committed_parent_view_for_child_lookup() {
        replay_ops([
            Op::template(
                0,
                TemplateEdit::SetNode {
                    node: 0,
                    kind: TemplateNodeKind::Dynamic(DynamicKind::Fragment {
                        children: 3,
                        key_base: Some(206),
                    }),
                },
            ),
            Op::Rerender,
            Op::template(
                3,
                TemplateEdit::SetNode {
                    node: 0,
                    kind: TemplateNodeKind::Dynamic(DynamicKind::Placeholder),
                },
            ),
            Op::wake_suspense(110),
            Op::fragment(
                0,
                225,
                FragmentEdit::Children(ListEdit::Insert {
                    index: 138,
                    item: None,
                }),
            ),
            Op::Rerender,
        ]);
    }

    #[test]
    fn non_keyed_fragment_pair_diff_keeps_later_siblings_available_for_placement() {
        replay_ops([
            Op::template(
                165,
                TemplateEdit::SetNode {
                    node: 194,
                    kind: TemplateNodeKind::Dynamic(DynamicKind::Suspense {
                        mode: SuspenseMode::Ready { wake_after: 44 },
                    }),
                },
            ),
            Op::wake_suspense(22),
            Op::template(
                1,
                TemplateEdit::SetNode {
                    node: 0,
                    kind: TemplateNodeKind::Dynamic(DynamicKind::Fragment {
                        children: 16,
                        key_base: None,
                    }),
                },
            ),
            Op::Rerender,
            Op::template(
                6,
                TemplateEdit::SetNode {
                    node: 168,
                    kind: TemplateNodeKind::Element {
                        tag: 179,
                        namespace: Some(178),
                    },
                },
            ),
            Op::template(
                12,
                TemplateEdit::SetNode {
                    node: 0,
                    kind: TemplateNodeKind::Text(199),
                },
            ),
            Op::Rerender,
        ]);
    }

    #[test]
    fn keyed_fragment_removals_stay_mounted_until_placements_finish() {
        replay_ops([
            Op::Rerender,
            Op::template(
                0,
                TemplateEdit::SetNode {
                    node: 0,
                    kind: TemplateNodeKind::Dynamic(DynamicKind::Suspense {
                        mode: SuspenseMode::Pending,
                    }),
                },
            ),
            Op::wake_suspense(96),
            Op::template(
                101,
                TemplateEdit::Children {
                    element: 204,
                    edit: ListEdit::Insert {
                        index: 240,
                        item: TemplateNodeKind::Dynamic(DynamicKind::Fragment {
                            children: 222,
                            key_base: Some(127),
                        }),
                    },
                },
            ),
            Op::Rerender,
            Op::template(
                160,
                TemplateEdit::Children {
                    element: 0,
                    edit: ListEdit::Insert {
                        index: 0,
                        item: TemplateNodeKind::Dynamic(DynamicKind::ComponentB),
                    },
                },
            ),
            Op::template(
                13,
                TemplateEdit::SetNode {
                    node: 139,
                    kind: TemplateNodeKind::Text(58),
                },
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
                    kind: TemplateNodeKind::Dynamic(DynamicKind::Empty),
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
                        item: TemplateNodeKind::Dynamic(DynamicKind::Empty),
                    },
                },
            ),
            Op::template(
                1,
                TemplateEdit::Children {
                    element: 0,
                    edit: ListEdit::Insert {
                        index: 1,
                        item: TemplateNodeKind::Dynamic(DynamicKind::Empty),
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
                    kind: TemplateNodeKind::Dynamic(DynamicKind::Empty),
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
                    kind: TemplateNodeKind::Dynamic(DynamicKind::Empty),
                },
            ),
            Op::dynamic(
                195,
                186,
                DynamicKind::Suspense {
                    mode: SuspenseMode::Ready { wake_after: 0 },
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
                    kind: TemplateNodeKind::Dynamic(DynamicKind::Empty),
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
            Op::wake_suspense(210),
            Op::Rerender,
            Op::suspense(0, SuspenseMode::Pending),
            Op::Rerender,
        ]);
    }

    #[test]
    fn resolved_suspense_reuses_promoted_mount_when_pending_again() {
        replay_ops([
            Op::template(
                0,
                TemplateEdit::SetNode {
                    node: 0,
                    kind: TemplateNodeKind::Dynamic(DynamicKind::Suspense {
                        mode: SuspenseMode::Ready { wake_after: 0 },
                    }),
                },
            ),
            Op::Rerender,
            Op::wake_suspense(253),
            Op::suspense(25, SuspenseMode::Pending),
            Op::Rerender,
        ]);
    }

    #[test]
    fn nested_ready_suspense_promotes_parent_after_last_wake() {
        replay_ops([
            Op::template(
                0,
                TemplateEdit::SetNode {
                    node: 0,
                    kind: TemplateNodeKind::Dynamic(DynamicKind::Suspense {
                        mode: SuspenseMode::Ready { wake_after: 0 },
                    }),
                },
            ),
            Op::Rerender,
            Op::template(
                1,
                TemplateEdit::SetNode {
                    node: 0,
                    kind: TemplateNodeKind::Dynamic(DynamicKind::Suspense {
                        mode: SuspenseMode::Ready { wake_after: 29 },
                    }),
                },
            ),
            Op::Rerender,
            Op::wake_suspense(3),
            Op::Rerender,
            Op::wake_suspense(108),
            Op::wake_suspense(235),
        ]);
    }

    #[test]
    fn nested_suspense_mode_change_to_resolved_replaces_visible_fallback() {
        replay_ops([
            Op::template(
                0,
                TemplateEdit::SetNode {
                    node: 0,
                    kind: TemplateNodeKind::Dynamic(DynamicKind::Suspense {
                        mode: SuspenseMode::Resolved,
                    }),
                },
            ),
            Op::template(
                1,
                TemplateEdit::Roots {
                    edit: ListEdit::Insert {
                        index: 224,
                        item: TemplateNodeKind::Dynamic(DynamicKind::Suspense {
                            mode: SuspenseMode::Ready { wake_after: 141 },
                        }),
                    },
                },
            ),
            Op::suspense_wake_mutation(25, WakeMutationSpec::None),
            Op::wake_suspense(187),
            Op::Rerender,
            Op::suspense(129, SuspenseMode::Resolved),
            Op::Rerender,
        ]);
    }

    #[test]
    fn hidden_fragment_shape_change_does_not_use_removed_child_mount_as_anchor() {
        replay_ops([
            Op::Rerender,
            Op::Rerender,
            Op::Rerender,
            Op::template(
                0,
                TemplateEdit::SetNode {
                    node: 0,
                    kind: TemplateNodeKind::Dynamic(DynamicKind::Suspense {
                        mode: SuspenseMode::Ready { wake_after: 0 },
                    }),
                },
            ),
            Op::Rerender,
            Op::fire_event(9, EventBehaviorSpec::Noop),
            Op::fire_event(71, EventBehaviorSpec::Noop),
            Op::template(
                1,
                TemplateEdit::SetNode {
                    node: 1,
                    kind: TemplateNodeKind::Dynamic(DynamicKind::Fragment {
                        children: 3,
                        key_base: Some(53),
                    }),
                },
            ),
            Op::Rerender,
            Op::fragment(
                1,
                61,
                FragmentEdit::Children(ListEdit::Insert {
                    index: 39,
                    item: None,
                }),
            ),
            Op::Rerender,
            Op::template(
                2,
                TemplateEdit::SetNode {
                    node: 0,
                    kind: TemplateNodeKind::Dynamic(DynamicKind::ComponentA),
                },
            ),
            Op::template(
                1,
                TemplateEdit::SetNode {
                    node: 1,
                    kind: TemplateNodeKind::Dynamic(DynamicKind::Fragment {
                        children: 2,
                        key_base: None,
                    }),
                },
            ),
            Op::Rerender,
            Op::Rerender,
        ]);
    }

    #[test]
    fn keyed_splice_placement_skips_mounts_claimed_by_earlier_splices() {
        replay_ops([
            Op::fire_event(90, EventBehaviorSpec::Noop),
            Op::template(
                0,
                TemplateEdit::SetNode {
                    node: 0,
                    kind: TemplateNodeKind::Dynamic(DynamicKind::Suspense {
                        mode: SuspenseMode::Ready { wake_after: 0 },
                    }),
                },
            ),
            Op::template(
                1,
                TemplateEdit::Children {
                    element: 0,
                    edit: ListEdit::Insert {
                        index: 0,
                        item: TemplateNodeKind::Dynamic(DynamicKind::Fragment {
                            children: 204,
                            key_base: Some(173),
                        }),
                    },
                },
            ),
            Op::wake_suspense(141),
            Op::Rerender,
            Op::template(
                101,
                TemplateEdit::Children {
                    element: 204,
                    edit: ListEdit::Insert {
                        index: 240,
                        item: TemplateNodeKind::Text(252),
                    },
                },
            ),
            Op::suspense(0, SuspenseMode::Ready { wake_after: 40 }),
            Op::wake_suspense(145),
            Op::fragment(
                1,
                0,
                FragmentEdit::Children(ListEdit::Move { from: 229, to: 24 }),
            ),
            Op::Rerender,
            Op::wake_suspense(187),
            Op::wake_suspense(88),
            Op::fire_event(16, EventBehaviorSpec::DispatchNestedEvent { target: 235 }),
            Op::Rerender,
        ]);
    }

    #[test]
    fn hidden_dynamic_replacement_does_not_resolve_renderer_placement() {
        replay_ops([
            Op::template(
                0,
                TemplateEdit::SetNode {
                    node: 255,
                    kind: TemplateNodeKind::Dynamic(DynamicKind::Suspense {
                        mode: SuspenseMode::Pending,
                    }),
                },
            ),
            Op::wake_suspense(122),
            Op::Rerender,
            Op::template(
                91,
                TemplateEdit::Children {
                    element: 201,
                    edit: ListEdit::Insert {
                        index: 41,
                        item: TemplateNodeKind::Dynamic(DynamicKind::Portal),
                    },
                },
            ),
            Op::Rerender,
            Op::template(
                1,
                TemplateEdit::SetNode {
                    node: 1,
                    kind: TemplateNodeKind::Dynamic(DynamicKind::ComponentA),
                },
            ),
            Op::Rerender,
            Op::Rerender,
        ]);
    }

    #[test]
    fn keyed_splice_placement_skips_mounts_removed_by_earlier_splices() {
        replay_ops([
            Op::template(
                0,
                TemplateEdit::SetNode {
                    node: 0,
                    kind: TemplateNodeKind::Dynamic(DynamicKind::Empty),
                },
            ),
            Op::fragment(
                0,
                0,
                FragmentEdit::Children(ListEdit::Insert {
                    index: 0,
                    item: Some(17),
                }),
            ),
            Op::fire_event(127, EventBehaviorSpec::Noop),
            Op::fragment(
                0,
                0,
                FragmentEdit::Children(ListEdit::Insert {
                    index: 0,
                    item: None,
                }),
            ),
            Op::fire_event(126, EventBehaviorSpec::DispatchNestedEvent { target: 52 }),
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
                173,
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
                FragmentEdit::Children(ListEdit::Move { from: 230, to: 110 }),
            ),
            Op::Rerender,
            Op::template(
                0,
                TemplateEdit::Attrs {
                    element: 45,
                    edit: ListEdit::Remove { index: 204 },
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
                202,
                0,
                FragmentEdit::Children(ListEdit::Insert {
                    index: 0,
                    item: Some(254),
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
            Op::wake_suspense(235),
            Op::fragment(
                0,
                0,
                FragmentEdit::Children(ListEdit::Insert {
                    index: 195,
                    item: Some(162),
                }),
            ),
            Op::template(
                0,
                TemplateEdit::Attrs {
                    element: 169,
                    edit: ListEdit::Remove { index: 161 },
                },
            ),
            Op::dynamic(1, 0, DynamicKind::ComponentA),
            Op::dynamic(2, 0, DynamicKind::Placeholder),
            Op::dynamic(3, 0, DynamicKind::ComponentA),
            Op::dynamic(4, 0, DynamicKind::ComponentA),
            Op::dynamic(6, 0, DynamicKind::ComponentA),
            Op::wake_suspense(186),
            Op::Rerender,
            Op::dynamic(9, 0, DynamicKind::Placeholder),
            Op::dynamic(10, 0, DynamicKind::ComponentA),
            Op::Rerender,
            Op::wake_suspense(253),
            Op::suspense(66, SuspenseMode::Pending),
            Op::Rerender,
            Op::dynamic(15, 0, DynamicKind::ComponentA),
            Op::dynamic(
                16,
                0,
                DynamicKind::Fragment {
                    children: 31,
                    key_base: Some(236),
                },
            ),
            Op::suspense_wake_mutation(54, WakeMutationSpec::None),
            Op::dynamic_attrs(18, 108, ListEdit::Remove { index: 72 }),
            Op::Rerender,
        ]);
    }

    #[test]
    fn keyed_splice_inner_replacement_keeps_future_siblings_available_for_placement() {
        replay_ops([
            Op::template(
                0,
                TemplateEdit::SetNode {
                    node: 0,
                    kind: TemplateNodeKind::Dynamic(DynamicKind::Empty),
                },
            ),
            Op::template(
                2,
                TemplateEdit::Children {
                    element: 214,
                    edit: ListEdit::Remove { index: 215 },
                },
            ),
            Op::suspense_wake_mutation(251, WakeMutationSpec::PrependStaticRoot { tag: 30 }),
            Op::Rerender,
            Op::suspense(249, SuspenseMode::Ready { wake_after: 140 }),
            Op::fragment(
                0,
                0,
                FragmentEdit::Children(ListEdit::Insert {
                    index: 0,
                    item: None,
                }),
            ),
            Op::fire_event(191, EventBehaviorSpec::DispatchNestedEvent { target: 60 }),
            Op::fragment(
                0,
                0,
                FragmentEdit::KeyMode(FragmentKeyMode::Keyed { base: 166 }),
            ),
            Op::template(
                137,
                TemplateEdit::SetNode {
                    node: 0,
                    kind: TemplateNodeKind::Text(76),
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
            Op::suspense_wake_mutation(38, WakeMutationSpec::None),
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
                FragmentEdit::Children(ListEdit::Insert {
                    index: 0,
                    item: None,
                }),
            ),
            Op::template(
                0,
                TemplateEdit::Attrs {
                    element: 197,
                    edit: ListEdit::Insert {
                        index: 32,
                        item: TemplateAttrSpec::Static {
                            name: 198,
                            value: 204,
                            namespace: None,
                        },
                    },
                },
            ),
            Op::fragment(
                0,
                253,
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
            Op::Rerender,
            Op::fragment(
                0,
                0,
                FragmentEdit::Children(ListEdit::Insert {
                    index: 0,
                    item: None,
                }),
            ),
            Op::fragment(
                1,
                225,
                FragmentEdit::KeyMode(FragmentKeyMode::Keyed { base: 209 }),
            ),
            Op::template(
                2,
                TemplateEdit::Roots {
                    edit: ListEdit::Move { from: 142, to: 169 },
                },
            ),
            Op::dynamic(3, 0, DynamicKind::ComponentA),
            Op::wake_suspense(11),
            Op::template(
                5,
                TemplateEdit::Attrs {
                    element: 20,
                    edit: ListEdit::Insert {
                        index: 245,
                        item: TemplateAttrSpec::Static {
                            name: 19,
                            value: 106,
                            namespace: Some(62),
                        },
                    },
                },
            ),
            Op::fire_event(15, EventBehaviorSpec::DispatchNestedEvent { target: 65 }),
            Op::template(
                230,
                TemplateEdit::Attrs {
                    element: 139,
                    edit: ListEdit::Move { from: 77, to: 92 },
                },
            ),
            Op::dynamic(8, 0, DynamicKind::ComponentA),
            Op::fire_event(238, EventBehaviorSpec::Noop),
            Op::wake_suspense(40),
            Op::template(
                11,
                TemplateEdit::Children {
                    element: 171,
                    edit: ListEdit::Insert {
                        index: 112,
                        item: TemplateNodeKind::Dynamic(DynamicKind::Empty),
                    },
                },
            ),
            Op::dynamic(12, 0, DynamicKind::Portal),
            Op::dynamic(13, 0, DynamicKind::ComponentA),
            Op::template(
                14,
                TemplateEdit::SetNode {
                    node: 26,
                    kind: TemplateNodeKind::Text(50),
                },
            ),
            Op::dynamic(15, 0, DynamicKind::Placeholder),
            Op::Rerender,
        ]);
    }

    #[test]
    fn placement_sibling_scan_uses_committed_fragment_shape() {
        replay_ops([
            Op::template(
                0,
                TemplateEdit::SetNode {
                    node: 0,
                    kind: TemplateNodeKind::Dynamic(DynamicKind::ComponentB),
                },
            ),
            Op::template(
                27,
                TemplateEdit::Roots {
                    edit: ListEdit::Insert {
                        index: 158,
                        item: TemplateNodeKind::Dynamic(DynamicKind::Suspense {
                            mode: SuspenseMode::Ready { wake_after: 170 },
                        }),
                    },
                },
            ),
            Op::Rerender,
            Op::template(
                2,
                TemplateEdit::Roots {
                    edit: ListEdit::Insert {
                        index: 204,
                        item: TemplateNodeKind::Dynamic(DynamicKind::Text(204)),
                    },
                },
            ),
            Op::template(
                170,
                TemplateEdit::Roots {
                    edit: ListEdit::Insert {
                        index: 224,
                        item: TemplateNodeKind::Dynamic(DynamicKind::Fragment {
                            children: 172,
                            key_base: None,
                        }),
                    },
                },
            ),
            Op::fire_event(72, EventBehaviorSpec::Noop),
            Op::Rerender,
            Op::Rerender,
            Op::template(
                5,
                TemplateEdit::SetNode {
                    node: 0,
                    kind: TemplateNodeKind::Dynamic(DynamicKind::ComponentB),
                },
            ),
            Op::template(
                2,
                TemplateEdit::SetNode {
                    node: 1,
                    kind: TemplateNodeKind::Dynamic(DynamicKind::ComponentB),
                },
            ),
            Op::Rerender,
            Op::template(
                40,
                TemplateEdit::Attrs {
                    element: 117,
                    edit: ListEdit::Move { from: 239, to: 214 },
                },
            ),
            Op::Rerender,
            Op::template(
                2,
                TemplateEdit::Roots {
                    edit: ListEdit::Move { from: 236, to: 168 },
                },
            ),
            Op::wake_suspense(50),
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
                    kind: TemplateNodeKind::Dynamic(DynamicKind::Empty),
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
                    kind: TemplateNodeKind::Dynamic(DynamicKind::Empty),
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
            Op::wake_suspense(167),
            Op::Rerender,
            Op::suspense(245, SuspenseMode::Ready { wake_after: 0 }),
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
                    kind: TemplateNodeKind::Dynamic(DynamicKind::Empty),
                },
            ),
            Op::dynamic(109, 211, DynamicKind::ComponentB),
            Op::template(
                1,
                TemplateEdit::SetNode {
                    node: 0,
                    kind: TemplateNodeKind::Dynamic(DynamicKind::Empty),
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
                    kind: TemplateNodeKind::Dynamic(DynamicKind::Empty),
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
                        item: TemplateNodeKind::Dynamic(DynamicKind::Empty),
                    },
                },
            ),
            Op::Rerender,
            Op::dynamic(3, 0, DynamicKind::ComponentB),
            Op::suspense(124, SuspenseMode::Resolved),
            Op::Rerender,
            Op::suspense(23, SuspenseMode::Ready { wake_after: 0 }),
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
                    kind: TemplateNodeKind::Dynamic(DynamicKind::Empty),
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
                    kind: TemplateNodeKind::Dynamic(DynamicKind::Empty),
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
            Op::suspense(0, SuspenseMode::Ready { wake_after: 0 }),
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
                    kind: TemplateNodeKind::Dynamic(DynamicKind::Empty),
                },
            ),
            Op::dynamic(
                0,
                0,
                DynamicKind::Suspense {
                    mode: SuspenseMode::Ready { wake_after: 0 },
                },
            ),
            Op::template(
                1,
                TemplateEdit::SetNode {
                    node: 0,
                    kind: TemplateNodeKind::Dynamic(DynamicKind::Empty),
                },
            ),
            Op::dynamic(1, 0, DynamicKind::ComponentA),
            Op::Rerender,
            Op::wake_suspense(0),
            Op::dynamic(
                1,
                0,
                DynamicKind::Suspense {
                    mode: SuspenseMode::Ready { wake_after: 0 },
                },
            ),
            Op::suspense(0, SuspenseMode::Ready { wake_after: 0 }),
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
                    kind: TemplateNodeKind::Dynamic(DynamicKind::Empty),
                },
            ),
            Op::dynamic(
                0,
                0,
                DynamicKind::Suspense {
                    mode: SuspenseMode::Ready { wake_after: 0 },
                },
            ),
            Op::template(
                1,
                TemplateEdit::SetNode {
                    node: 0,
                    kind: TemplateNodeKind::Dynamic(DynamicKind::Empty),
                },
            ),
            Op::dynamic(
                15,
                170,
                DynamicKind::Suspense {
                    mode: SuspenseMode::Ready { wake_after: 0 },
                },
            ),
            Op::template(
                2,
                TemplateEdit::SetNode {
                    node: 0,
                    kind: TemplateNodeKind::Dynamic(DynamicKind::Empty),
                },
            ),
            Op::dynamic(2, 0, DynamicKind::ComponentA),
            Op::suspense(83, SuspenseMode::Pending),
            Op::wake_suspense(0),
            Op::Rerender,
            Op::suspense(204, SuspenseMode::Ready { wake_after: 0 }),
            Op::Rerender,
            Op::wake_suspense(2),
            Op::suspense(31, SuspenseMode::Ready { wake_after: 0 }),
            Op::Rerender,
            Op::Rerender,
            Op::suspense(2, SuspenseMode::Ready { wake_after: 0 }),
            Op::wake_suspense(0),
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
                    kind: TemplateNodeKind::Dynamic(DynamicKind::Empty),
                },
            ),
            Op::dynamic(
                0,
                0,
                DynamicKind::Suspense {
                    mode: SuspenseMode::Ready { wake_after: 0 },
                },
            ),
            Op::template(
                1,
                TemplateEdit::SetNode {
                    node: 0,
                    kind: TemplateNodeKind::Dynamic(DynamicKind::Empty),
                },
            ),
            Op::Rerender,
            Op::dynamic(1, 0, DynamicKind::ComponentB),
            Op::Rerender,
            Op::wake_suspense(164),
            Op::dynamic(0, 0, DynamicKind::ComponentB),
            Op::dynamic(
                0,
                0,
                DynamicKind::Suspense {
                    mode: SuspenseMode::Ready { wake_after: 0 },
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
                    kind: TemplateNodeKind::Dynamic(DynamicKind::Empty),
                },
            ),
            Op::Rerender,
            Op::Rerender,
            Op::wake_suspense(104),
            Op::dynamic(
                0,
                0,
                DynamicKind::Suspense {
                    mode: SuspenseMode::Ready { wake_after: 0 },
                },
            ),
            Op::template(
                1,
                TemplateEdit::SetNode {
                    node: 0,
                    kind: TemplateNodeKind::Dynamic(DynamicKind::Empty),
                },
            ),
            Op::wake_suspense(94),
            Op::Rerender,
            Op::suspense(50, SuspenseMode::Ready { wake_after: 0 }),
            Op::Rerender,
            Op::wake_suspense(120),
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
                    kind: TemplateNodeKind::Dynamic(DynamicKind::Empty),
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
                    kind: TemplateNodeKind::Dynamic(DynamicKind::Empty),
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
                    kind: TemplateNodeKind::Dynamic(DynamicKind::Empty),
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
                        item: TemplateAttrSpec::Dynamic(Vec::new()),
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
                .borrow()
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
                        item: TemplateAttrSpec::Dynamic(Vec::new()),
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
                .borrow()
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
                    kind: TemplateNodeKind::Dynamic(DynamicKind::Empty),
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
                    kind: TemplateNodeKind::Dynamic(DynamicKind::Empty),
                },
            ),
            Op::dynamic(
                7,
                0,
                DynamicKind::Suspense {
                    mode: SuspenseMode::Ready { wake_after: 0 },
                },
            ),
            Op::Rerender,
            Op::suspense(0, SuspenseMode::Pending),
            Op::template(
                7,
                TemplateEdit::Roots {
                    edit: ListEdit::Insert {
                        index: 0,
                        item: TemplateNodeKind::Dynamic(DynamicKind::Empty),
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
                    kind: TemplateNodeKind::Dynamic(DynamicKind::Empty),
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
                    kind: TemplateNodeKind::Dynamic(DynamicKind::Empty),
                },
            ),
            Op::dynamic(
                7,
                0,
                DynamicKind::Suspense {
                    mode: SuspenseMode::Ready { wake_after: 0 },
                },
            ),
            Op::Rerender,
            Op::suspense(0, SuspenseMode::Pending),
            Op::template(
                7,
                TemplateEdit::Roots {
                    edit: ListEdit::Insert {
                        index: 0,
                        item: TemplateNodeKind::Dynamic(DynamicKind::Empty),
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
                        item: TemplateNodeKind::Dynamic(DynamicKind::Empty),
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
                        item: TemplateNodeKind::Dynamic(DynamicKind::Empty),
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
                    kind: TemplateNodeKind::Dynamic(DynamicKind::Empty),
                },
            ),
            Op::dynamic(
                7,
                0,
                DynamicKind::Suspense {
                    mode: SuspenseMode::Ready { wake_after: 0 },
                },
            ),
            Op::Rerender,
            Op::suspense(0, SuspenseMode::Ready { wake_after: 0 }),
            Op::Rerender,
            Op::template(
                0,
                TemplateEdit::Attrs {
                    element: 0,
                    edit: ListEdit::Insert {
                        index: 0,
                        item: TemplateAttrSpec::Dynamic(Vec::new()),
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
                        item: TemplateNodeKind::Dynamic(DynamicKind::Empty),
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
    fn waker_wake_unmounted_ready_suspense_is_noop() {
        let ops = [
            Op::template(
                3,
                TemplateEdit::Children {
                    element: 0,
                    edit: ListEdit::Insert {
                        index: 5,
                        item: TemplateNodeKind::Dynamic(DynamicKind::Empty),
                    },
                },
            ),
            Op::dynamic(
                5,
                2,
                DynamicKind::Suspense {
                    mode: SuspenseMode::Ready { wake_after: 0 },
                },
            ),
            Op::wake_suspense(3),
        ];

        let mut harness = Harness::fresh();
        for op in ops {
            apply_op(&mut harness, &op).unwrap();
        }
    }

    #[test]
    fn waker_wake_after_unrendered_parent_edit_matches_fresh_model() {
        let ops = [
            Op::template(
                2,
                TemplateEdit::Roots {
                    edit: ListEdit::Insert {
                        index: 4,
                        item: TemplateNodeKind::Dynamic(DynamicKind::Empty),
                    },
                },
            ),
            Op::dynamic(
                6,
                4,
                DynamicKind::Suspense {
                    mode: SuspenseMode::Ready { wake_after: 0 },
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
            Op::wake_suspense(0),
            Op::Rerender,
        ];

        let mut harness = Harness::fresh();
        for op in ops {
            apply_op(&mut harness, &op).unwrap();
        }
    }

    #[test]
    fn waker_wake_nested_suspense_applies_hidden_wake_mutation() {
        let ops = [
            Op::template(
                0,
                TemplateEdit::SetNode {
                    node: 0,
                    kind: TemplateNodeKind::Dynamic(DynamicKind::Empty),
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
                    kind: TemplateNodeKind::Dynamic(DynamicKind::Empty),
                },
            ),
            Op::dynamic(
                7,
                0,
                DynamicKind::Suspense {
                    mode: SuspenseMode::Ready { wake_after: 0 },
                },
            ),
            Op::suspense_wake_mutation(1, WakeMutationSpec::PrependStaticRoot { tag: 42 }),
            Op::Rerender,
            Op::suspense(0, SuspenseMode::Ready { wake_after: 0 }),
            Op::Rerender,
            Op::wake_suspense(1),
            Op::wake_suspense(0),
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
                    kind: TemplateNodeKind::Dynamic(DynamicKind::Empty),
                },
            ),
            Op::dynamic(
                0,
                0,
                DynamicKind::Suspense {
                    mode: SuspenseMode::Ready { wake_after: 0 },
                },
            ),
            Op::Rerender,
            Op::template(
                1,
                TemplateEdit::SetNode {
                    node: 0,
                    kind: TemplateNodeKind::Dynamic(DynamicKind::Empty),
                },
            ),
            Op::dynamic(
                1,
                0,
                DynamicKind::Suspense {
                    mode: SuspenseMode::Ready { wake_after: 0 },
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
                    kind: TemplateNodeKind::Dynamic(DynamicKind::Empty),
                },
            ),
            Op::Rerender,
            Op::dynamic(
                109,
                103,
                DynamicKind::Suspense {
                    mode: SuspenseMode::Ready { wake_after: 0 },
                },
            ),
            Op::Rerender,
            Op::Rerender,
            Op::wake_suspense(34),
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
                        item: TemplateNodeKind::Dynamic(DynamicKind::Empty),
                    },
                },
            ),
            Op::template(
                0,
                TemplateEdit::Roots {
                    edit: ListEdit::Insert {
                        index: 0,
                        item: TemplateNodeKind::Dynamic(DynamicKind::Empty),
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
                        item: TemplateAttrSpec::Dynamic(Vec::new()),
                    },
                },
            ),
            Op::template(
                0,
                TemplateEdit::Attrs {
                    element: 0,
                    edit: ListEdit::Insert {
                        index: 0,
                        item: TemplateAttrSpec::Dynamic(Vec::new()),
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
                        item: TemplateAttrSpec::Dynamic(Vec::new()),
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
    fn removing_dynamic_attr_restores_last_duplicate_static_template_attr() {
        replay_ops([
            Op::template(
                0,
                TemplateEdit::Attrs {
                    element: 0,
                    edit: ListEdit::Insert {
                        index: 0,
                        item: TemplateAttrSpec::Dynamic(vec![AttrSpec {
                            name: 13,
                            namespace: None,
                            value: AttrValueSpec::Text(0),
                            volatile: false,
                        }]),
                    },
                },
            ),
            Op::template(
                0,
                TemplateEdit::Attrs {
                    element: 0,
                    edit: ListEdit::Insert {
                        index: 0,
                        item: TemplateAttrSpec::Static {
                            name: 13,
                            value: 254,
                            namespace: None,
                        },
                    },
                },
            ),
            Op::template(
                0,
                TemplateEdit::Attrs {
                    element: 0,
                    edit: ListEdit::Insert {
                        index: 0,
                        item: TemplateAttrSpec::Static {
                            name: 13,
                            value: 190,
                            namespace: None,
                        },
                    },
                },
            ),
            Op::Rerender,
            Op::dynamic_attrs(136, 0, ListEdit::Remove { index: 0 }),
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
                        item: TemplateAttrSpec::Dynamic(Vec::new()),
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
                        item: TemplateAttrSpec::Dynamic(Vec::new()),
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
                        item: TemplateAttrSpec::Dynamic(Vec::new()),
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
                        item: TemplateAttrSpec::Dynamic(Vec::new()),
                    },
                },
            ),
            Op::template(
                0,
                TemplateEdit::Attrs {
                    element: 0,
                    edit: ListEdit::Insert {
                        index: 0,
                        item: TemplateAttrSpec::Dynamic(Vec::new()),
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
                    kind: TemplateNodeKind::Dynamic(DynamicKind::Empty),
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
                    kind: TemplateNodeKind::Dynamic(DynamicKind::Empty),
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
                    kind: TemplateNodeKind::Dynamic(DynamicKind::Empty),
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
                    mode: SuspenseMode::Ready { wake_after: 0 },
                },
            ),
            Op::template(
                7,
                TemplateEdit::SetNode {
                    node: 7,
                    kind: TemplateNodeKind::Dynamic(DynamicKind::Empty),
                },
            ),
            Op::template(
                88,
                TemplateEdit::SetNode {
                    node: 6,
                    kind: TemplateNodeKind::Dynamic(DynamicKind::Empty),
                },
            ),
            Op::template(
                0,
                TemplateEdit::Children {
                    element: 1,
                    edit: ListEdit::Insert {
                        index: 5,
                        item: TemplateNodeKind::Dynamic(DynamicKind::Empty),
                    },
                },
            ),
            Op::dynamic(4, 2, DynamicKind::ComponentB),
            Op::wake_suspense(120),
            Op::dynamic(
                1,
                5,
                DynamicKind::Suspense {
                    mode: SuspenseMode::Ready { wake_after: 0 },
                },
            ),
            Op::template(
                6,
                TemplateEdit::SetNode {
                    node: 7,
                    kind: TemplateNodeKind::Dynamic(DynamicKind::Empty),
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
                        item: TemplateNodeKind::Dynamic(DynamicKind::Empty),
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
                        item: TemplateNodeKind::Dynamic(DynamicKind::Empty),
                    },
                },
            ),
            Op::suspense(4, SuspenseMode::Resolved),
            Op::dynamic(
                3,
                2,
                DynamicKind::Suspense {
                    mode: SuspenseMode::Ready { wake_after: 0 },
                },
            ),
            Op::Rerender,
            Op::suspense(0, SuspenseMode::Ready { wake_after: 0 }),
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
                    kind: TemplateNodeKind::Dynamic(DynamicKind::Empty),
                },
            ),
            Op::dynamic(
                0,
                0,
                DynamicKind::Suspense {
                    mode: SuspenseMode::Ready { wake_after: 0 },
                },
            ),
            Op::template(
                1,
                TemplateEdit::SetNode {
                    node: 0,
                    kind: TemplateNodeKind::Dynamic(DynamicKind::Empty),
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
                        item: TemplateNodeKind::Dynamic(DynamicKind::Empty),
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
            Op::suspense(31, SuspenseMode::Ready { wake_after: 0 }),
            Op::Rerender,
            Op::suspense(240, SuspenseMode::Ready { wake_after: 0 }),
            Op::wake_suspense(197),
        ];

        let mut harness = Harness::fresh_strict();
        for op in ops {
            apply_op(&mut harness, &op).unwrap();
        }
    }

    #[test]
    fn hidden_suspense_branch_retains_background_mode_recursively() {
        let ops = [
            Op::Rerender,
            Op::wake_suspense(166),
            Op::fire_event(2, EventBehaviorSpec::Noop),
            Op::Rerender,
            Op::template(
                0,
                TemplateEdit::SetNode {
                    node: 0,
                    kind: TemplateNodeKind::Dynamic(DynamicKind::Suspense {
                        mode: SuspenseMode::Ready { wake_after: 0 },
                    }),
                },
            ),
            Op::template(
                101,
                TemplateEdit::Children {
                    element: 204,
                    edit: ListEdit::Insert {
                        index: 240,
                        item: TemplateNodeKind::Dynamic(DynamicKind::Fragment {
                            children: 222,
                            key_base: Some(127),
                        }),
                    },
                },
            ),
            Op::Rerender,
            Op::template(
                1,
                TemplateEdit::SetNode {
                    node: 1,
                    kind: TemplateNodeKind::Dynamic(DynamicKind::Fragment {
                        children: 2,
                        key_base: Some(28),
                    }),
                },
            ),
            Op::Rerender,
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
                    kind: TemplateNodeKind::Dynamic(DynamicKind::Empty),
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
                    kind: TemplateNodeKind::Dynamic(DynamicKind::Empty),
                },
            ),
            Op::template(
                7,
                TemplateEdit::Children {
                    element: 0,
                    edit: ListEdit::Insert {
                        index: 0,
                        item: TemplateNodeKind::Dynamic(DynamicKind::Empty),
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
                    kind: TemplateNodeKind::Dynamic(DynamicKind::Empty),
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
                    kind: TemplateNodeKind::Dynamic(DynamicKind::Empty),
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
