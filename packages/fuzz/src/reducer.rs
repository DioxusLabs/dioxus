use crate::{
    FuzzCase, FuzzFailure, encode_case_vec,
    model::{
        AttrSpec, AttrValueSpec, DynamicKind, FragmentKeyMode, SuspenseMode, TemplateAttrSpec,
        TemplateNodeKind, WakeMutationSpec,
    },
    ops::{EventBehaviorSpec, FragmentEdit, ListEdit, ModelEdit, Op, SuspenseEdit, TemplateEdit},
    run_case,
};
use std::{
    collections::HashSet,
    hash::Hash,
    panic::{self, AssertUnwindSafe},
    sync::Mutex,
};

#[derive(Clone)]
pub struct ReductionOptions {
    random_multi_attempts: usize,
    max_attempts: Option<usize>,
}

impl ReductionOptions {
    pub fn random_multi_attempts(mut self, attempts: usize) -> Self {
        self.random_multi_attempts = attempts;
        self
    }

    pub fn max_attempts(mut self, attempts: usize) -> Self {
        self.max_attempts = Some(attempts);
        self
    }
}

impl Default for ReductionOptions {
    fn default() -> Self {
        Self {
            random_multi_attempts: 2048,
            max_attempts: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct FailureSignature {
    summary: String,
}

impl FailureSignature {
    fn new(failure: &FuzzFailure) -> Self {
        Self {
            summary: failure_summary(failure).to_string(),
        }
    }

    fn matches(&self, failure: &FuzzFailure) -> bool {
        self.summary == failure_summary(failure)
    }
}

struct Reducer {
    options: ReductionOptions,
    signature: FailureSignature,
    failing_step: usize,
    rng: ReductionRng,
    attempts: usize,
}

enum ReductionRun {
    Passed,
    Failed(FuzzFailure),
    Panicked,
}

pub(crate) fn reduce_case_to_encoded_vec(
    case: &FuzzCase,
    encoded_len: usize,
    max_size: usize,
    options: ReductionOptions,
) -> Option<Vec<u8>> {
    let original_failure = match run_case_for_reduction(case) {
        ReductionRun::Failed(failure) => failure,
        ReductionRun::Passed | ReductionRun::Panicked => return None,
    };
    let original_ops = case.ops.len();
    let signature = FailureSignature::new(&original_failure);
    let mut reducer = Reducer {
        options,
        signature,
        failing_step: original_failure.step,
        rng: ReductionRng::new(seed_from_case(case)),
        attempts: 0,
    };
    let mut case = case.clone_case();

    reducer.truncate_after_failure(&mut case);
    reducer.reduce_to_local_minimum(&mut case);
    reducer.reduce_by_random_multistep(&mut case);
    reducer.reduce_to_local_minimum(&mut case);
    reducer.reduce_by_random_multistep(&mut case);
    reducer.reduce_to_local_minimum(&mut case);

    let encoded = encode_case_vec(&case)?;
    let reduced_ops = case.ops.len() < original_ops;
    let reduced_bytes = encoded.len() < encoded_len;

    (encoded.len() <= max_size && (reduced_ops || reduced_bytes)).then_some(encoded)
}

impl Reducer {
    fn reduce_to_local_minimum(&mut self, case: &mut FuzzCase) {
        self.reduce_by_chunk_deletion(case);
        self.reduce_by_single_deletion(case);
        self.reduce_values(case);
        self.reduce_by_peepholes(case);
    }

    fn accepts(&mut self, case: &FuzzCase) -> Option<FuzzFailure> {
        if self
            .options
            .max_attempts
            .is_some_and(|max_attempts| self.attempts >= max_attempts)
        {
            return None;
        }

        self.attempts += 1;
        let ReductionRun::Failed(failure) = run_case_for_reduction(case) else {
            return None;
        };
        if self.signature.matches(&failure) {
            Some(failure)
        } else {
            None
        }
    }

    fn try_replace(&mut self, case: &mut FuzzCase, mut candidate: FuzzCase) -> bool {
        let Some(failure) = self.accepts(&candidate) else {
            return false;
        };
        candidate.ops.truncate(failure.step + 1);
        *case = candidate;
        self.failing_step = failure.step;
        true
    }

    fn truncate_after_failure(&mut self, case: &mut FuzzCase) {
        let needed_len = self.failing_step + 1;
        if needed_len >= case.ops.len() {
            return;
        }

        let mut candidate = case.clone_case();
        candidate.ops.truncate(needed_len);
        self.try_replace(case, candidate);
    }

    fn reduce_by_chunk_deletion(&mut self, case: &mut FuzzCase) {
        let mut granularity = 2;

        while case.ops.len() > 1 {
            let len = case.ops.len();
            let chunk_size = len.div_ceil(granularity);
            let mut changed = false;
            let mut start = 0;

            while start < case.ops.len() {
                let end = (start + chunk_size).min(case.ops.len());
                if start == 0 && end == case.ops.len() {
                    break;
                }

                if self.try_remove_range(case, start, end) {
                    changed = true;
                } else {
                    start = end;
                }
            }

            if changed {
                granularity = 2;
            } else if granularity >= len {
                break;
            } else {
                granularity = (granularity * 2).min(len);
            }
        }
    }

    fn reduce_by_single_deletion(&mut self, case: &mut FuzzCase) {
        let mut index = 0;
        while index < case.ops.len() {
            if !self.try_remove_range(case, index, index + 1) {
                index += 1;
            }
        }
    }

    fn try_remove_range(&mut self, case: &mut FuzzCase, start: usize, end: usize) -> bool {
        if start >= end || end > case.ops.len() || end - start == case.ops.len() {
            return false;
        }

        let mut ops = Vec::with_capacity(case.ops.len() - (end - start));
        ops.extend_from_slice(&case.ops[..start]);
        ops.extend_from_slice(&case.ops[end..]);
        self.try_replace(case, FuzzCase::new(ops))
    }

    fn reduce_values(&mut self, case: &mut FuzzCase) {
        let mut index = 0;
        while index < case.ops.len() {
            let candidates = simplified_ops(&case.ops[index]);
            let mut changed = false;
            for replacement in candidates {
                let mut candidate = case.clone_case();
                candidate.ops[index] = replacement;
                if self.try_replace(case, candidate) {
                    changed = true;
                    break;
                }
            }

            if !changed {
                index += 1;
            }
        }
    }

    fn reduce_by_peepholes(&mut self, case: &mut FuzzCase) {
        loop {
            let mut changed = false;
            for index in 0..case.ops.len() {
                for candidate in peephole_cases(case, index) {
                    if self.try_replace(case, candidate) {
                        changed = true;
                        break;
                    }
                }
                if changed {
                    break;
                }
            }

            if !changed {
                break;
            }
        }
    }

    fn reduce_by_random_multistep(&mut self, case: &mut FuzzCase) {
        for _ in 0..self.options.random_multi_attempts {
            if case.ops.len() <= 1 {
                return;
            }

            let mut candidate = case.clone_case();
            let changed =
                random_multistep_shrink_case_with(&mut candidate, |len| self.rng.index(len));

            if changed {
                self.try_replace(case, candidate);
            }
        }
    }
}

fn run_case_for_reduction(case: &FuzzCase) -> ReductionRun {
    static PANIC_HOOK_LOCK: Mutex<()> = Mutex::new(());

    let _lock = PANIC_HOOK_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let previous_hook = panic::take_hook();
    panic::set_hook(Box::new(|_| {}));
    let result = panic::catch_unwind(AssertUnwindSafe(|| run_case(case)));
    panic::set_hook(previous_hook);

    match result {
        Ok(Ok(())) => ReductionRun::Passed,
        Ok(Err(failure)) => ReductionRun::Failed(failure),
        Err(_) => ReductionRun::Panicked,
    }
}

fn failure_summary(failure: &FuzzFailure) -> &str {
    failure.message.lines().next().unwrap_or(&failure.message)
}

pub(crate) fn simplified_ops(op: &Op) -> Vec<Op> {
    let mut out = HashSet::new();
    if !matches!(op, Op::Rerender) {
        out.insert(Op::Rerender);
    }
    if !matches!(op, Op::RenderDirty) {
        out.insert(Op::RenderDirty);
    }
    if !matches!(op, Op::RenderSuspenseDirty) {
        out.insert(Op::RenderSuspenseDirty);
    }

    match op {
        Op::Rerender | Op::RenderDirty | Op::RenderSuspenseDirty => {}
        Op::WakeSuspense { suspense } => {
            for suspense in simpler_u8_values(*suspense) {
                out.insert(Op::wake_suspense(suspense));
            }
        }
        Op::FireEvent { target, behavior } => {
            for target in simpler_u8_values(*target) {
                out.insert(Op::fire_event(target, *behavior));
            }
            for behavior in simplified_event_behaviors(*behavior) {
                out.insert(Op::fire_event(*target, behavior));
            }
        }
        Op::Mutate(edit) => simplified_model_edit_ops(edit, &mut out),
    }

    out.into_iter().collect()
}

fn simplified_event_behaviors(behavior: EventBehaviorSpec) -> Vec<EventBehaviorSpec> {
    let mut out = HashSet::new();
    match behavior {
        EventBehaviorSpec::Noop => {}
        EventBehaviorSpec::DispatchNestedEvent { target } => {
            for target in simpler_u8_values(target) {
                out.insert(EventBehaviorSpec::DispatchNestedEvent { target });
            }
            out.insert(EventBehaviorSpec::Noop);
        }
        EventBehaviorSpec::ScheduleUpdate
        | EventBehaviorSpec::ScheduleUpdateAny
        | EventBehaviorSpec::NeedsUpdate
        | EventBehaviorSpec::NeedsUpdateAny
        | EventBehaviorSpec::ContextRoundTrip
        | EventBehaviorSpec::RootContextRoundTrip
        | EventBehaviorSpec::QueueEffect
        | EventBehaviorSpec::SpawnIsomorphic => {
            out.insert(EventBehaviorSpec::Noop);
        }
    }
    out.into_iter().collect()
}

fn simplified_model_edit_ops(edit: &ModelEdit, out: &mut HashSet<Op>) {
    match edit {
        ModelEdit::VNode { vnode, edit } => simplified_vnode_edit_ops(*vnode, edit, out),
        ModelEdit::Suspense { suspense, edit } => {
            for suspense in simpler_u8_values(*suspense) {
                out.insert(Op::Mutate(ModelEdit::Suspense {
                    suspense,
                    edit: *edit,
                }));
            }
            match edit {
                SuspenseEdit::Mode(mode) => {
                    for mode in simplified_suspense_modes(*mode) {
                        out.insert(Op::suspense(*suspense, mode));
                    }
                }
                SuspenseEdit::WakeMutation(mutation) => {
                    for mutation in simplified_wake_mutations(*mutation) {
                        out.insert(Op::suspense_wake_mutation(*suspense, mutation));
                    }
                }
            }
        }
    }
}

fn simplified_vnode_edit_ops(vnode: u8, edit: &TemplateEdit, out: &mut HashSet<Op>) {
    for simpler_vnode in simpler_u8_values(vnode) {
        out.insert(Op::Mutate(ModelEdit::VNode {
            vnode: simpler_vnode,
            edit: edit.clone(),
        }));
    }

    for edit in simplified_template_edits(edit) {
        out.insert(Op::template(vnode, edit));
    }
}

fn peephole_cases(case: &FuzzCase, index: usize) -> Vec<FuzzCase> {
    let mut out = Vec::new();
    fold_key_mode_into_previous_insert(case, index, &mut out);
    out
}

fn fold_key_mode_into_previous_insert(case: &FuzzCase, index: usize, out: &mut Vec<FuzzCase>) {
    if index == 0 {
        return;
    }

    let Op::Mutate(ModelEdit::VNode {
        vnode,
        edit:
            TemplateEdit::Fragment {
                node,
                edit: FragmentEdit::KeyMode(FragmentKeyMode::Keyed { base }),
            },
    }) = &case.ops[index]
    else {
        return;
    };

    let Op::Mutate(ModelEdit::VNode {
        vnode: previous_vnode,
        edit:
            TemplateEdit::Fragment {
                node: previous_node,
                edit: FragmentEdit::Children(ListEdit::Insert { item, .. }),
            },
    }) = &case.ops[index - 1]
    else {
        return;
    };

    if vnode != previous_vnode || node != previous_node || item.is_some() {
        return;
    }

    let mut candidate = case.clone_case();
    let Op::Mutate(ModelEdit::VNode {
        edit:
            TemplateEdit::Fragment {
                edit: FragmentEdit::Children(ListEdit::Insert { item, .. }),
                ..
            },
        ..
    }) = &mut candidate.ops[index - 1]
    else {
        unreachable!();
    };
    *item = Some(*base);
    candidate.ops.remove(index);
    out.push(candidate);
}

pub(crate) fn random_multistep_shrink_case(case: &mut FuzzCase, rng: &mut mutatis::Rng) -> bool {
    random_multistep_shrink_case_with(case, |len| rng.gen_index(len).unwrap())
}

fn random_multistep_shrink_case_with(
    case: &mut FuzzCase,
    mut random_index: impl FnMut(usize) -> usize,
) -> bool {
    if case.ops.len() <= 1 {
        return false;
    }

    let steps = 2 + random_index(case.ops.len().min(8));
    let mut changed = 0;

    for _ in 0..steps {
        if apply_random_reduction(case, &mut random_index) {
            changed += 1;
        }
        if case.ops.len() <= 1 {
            break;
        }
    }

    changed >= 2
}

fn apply_random_reduction(
    case: &mut FuzzCase,
    random_index: &mut impl FnMut(usize) -> usize,
) -> bool {
    if case.ops.is_empty() {
        return false;
    }

    match random_index(5) {
        0 => random_delete_range(random_index, case),
        1 => random_truncate(random_index, case),
        2 | 3 => random_simplify_op(random_index, case),
        _ => random_peephole(random_index, case),
    }
}

fn random_delete_range(random_index: &mut impl FnMut(usize) -> usize, case: &mut FuzzCase) -> bool {
    if case.ops.len() <= 1 {
        return false;
    }

    let max_delete = case.ops.len() - 1;
    let len = 1 + random_index(max_delete);
    let start = random_index(case.ops.len() - len + 1);
    case.ops.drain(start..start + len);
    true
}

fn random_truncate(random_index: &mut impl FnMut(usize) -> usize, case: &mut FuzzCase) -> bool {
    if case.ops.len() <= 1 {
        return false;
    }

    let new_len = 1 + random_index(case.ops.len() - 1);
    case.ops.truncate(new_len);
    true
}

fn random_simplify_op(random_index: &mut impl FnMut(usize) -> usize, case: &mut FuzzCase) -> bool {
    for _ in 0..case.ops.len().min(16) {
        let index = random_index(case.ops.len());
        let replacements = simplified_ops(&case.ops[index]);
        if replacements.is_empty() {
            continue;
        }

        case.ops[index] = replacements[random_index(replacements.len())].clone();
        return true;
    }
    false
}

fn random_peephole(random_index: &mut impl FnMut(usize) -> usize, case: &mut FuzzCase) -> bool {
    for _ in 0..case.ops.len().min(16) {
        let index = random_index(case.ops.len());
        let candidates = peephole_cases(case, index);
        if candidates.is_empty() {
            continue;
        }

        *case = candidates[random_index(candidates.len())].clone_case();
        return true;
    }
    false
}

fn seed_from_case(case: &FuzzCase) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for byte in format!("{:?}", case.ops).bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

#[derive(Clone, Debug)]
struct ReductionRng {
    state: u64,
}

impl ReductionRng {
    fn new(seed: u64) -> Self {
        Self { state: seed.max(1) }
    }

    fn next(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }

    fn index(&mut self, len: usize) -> usize {
        debug_assert!(len > 0);
        (self.next() as usize) % len
    }
}

fn simplified_template_edits(edit: &TemplateEdit) -> Vec<TemplateEdit> {
    let mut out = HashSet::new();
    match edit {
        TemplateEdit::SetNode { node, kind } => {
            for node in simpler_u8_values(*node) {
                out.insert(TemplateEdit::SetNode {
                    node,
                    kind: kind.clone(),
                });
            }
            for kind in simplified_template_node_kinds(kind) {
                out.insert(TemplateEdit::SetNode { node: *node, kind });
            }
        }
        TemplateEdit::Roots { edit } => {
            for edit in simplified_list_edits(edit, simplified_template_node_kinds) {
                out.insert(TemplateEdit::Roots { edit });
            }
        }
        TemplateEdit::Children { element, edit } => {
            for element in simpler_u8_values(*element) {
                out.insert(TemplateEdit::Children {
                    element,
                    edit: edit.clone(),
                });
            }
            for edit in simplified_list_edits(edit, simplified_template_node_kinds) {
                out.insert(TemplateEdit::Children {
                    element: *element,
                    edit,
                });
            }
        }
        TemplateEdit::Attrs { element, edit } => {
            for element in simpler_u8_values(*element) {
                out.insert(TemplateEdit::Attrs {
                    element,
                    edit: edit.clone(),
                });
            }
            for edit in simplified_list_edits(edit, simplified_template_attr_specs) {
                out.insert(TemplateEdit::Attrs {
                    element: *element,
                    edit,
                });
            }
        }
        TemplateEdit::Fragment { node, edit } => {
            for node in simpler_u8_values(*node) {
                out.insert(TemplateEdit::Fragment {
                    node,
                    edit: edit.clone(),
                });
            }
            for edit in simplified_fragment_edits(edit) {
                out.insert(TemplateEdit::Fragment { node: *node, edit });
            }
        }
        TemplateEdit::DynamicAttrs { attr, edit } => {
            for attr in simpler_u8_values(*attr) {
                out.insert(TemplateEdit::DynamicAttrs {
                    attr,
                    edit: edit.clone(),
                });
            }
            for edit in simplified_list_edits(edit, simplified_attr_specs) {
                out.insert(TemplateEdit::DynamicAttrs { attr: *attr, edit });
            }
        }
    }
    out.into_iter().collect()
}

fn simplified_template_node_kinds(kind: &TemplateNodeKind) -> Vec<TemplateNodeKind> {
    let mut out = HashSet::new();
    match kind {
        TemplateNodeKind::Element { tag, namespace } => {
            for tag in simpler_u8_values(*tag) {
                out.insert(TemplateNodeKind::Element {
                    tag,
                    namespace: *namespace,
                });
            }
            for namespace in simplified_options(*namespace) {
                out.insert(TemplateNodeKind::Element {
                    tag: *tag,
                    namespace,
                });
            }
            out.insert(TemplateNodeKind::Text(0));
            out.insert(TemplateNodeKind::Dynamic(DynamicKind::Empty));
        }
        TemplateNodeKind::Text(value) => {
            for value in simpler_u8_values(*value) {
                out.insert(TemplateNodeKind::Text(value));
            }
            out.insert(TemplateNodeKind::Dynamic(DynamicKind::Empty));
        }
        TemplateNodeKind::Dynamic(kind) => {
            for kind in simplified_dynamic_kinds(kind) {
                out.insert(TemplateNodeKind::Dynamic(kind));
            }
        }
    }
    out.into_iter().collect()
}

fn simplified_template_attr_specs(attr: &TemplateAttrSpec) -> Vec<TemplateAttrSpec> {
    let mut out = HashSet::new();
    match attr {
        TemplateAttrSpec::Static {
            name,
            value,
            namespace,
        } => {
            for name in simpler_u8_values(*name) {
                out.insert(TemplateAttrSpec::Static {
                    name,
                    value: *value,
                    namespace: *namespace,
                });
            }
            for value in simpler_u8_values(*value) {
                out.insert(TemplateAttrSpec::Static {
                    name: *name,
                    value,
                    namespace: *namespace,
                });
            }
            for namespace in simplified_options(*namespace) {
                out.insert(TemplateAttrSpec::Static {
                    name: *name,
                    value: *value,
                    namespace,
                });
            }
        }
        TemplateAttrSpec::Dynamic(attrs) => {
            for attrs in simplified_attr_vecs(attrs) {
                out.insert(TemplateAttrSpec::Dynamic(attrs));
            }
        }
    }
    out.into_iter().collect()
}

fn simplified_attr_vecs(attrs: &[AttrSpec]) -> Vec<Vec<AttrSpec>> {
    let mut out = HashSet::new();
    if !attrs.is_empty() {
        out.insert(Vec::new());
    }

    for index in 0..attrs.len() {
        let mut candidate = attrs.to_vec();
        candidate.remove(index);
        out.insert(candidate);

        for attr in simplified_attr_specs(&attrs[index]) {
            let mut candidate = attrs.to_vec();
            candidate[index] = attr;
            out.insert(candidate);
        }
    }

    out.into_iter().collect()
}

fn simplified_dynamic_kinds(kind: &DynamicKind) -> Vec<DynamicKind> {
    let mut out = HashSet::new();
    match kind {
        DynamicKind::Empty => {}
        DynamicKind::Text(value) => {
            for value in simpler_u8_values(*value) {
                out.insert(DynamicKind::Text(value));
            }
            out.insert(DynamicKind::Empty);
        }
        DynamicKind::Placeholder => {
            out.insert(DynamicKind::Empty);
        }
        DynamicKind::Fragment { children, key_base } => {
            for children in simpler_u8_values(*children) {
                out.insert(DynamicKind::Fragment {
                    children,
                    key_base: *key_base,
                });
            }
            for key_base in simplified_options(*key_base) {
                out.insert(DynamicKind::Fragment {
                    children: *children,
                    key_base,
                });
            }
            out.insert(DynamicKind::Empty);
        }
        DynamicKind::ComponentA => {
            out.insert(DynamicKind::Fragment {
                children: 0,
                key_base: None,
            });
            out.insert(DynamicKind::Empty);
        }
        DynamicKind::ComponentB => {
            out.insert(DynamicKind::ComponentA);
            out.insert(DynamicKind::Fragment {
                children: 0,
                key_base: None,
            });
            out.insert(DynamicKind::Empty);
        }
        DynamicKind::Suspense { mode } => {
            for mode in simplified_suspense_modes(*mode) {
                out.insert(DynamicKind::Suspense { mode });
            }
            out.insert(DynamicKind::ComponentA);
            out.insert(DynamicKind::Fragment {
                children: 0,
                key_base: None,
            });
            out.insert(DynamicKind::Empty);
        }
        DynamicKind::Portal => {
            out.insert(DynamicKind::Fragment {
                children: 0,
                key_base: None,
            });
            out.insert(DynamicKind::Empty);
        }
    }
    out.into_iter().collect()
}

fn simplified_fragment_edits(edit: &FragmentEdit) -> Vec<FragmentEdit> {
    let mut out = HashSet::new();
    match edit {
        FragmentEdit::KeyMode(mode) => {
            for mode in simplified_fragment_key_modes(mode) {
                out.insert(FragmentEdit::KeyMode(mode));
            }
        }
        FragmentEdit::Children(edit) => {
            for edit in simplified_list_edits(edit, simplified_option_values) {
                out.insert(FragmentEdit::Children(edit));
            }
        }
    }
    out.into_iter().collect()
}

fn simplified_fragment_key_modes(mode: &FragmentKeyMode) -> Vec<FragmentKeyMode> {
    let mut out = HashSet::new();
    match mode {
        FragmentKeyMode::Unkeyed => {}
        FragmentKeyMode::Keyed { base } => {
            for base in simpler_u8_values(*base) {
                out.insert(FragmentKeyMode::Keyed { base });
            }
            out.insert(FragmentKeyMode::Unkeyed);
        }
    }
    out.into_iter().collect()
}

fn simplified_attr_specs(attr: &AttrSpec) -> Vec<AttrSpec> {
    let mut out = HashSet::new();
    for name in simpler_u8_values(attr.name) {
        let mut candidate = attr.clone();
        candidate.name = name;
        out.insert(candidate);
    }
    for namespace in simplified_options(attr.namespace) {
        let mut candidate = attr.clone();
        candidate.namespace = namespace;
        out.insert(candidate);
    }
    for value in simplified_attr_values(&attr.value) {
        let mut candidate = attr.clone();
        candidate.value = value;
        out.insert(candidate);
    }
    if attr.volatile {
        let mut candidate = attr.clone();
        candidate.volatile = false;
        out.insert(candidate);
    }
    out.into_iter().collect()
}

fn simplified_attr_values(value: &AttrValueSpec) -> Vec<AttrValueSpec> {
    let mut out = HashSet::new();
    match value {
        AttrValueSpec::Text(value) => {
            for value in simpler_u8_values(*value) {
                out.insert(AttrValueSpec::Text(value));
            }
        }
        AttrValueSpec::Float(value) => {
            for value in simpler_u8_values(*value) {
                out.insert(AttrValueSpec::Float(value));
            }
            out.insert(AttrValueSpec::Int(*value));
            out.insert(AttrValueSpec::Text(0));
        }
        AttrValueSpec::Int(value) => {
            for value in simpler_u8_values(*value) {
                out.insert(AttrValueSpec::Int(value));
            }
            out.insert(AttrValueSpec::Text(0));
        }
        AttrValueSpec::Bool(value) => {
            if *value {
                out.insert(AttrValueSpec::Bool(false));
            }
            out.insert(AttrValueSpec::Text(0));
        }
        AttrValueSpec::Any(value) => {
            for value in simpler_u8_values(*value) {
                out.insert(AttrValueSpec::Any(value));
            }
            out.insert(AttrValueSpec::Text(0));
        }
        AttrValueSpec::None => {
            out.insert(AttrValueSpec::Text(0));
        }
        AttrValueSpec::Listener => {
            out.insert(AttrValueSpec::None);
            out.insert(AttrValueSpec::Text(0));
        }
    }
    out.into_iter().collect()
}

fn simplified_wake_mutations(mutation: WakeMutationSpec) -> Vec<WakeMutationSpec> {
    let mut out = HashSet::new();
    match mutation {
        WakeMutationSpec::None => {}
        WakeMutationSpec::PrependStaticRoot { tag } => {
            for tag in simpler_u8_values(tag) {
                out.insert(WakeMutationSpec::PrependStaticRoot { tag });
            }
            out.insert(WakeMutationSpec::None);
        }
    }
    out.into_iter().collect()
}

fn simplified_suspense_modes(mode: SuspenseMode) -> Vec<SuspenseMode> {
    let mut out = HashSet::new();
    if let SuspenseMode::Ready { wake_after } = mode {
        for wake_after in simpler_u8_values(wake_after) {
            out.insert(SuspenseMode::Ready { wake_after });
        }
        out.insert(SuspenseMode::Ready { wake_after: 0 });
    }
    for candidate in [SuspenseMode::Resolved, SuspenseMode::Pending] {
        if candidate != mode {
            out.insert(candidate);
        }
    }
    out.insert(SuspenseMode::Ready { wake_after: 0 });
    out.into_iter().collect()
}

fn simplified_list_edits<T>(edit: &ListEdit<T>, simplify_item: fn(&T) -> Vec<T>) -> Vec<ListEdit<T>>
where
    T: Clone + Eq + Hash,
{
    let mut out = HashSet::new();
    match edit {
        ListEdit::Insert { index, item } => {
            for index in simpler_u8_values(*index) {
                out.insert(ListEdit::Insert {
                    index,
                    item: item.clone(),
                });
            }
            for item in simplify_item(item) {
                out.insert(ListEdit::Insert {
                    index: *index,
                    item,
                });
            }
            out.insert(ListEdit::Remove { index: *index });
        }
        ListEdit::Remove { index } => {
            for index in simpler_u8_values(*index) {
                out.insert(ListEdit::Remove { index });
            }
        }
        ListEdit::Move { from, to } => {
            for from in simpler_u8_values(*from) {
                out.insert(ListEdit::Move { from, to: *to });
            }
            for to in simpler_u8_values(*to) {
                out.insert(ListEdit::Move { from: *from, to });
            }
            out.insert(ListEdit::Remove { index: *from });
        }
    }
    out.into_iter().collect()
}

fn simplified_options(value: Option<u8>) -> Vec<Option<u8>> {
    let mut out = HashSet::new();
    if let Some(value) = value {
        out.insert(None);
        for value in simpler_u8_values(value) {
            out.insert(Some(value));
        }
    }
    out.into_iter().collect()
}

fn simplified_option_values(value: &Option<u8>) -> Vec<Option<u8>> {
    simplified_options(*value)
}

fn simpler_u8_values(value: u8) -> Vec<u8> {
    let mut out = HashSet::new();
    for candidate in [
        0,
        1,
        2,
        3,
        4,
        5,
        6,
        7,
        value % 8,
        value % 16,
        value / 2,
        value.saturating_sub(1),
    ] {
        if candidate < value {
            out.insert(candidate);
        }
    }
    let mut out = out.into_iter().collect::<Vec<_>>();
    out.sort_unstable();
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn passing_case_is_not_reduced() {
        let case = FuzzCase::default();
        assert!(
            reduce_case_to_encoded_vec(&case, usize::MAX, usize::MAX, ReductionOptions::default())
                .is_none()
        );
    }

    #[test]
    fn u8_simplification_prefers_small_values() {
        assert_eq!(simpler_u8_values(0), Vec::<u8>::new());
        assert_eq!(simpler_u8_values(3), vec![0, 1, 2]);
        assert_eq!(
            simpler_u8_values(146),
            vec![0, 1, 2, 3, 4, 5, 6, 7, 73, 145]
        );
    }

    #[test]
    fn key_mode_can_fold_into_previous_insert() {
        let case = FuzzCase::new(vec![
            Op::template(
                0,
                TemplateEdit::SetNode {
                    node: 0,
                    kind: TemplateNodeKind::Dynamic(DynamicKind::Fragment {
                        children: 0,
                        key_base: None,
                    }),
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
                FragmentEdit::KeyMode(FragmentKeyMode::Keyed { base: 3 }),
            ),
        ]);

        let candidates = peephole_cases(&case, 2);
        assert_eq!(candidates.len(), 1);
        assert_eq!(
            candidates[0].ops[1],
            Op::fragment(
                0,
                0,
                FragmentEdit::Children(ListEdit::Insert {
                    index: 0,
                    item: Some(3),
                }),
            )
        );
        assert_eq!(candidates[0].ops.len(), 2);
    }

    #[test]
    fn random_multistep_can_compose_reductions() {
        let mut case = FuzzCase::new(vec![Op::Rerender, Op::Rerender, Op::Rerender, Op::Rerender]);

        assert!(random_multistep_shrink_case_with(&mut case, |_| 0));
        assert_eq!(case.ops.len(), 2);
    }
}
