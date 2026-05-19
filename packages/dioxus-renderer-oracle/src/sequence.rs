use crate::renderer::{EditSummary, OracleNodeId, RendererOracle};
use crate::vdom_snapshot::vdom_snapshot;
use dioxus_core::{consume_context, generation, Element, ScopeId, VNode, VirtualDom};
use std::rc::Rc;

/// The steps for a [`Sequence`], handed to the source app via a root context so
/// the dispatcher can pick the current state by `generation()`.
#[derive(Clone)]
struct SequenceSteps(Rc<Vec<StepSource>>);

/// The step a [`Sequence`]'s expected-side `VirtualDom` should render, passed in
/// via a root context so the same dispatch function works for both source and
/// expected sides.
#[derive(Clone)]
struct ExpectedStep(Rc<StepSource>);

/// Drive a `VirtualDom` through an ordered sequence of states. Each step is an
/// `rsx!` block that plays both roles: the content the source component renders
/// for that generation and the expected DOM the oracle asserts after rendering.
///
/// Usage:
///
/// ```ignore
/// Sequence::new()
///     .render(rsx! { div { "a" } })
///     .render(rsx! { div { "b" } })
///     .run();
/// ```
///
/// For parameterized steps, call a helper that returns `Element`:
///
/// ```ignore
/// fn divs(keys: &[i32]) -> Element { rsx! { for k in keys.iter().copied() { div { "{k}" } } } }
/// Sequence::new()
///     .render(divs(&[1, 2, 3]))
///     .render(divs(&[3, 2, 1]))
///     .run();
/// ```
///
/// The source app dispatches on `dioxus_core::generation()` to pick the current
/// step (cloned from a root context — no globals, no unsafe). Between steps
/// `Sequence` marks `ScopeId::APP` dirty and renders. The expected DOM is built
/// by walking the VNode tree of the same step in a throwaway `VirtualDom` —
/// independent of the renderer's mutation path.
/// How a step's source/expected content is produced.
///
/// `Static` is a pre-built `Element` — what `rsx!{...}` evaluates to outside any
/// runtime. Works for handler-free, signal-free content.
///
/// `Lazy` is a closure invoked inside the Dioxus runtime each time the step
/// renders. Required for rsx that creates event handlers, reads signals, or
/// otherwise needs runtime context to construct.
enum StepSource {
    Static(Element),
    Lazy(Box<dyn Fn() -> Element>),
}

impl StepSource {
    fn produce(&self) -> Element {
        match self {
            StepSource::Static(e) => e.clone(),
            StepSource::Lazy(f) => f(),
        }
    }
}

/// One entry in a [`Sequence`]'s timeline. Steps and callbacks interleave in
/// authoring order — there's no parallel-indexed second list.
enum SequenceItem {
    /// An expected DOM state. Doubles as the source content for that generation.
    Step(StepSource),
    /// A side-effect that runs in authoring position. Useful for firing synthetic
    /// events, reading context, or making side-channel assertions on the
    /// `VirtualDom` between renders. Receives the live oracle so that event
    /// targets can be resolved semantically (`oracle.element_id_by_tag(...)`,
    /// `oracle.element_id_by_attr(...)`) instead of by raw `ElementId(N)`
    /// literal.
    Then(Box<dyn FnMut(&mut VirtualDom, &RendererOracle)>),
}

/// An assertion registered against the [`EditSummary`] captured at a specific
/// step. `step` is the 0-indexed transition (step 0 = initial rebuild, step 1 =
/// first rerender, ...). The closure runs after the step's render completes and
/// is free to panic to signal failure.
struct EditSummaryAssertion {
    step: usize,
    check: Box<dyn Fn(&EditSummary)>,
}

#[must_use]
pub struct Sequence {
    items: Vec<SequenceItem>,
    identity_attr: Option<String>,
    edit_summary_assertions: Vec<EditSummaryAssertion>,
}

fn sequence_dispatch() -> Element {
    let steps = consume_context::<SequenceSteps>();
    let idx = generation().min(steps.0.len() - 1);
    steps.0[idx].produce()
}

fn expected_dispatch() -> Element {
    let step = consume_context::<ExpectedStep>();
    step.0.produce()
}

impl Sequence {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            identity_attr: None,
            edit_summary_assertions: Vec::new(),
        }
    }

    /// Append a state from a pre-built `rsx!` block. The same `Element` is cloned
    /// for the source-side render and for the expected-DOM comparison. Use this
    /// for handler-free, signal-free content.
    pub fn render(mut self, state: Element) -> Self {
        self.items
            .push(SequenceItem::Step(StepSource::Static(state)));
        self
    }

    /// Append a state from a closure that runs *inside* the Dioxus runtime each
    /// time the step renders. Use this when the rsx contains event handlers or
    /// reads signals — those constructions require an active runtime.
    pub fn render_with(mut self, state: impl Fn() -> Element + 'static) -> Self {
        self.items
            .push(SequenceItem::Step(StepSource::Lazy(Box::new(state))));
        self
    }

    /// Append a side-effect that runs in authoring position — between the
    /// previous step's assertion and the next step's `mark_dirty`. The closure
    /// receives both the `VirtualDom` and the oracle's current view of the DOM
    /// so that event targets can be resolved semantically:
    ///
    /// ```ignore
    /// Sequence::new()
    ///     .render(rsx! { button { onclick: ..., "click me" } })
    ///     .then(|dom, oracle| {
    ///         let btn = oracle.element_id_by_tag("button");
    ///         dom.runtime().handle_event("click", event, btn);
    ///     })
    ///     .render(rsx! { button { onclick: ..., "clicked once" } })
    ///     .run();
    /// ```
    pub fn then(mut self, action: impl FnMut(&mut VirtualDom, &RendererOracle) + 'static) -> Self {
        self.items.push(SequenceItem::Then(Box::new(action)));
        self
    }

    /// Track per-node DOM identity across renders by the value of an HTML
    /// attribute on each element. After each step, the oracle records the
    /// `attr_value -> OracleNodeId` mapping; values that appear in two
    /// consecutive steps must map to the *same* `OracleNodeId`, otherwise the
    /// renderer dropped-and-recreated a node that should have been moved.
    ///
    /// Use this on tests that need to assert keyed-diffing identity (animation,
    /// focus, scroll position preservation):
    ///
    /// ```ignore
    /// Sequence::new()
    ///     .track_identity_by("id")
    ///     .render_with(|| rsx! { div { id: "0", "first" } div { id: "1", "second" } })
    ///     .render_with(|| rsx! { div { id: "1", "second" } div { id: "0", "first" } })
    ///     .run();
    /// ```
    pub fn track_identity_by(mut self, attr: &str) -> Self {
        self.identity_attr = Some(attr.to_string());
        self
    }

    /// Register an assertion against the [`EditSummary`] captured for the render
    /// at `step` (0-indexed: step 0 is the initial rebuild, step 1 is the first
    /// rerender, ...). Use this to guard structural diff properties that
    /// final-DOM snapshots cannot see — minimal move counts, in-place patches,
    /// no-op rerenders:
    ///
    /// ```ignore
    /// Sequence::new()
    ///     .render(rsx! { for k in [0,1,2] { div { key: "{k}", id: "{k}" } } })
    ///     .render(rsx! { for k in [2,0,1] { div { key: "{k}", id: "{k}" } } })
    ///     .assert_edit_summary(1, |s| {
    ///         assert!(s.pushes <= 1, "expected one move, got {} pushes", s.pushes);
    ///         assert_eq!(s.creates(), 0);
    ///     })
    ///     .run();
    /// ```
    ///
    /// Multiple assertions for the same step are allowed and all run.
    pub fn assert_edit_summary(
        mut self,
        step: usize,
        check: impl Fn(&EditSummary) + 'static,
    ) -> Self {
        self.edit_summary_assertions.push(EditSummaryAssertion {
            step,
            check: Box::new(check),
        });
        self
    }

    /// Execute every item in order. Each `Step` renders the source and asserts
    /// the DOM matches; each `Then` runs its side-effect at that point in
    /// the timeline.
    pub fn run(mut self) {
        // Pull the steps into a shared list. Callbacks don't reach the source
        // VDom — they manipulate it externally between renders.
        let just_steps: Vec<Rc<StepSource>> = self
            .items
            .iter_mut()
            .filter_map(|item| match item {
                SequenceItem::Step(src) => {
                    // Replace the StepSource with a placeholder so we can move it
                    // out (Element is Clone but Box<dyn Fn> isn't); we'll share
                    // each step via Rc to allow both source and expected sides.
                    let taken = std::mem::replace(src, StepSource::Static(VNode::empty()));
                    Some(Rc::new(taken))
                }
                SequenceItem::Then(_) => None,
            })
            .collect();
        assert!(!just_steps.is_empty(), "Sequence needs at least one step");

        let source_steps: Vec<StepSource> = just_steps
            .iter()
            .map(|s| match s.as_ref() {
                StepSource::Static(e) => StepSource::Static(e.clone()),
                // For Lazy we share via Rc through ExpectedStep; the source side
                // gets its own clone of the Rc-wrapped closure too.
                StepSource::Lazy(_) => StepSource::Lazy(Box::new({
                    let shared = s.clone();
                    move || shared.produce()
                })),
            })
            .collect();
        let steps_ctx = SequenceSteps(Rc::new(source_steps));
        let mut dom = VirtualDom::new(sequence_dispatch).with_root_context(steps_ctx);
        let mut oracle = RendererOracle::new();
        let identity_attr = self.identity_attr.clone();
        let mut prev_identities: Option<Vec<(String, OracleNodeId)>> = None;
        let mut step_index = 0usize;
        let max_step = just_steps.len();
        for assertion in &self.edit_summary_assertions {
            assert!(
                assertion.step < max_step,
                "assert_edit_summary references step {} but the sequence only has {} step(s)",
                assertion.step,
                max_step,
            );
        }

        for item in &mut self.items {
            match item {
                SequenceItem::Step(_) => {
                    if step_index == 0 {
                        oracle.rebuild(&mut dom);
                    } else {
                        dom.mark_dirty(ScopeId::APP);
                        oracle.render(&mut dom);
                    }
                    assert_step(&oracle, &just_steps[step_index]);
                    if let Some(attr) = identity_attr.as_deref() {
                        let current = oracle.identities_by_attr(attr);
                        if let Some(prev) = prev_identities.as_deref() {
                            assert_identity_preserved(prev, &current, attr, step_index);
                        }
                        prev_identities = Some(current);
                    }
                    let summary = oracle.last_edit_summary();
                    for assertion in &self.edit_summary_assertions {
                        if assertion.step == step_index {
                            (assertion.check)(&summary);
                        }
                    }
                    step_index += 1;
                }
                SequenceItem::Then(action) => {
                    action(&mut dom, &oracle);
                }
            }
        }
    }
}

impl Default for Sequence {
    fn default() -> Self {
        Self::new()
    }
}

/// For each value that appears in both `prev` and `current`, assert that the
/// `OracleNodeId` is preserved. New values (added this step) and dropped values
/// (removed this step) are allowed; only common-value mismatches are a failure.
fn assert_identity_preserved(
    prev: &[(String, OracleNodeId)],
    current: &[(String, OracleNodeId)],
    attr: &str,
    step: usize,
) {
    use std::collections::HashMap;
    let prev_map: HashMap<&str, OracleNodeId> =
        prev.iter().map(|(k, v)| (k.as_str(), *v)).collect();
    for (value, current_id) in current {
        if let Some(prev_id) = prev_map.get(value.as_str()) {
            assert_eq!(
                *prev_id, *current_id,
                "step {step}: node identity for `{attr}={value}` was not preserved \
                 (previous OracleNodeId {prev_id:?}, current {current_id:?}). \
                 This means the renderer dropped and recreated the node when it should \
                 have moved it — any browser-side state (animations, focus, scroll) \
                 would be lost.",
            );
        }
    }
}

/// Compare the oracle's current DOM against the DOM produced by rendering `step`
/// directly. Builds a throwaway `VirtualDom` whose component invokes the step
/// (via root-context dispatch) so handler/signal-bearing rsx is constructed
/// inside the runtime.
fn assert_step(oracle: &RendererOracle, step: &Rc<StepSource>) {
    let mut tmp = VirtualDom::new(expected_dispatch).with_root_context(ExpectedStep(step.clone()));
    tmp.rebuild_in_place();
    let expected_snapshot = vdom_snapshot(&tmp);
    pretty_assertions::assert_eq!(
        oracle.snapshot(),
        expected_snapshot,
        "renderer DOM diverged from expected rsx tree"
    );
}
