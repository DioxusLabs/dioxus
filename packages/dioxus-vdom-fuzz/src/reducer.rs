use crate::{
    FuzzCase, FuzzFailure,
    model::{
        AttrSpec, AttrValueSpec, DynamicKind, FragmentKeyMode, PortalTargetSpec, SuspenseMode,
        TemplateAttrSpec, TemplateNodeKind, WakeMutationSpec,
    },
    ops::{FragmentEdit, ListEdit, Op, TemplateEdit},
    run_case,
};
use std::{
    fmt,
    panic::{self, AssertUnwindSafe},
    sync::Mutex,
};

#[derive(Clone, Debug)]
pub struct ReductionOptions {
    preserve_failure: bool,
    random_multi_attempts: usize,
}

impl ReductionOptions {
    pub fn preserve_failure(mut self, preserve_failure: bool) -> Self {
        self.preserve_failure = preserve_failure;
        self
    }

    pub fn random_multi_attempts(mut self, attempts: usize) -> Self {
        self.random_multi_attempts = attempts;
        self
    }
}

impl Default for ReductionOptions {
    fn default() -> Self {
        Self {
            preserve_failure: true,
            random_multi_attempts: 2048,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReductionStats {
    pub original_ops: usize,
    pub reduced_ops: usize,
    pub attempts: usize,
    pub accepted: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReductionReport {
    pub case: FuzzCase,
    pub original_failure: FuzzFailure,
    pub reduced_failure: FuzzFailure,
    pub stats: ReductionStats,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReduceError {
    NotFailing,
}

impl fmt::Display for ReduceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFailing => write!(f, "input does not reproduce a fuzz failure"),
        }
    }
}

impl std::error::Error for ReduceError {}

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
    current_failure: FuzzFailure,
    rng: ReductionRng,
    attempts: usize,
    accepted: usize,
}

enum ReductionRun {
    Passed,
    Failed(FuzzFailure),
    Panicked,
}

pub fn reduce_case(
    case: FuzzCase,
    options: ReductionOptions,
) -> Result<ReductionReport, ReduceError> {
    let original_failure = match run_case_for_reduction(&case) {
        ReductionRun::Failed(failure) => failure,
        ReductionRun::Passed | ReductionRun::Panicked => return Err(ReduceError::NotFailing),
    };
    let original_ops = case.ops.len();
    let signature = FailureSignature::new(&original_failure);
    let mut reducer = Reducer {
        options,
        signature,
        current_failure: original_failure.clone(),
        rng: ReductionRng::new(seed_from_case(&case)),
        attempts: 0,
        accepted: 0,
    };
    let mut case = case;

    reducer.truncate_after_failure(&mut case);
    reducer.reduce_to_local_minimum(&mut case);
    reducer.reduce_by_random_multistep(&mut case);
    reducer.reduce_to_local_minimum(&mut case);
    reducer.reduce_by_random_multistep(&mut case);
    reducer.reduce_to_local_minimum(&mut case);

    Ok(ReductionReport {
        stats: ReductionStats {
            original_ops,
            reduced_ops: case.ops.len(),
            attempts: reducer.attempts,
            accepted: reducer.accepted,
        },
        case,
        original_failure,
        reduced_failure: reducer.current_failure,
    })
}

impl Reducer {
    fn reduce_to_local_minimum(&mut self, case: &mut FuzzCase) {
        self.reduce_by_chunk_deletion(case);
        self.reduce_by_single_deletion(case);
        self.reduce_values(case);
        self.reduce_by_peepholes(case);
    }

    fn accepts(&mut self, case: &FuzzCase) -> Option<FuzzFailure> {
        self.attempts += 1;
        let ReductionRun::Failed(failure) = run_case_for_reduction(case) else {
            return None;
        };
        if !self.options.preserve_failure || self.signature.matches(&failure) {
            Some(failure)
        } else {
            None
        }
    }

    fn try_replace(&mut self, case: &mut FuzzCase, mut candidate: FuzzCase) -> bool {
        let Some(failure) = self.accepts(&candidate) else {
            return false;
        };
        candidate.ops.truncate(failure.step() + 1);
        *case = candidate;
        self.current_failure = failure;
        self.accepted += 1;
        true
    }

    fn truncate_after_failure(&mut self, case: &mut FuzzCase) {
        let needed_len = self.current_failure.step() + 1;
        if needed_len >= case.ops.len() {
            return;
        }

        let mut candidate = case.clone();
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
                let mut candidate = case.clone();
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

            let mut candidate = case.clone();
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
    failure
        .message()
        .lines()
        .next()
        .unwrap_or(failure.message())
}

pub(crate) fn simplified_ops(op: &Op) -> Vec<Op> {
    let mut out = Vec::new();
    if !matches!(op, Op::Rerender) {
        push_unique(&mut out, Op::Rerender);
    }

    match op {
        Op::Rerender => {}
        Op::WakeSuspense { suspense } => {
            for suspense in simpler_u8_values(*suspense) {
                push_unique(&mut out, Op::WakeSuspense { suspense });
            }
        }
        Op::WakeSuspenseNatural { suspense } => {
            for suspense in simpler_u8_values(*suspense) {
                push_unique(&mut out, Op::WakeSuspenseNatural { suspense });
            }
            push_unique(
                &mut out,
                Op::WakeSuspense {
                    suspense: *suspense,
                },
            );
        }
        Op::Template { vnode, edit } => {
            for vnode in simpler_u8_values(*vnode) {
                push_unique(
                    &mut out,
                    Op::Template {
                        vnode,
                        edit: edit.clone(),
                    },
                );
            }
            for edit in simplified_template_edits(edit) {
                push_unique(
                    &mut out,
                    Op::Template {
                        vnode: *vnode,
                        edit,
                    },
                );
            }
        }
        Op::Dynamic { vnode, slot, kind } => {
            for vnode in simpler_u8_values(*vnode) {
                push_unique(
                    &mut out,
                    Op::Dynamic {
                        vnode,
                        slot: *slot,
                        kind: kind.clone(),
                    },
                );
            }
            for slot in simpler_u8_values(*slot) {
                push_unique(
                    &mut out,
                    Op::Dynamic {
                        vnode: *vnode,
                        slot,
                        kind: kind.clone(),
                    },
                );
            }
            for kind in simplified_dynamic_kinds(kind) {
                push_unique(
                    &mut out,
                    Op::Dynamic {
                        vnode: *vnode,
                        slot: *slot,
                        kind,
                    },
                );
            }
        }
        Op::DynamicAttrs { vnode, slot, edit } => {
            for vnode in simpler_u8_values(*vnode) {
                push_unique(
                    &mut out,
                    Op::DynamicAttrs {
                        vnode,
                        slot: *slot,
                        edit: edit.clone(),
                    },
                );
            }
            for slot in simpler_u8_values(*slot) {
                push_unique(
                    &mut out,
                    Op::DynamicAttrs {
                        vnode: *vnode,
                        slot,
                        edit: edit.clone(),
                    },
                );
            }
            for edit in simplified_list_edits(edit, simplified_attr_specs) {
                push_unique(
                    &mut out,
                    Op::DynamicAttrs {
                        vnode: *vnode,
                        slot: *slot,
                        edit,
                    },
                );
            }
        }
        Op::Fragment { vnode, slot, edit } => {
            for vnode in simpler_u8_values(*vnode) {
                push_unique(
                    &mut out,
                    Op::Fragment {
                        vnode,
                        slot: *slot,
                        edit: edit.clone(),
                    },
                );
            }
            for slot in simpler_u8_values(*slot) {
                push_unique(
                    &mut out,
                    Op::Fragment {
                        vnode: *vnode,
                        slot,
                        edit: edit.clone(),
                    },
                );
            }
            for edit in simplified_fragment_edits(edit) {
                push_unique(
                    &mut out,
                    Op::Fragment {
                        vnode: *vnode,
                        slot: *slot,
                        edit,
                    },
                );
            }
        }
        Op::Suspense { suspense, mode } => {
            for suspense in simpler_u8_values(*suspense) {
                push_unique(
                    &mut out,
                    Op::Suspense {
                        suspense,
                        mode: *mode,
                    },
                );
            }
            for mode in simplified_suspense_modes(*mode) {
                push_unique(
                    &mut out,
                    Op::Suspense {
                        suspense: *suspense,
                        mode,
                    },
                );
            }
        }
        Op::SuspenseWakeMutation { suspense, mutation } => {
            for suspense in simpler_u8_values(*suspense) {
                push_unique(
                    &mut out,
                    Op::SuspenseWakeMutation {
                        suspense,
                        mutation: *mutation,
                    },
                );
            }
            for mutation in simplified_wake_mutations(*mutation) {
                push_unique(
                    &mut out,
                    Op::SuspenseWakeMutation {
                        suspense: *suspense,
                        mutation,
                    },
                );
            }
        }
    }

    out
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

    let Op::Fragment {
        vnode,
        slot,
        edit: FragmentEdit::KeyMode(FragmentKeyMode::Keyed { base }),
    } = &case.ops[index]
    else {
        return;
    };

    let Op::Fragment {
        vnode: previous_vnode,
        slot: previous_slot,
        edit: FragmentEdit::Children(ListEdit::Insert { item, .. }),
    } = &case.ops[index - 1]
    else {
        return;
    };

    if vnode != previous_vnode || slot != previous_slot || item.is_some() {
        return;
    }

    let mut candidate = case.clone();
    let Op::Fragment {
        edit: FragmentEdit::Children(ListEdit::Insert { item, .. }),
        ..
    } = &mut candidate.ops[index - 1]
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

        *case = candidates[random_index(candidates.len())].clone();
        return true;
    }
    false
}

fn seed_from_case(case: &FuzzCase) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for byte in format!("{case:?}").bytes() {
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
    let mut out = Vec::new();
    match edit {
        TemplateEdit::SetNode { node, kind } => {
            for node in simpler_u8_values(*node) {
                push_unique(
                    &mut out,
                    TemplateEdit::SetNode {
                        node,
                        kind: kind.clone(),
                    },
                );
            }
            for kind in simplified_template_node_kinds(kind) {
                push_unique(&mut out, TemplateEdit::SetNode { node: *node, kind });
            }
        }
        TemplateEdit::Roots { edit } => {
            for edit in simplified_list_edits(edit, simplified_template_node_kinds) {
                push_unique(&mut out, TemplateEdit::Roots { edit });
            }
        }
        TemplateEdit::Children { element, edit } => {
            for element in simpler_u8_values(*element) {
                push_unique(
                    &mut out,
                    TemplateEdit::Children {
                        element,
                        edit: edit.clone(),
                    },
                );
            }
            for edit in simplified_list_edits(edit, simplified_template_node_kinds) {
                push_unique(
                    &mut out,
                    TemplateEdit::Children {
                        element: *element,
                        edit,
                    },
                );
            }
        }
        TemplateEdit::Attrs { element, edit } => {
            for element in simpler_u8_values(*element) {
                push_unique(
                    &mut out,
                    TemplateEdit::Attrs {
                        element,
                        edit: edit.clone(),
                    },
                );
            }
            for edit in simplified_list_edits(edit, simplified_template_attr_specs) {
                push_unique(
                    &mut out,
                    TemplateEdit::Attrs {
                        element: *element,
                        edit,
                    },
                );
            }
        }
        TemplateEdit::Generated {
            seed,
            dynamic_nodes,
            dynamic_attrs,
        } => {
            for seed in simpler_u64_values(*seed) {
                push_unique(
                    &mut out,
                    TemplateEdit::Generated {
                        seed,
                        dynamic_nodes: *dynamic_nodes,
                        dynamic_attrs: *dynamic_attrs,
                    },
                );
            }
            for dynamic_nodes in simpler_u16_values(*dynamic_nodes) {
                push_unique(
                    &mut out,
                    TemplateEdit::Generated {
                        seed: *seed,
                        dynamic_nodes,
                        dynamic_attrs: *dynamic_attrs,
                    },
                );
            }
            for dynamic_attrs in simpler_u16_values(*dynamic_attrs) {
                push_unique(
                    &mut out,
                    TemplateEdit::Generated {
                        seed: *seed,
                        dynamic_nodes: *dynamic_nodes,
                        dynamic_attrs,
                    },
                );
            }
        }
    }
    out
}

fn simplified_template_node_kinds(kind: &TemplateNodeKind) -> Vec<TemplateNodeKind> {
    let mut out = Vec::new();
    match kind {
        TemplateNodeKind::Element { tag, namespace } => {
            for tag in simpler_u8_values(*tag) {
                push_unique(
                    &mut out,
                    TemplateNodeKind::Element {
                        tag,
                        namespace: *namespace,
                    },
                );
            }
            for namespace in simplified_options(*namespace) {
                push_unique(
                    &mut out,
                    TemplateNodeKind::Element {
                        tag: *tag,
                        namespace,
                    },
                );
            }
            push_unique(&mut out, TemplateNodeKind::Text(0));
            push_unique(&mut out, TemplateNodeKind::Dynamic);
        }
        TemplateNodeKind::Text(value) => {
            for value in simpler_u8_values(*value) {
                push_unique(&mut out, TemplateNodeKind::Text(value));
            }
            push_unique(&mut out, TemplateNodeKind::Dynamic);
        }
        TemplateNodeKind::Dynamic => {}
    }
    out
}

fn simplified_template_attr_specs(attr: &TemplateAttrSpec) -> Vec<TemplateAttrSpec> {
    let mut out = Vec::new();
    match attr {
        TemplateAttrSpec::Static {
            name,
            value,
            namespace,
        } => {
            for name in simpler_u8_values(*name) {
                push_unique(
                    &mut out,
                    TemplateAttrSpec::Static {
                        name,
                        value: *value,
                        namespace: *namespace,
                    },
                );
            }
            for value in simpler_u8_values(*value) {
                push_unique(
                    &mut out,
                    TemplateAttrSpec::Static {
                        name: *name,
                        value,
                        namespace: *namespace,
                    },
                );
            }
            for namespace in simplified_options(*namespace) {
                push_unique(
                    &mut out,
                    TemplateAttrSpec::Static {
                        name: *name,
                        value: *value,
                        namespace,
                    },
                );
            }
        }
        TemplateAttrSpec::Dynamic => {}
    }
    out
}

fn simplified_dynamic_kinds(kind: &DynamicKind) -> Vec<DynamicKind> {
    let mut out = Vec::new();
    match kind {
        DynamicKind::Empty => {}
        DynamicKind::Text(value) => {
            for value in simpler_u8_values(*value) {
                push_unique(&mut out, DynamicKind::Text(value));
            }
            push_unique(&mut out, DynamicKind::Empty);
        }
        DynamicKind::Fragment => {
            push_unique(&mut out, DynamicKind::Empty);
        }
        DynamicKind::ComponentA => {
            push_unique(&mut out, DynamicKind::Fragment);
            push_unique(&mut out, DynamicKind::Empty);
        }
        DynamicKind::ComponentB => {
            push_unique(&mut out, DynamicKind::ComponentA);
            push_unique(&mut out, DynamicKind::Fragment);
            push_unique(&mut out, DynamicKind::Empty);
        }
        DynamicKind::Portal { target } => {
            for target in simplified_portal_targets(*target) {
                push_unique(&mut out, DynamicKind::Portal { target });
            }
            push_unique(&mut out, DynamicKind::ComponentA);
            push_unique(&mut out, DynamicKind::Fragment);
            push_unique(&mut out, DynamicKind::Empty);
        }
        DynamicKind::Suspense { mode } => {
            for mode in simplified_suspense_modes(*mode) {
                push_unique(&mut out, DynamicKind::Suspense { mode });
            }
            push_unique(&mut out, DynamicKind::ComponentA);
            push_unique(&mut out, DynamicKind::Fragment);
            push_unique(&mut out, DynamicKind::Empty);
        }
    }
    out
}

fn simplified_portal_targets(target: PortalTargetSpec) -> Vec<PortalTargetSpec> {
    let mut out = Vec::new();
    match target {
        PortalTargetSpec::TargetA => {}
        PortalTargetSpec::TargetB => {
            push_unique(&mut out, PortalTargetSpec::TargetA);
        }
        PortalTargetSpec::Noop => {
            push_unique(&mut out, PortalTargetSpec::TargetA);
            push_unique(&mut out, PortalTargetSpec::TargetB);
        }
    }
    out
}

fn simplified_fragment_edits(edit: &FragmentEdit) -> Vec<FragmentEdit> {
    let mut out = Vec::new();
    match edit {
        FragmentEdit::KeyMode(mode) => {
            for mode in simplified_fragment_key_modes(mode) {
                push_unique(&mut out, FragmentEdit::KeyMode(mode));
            }
        }
        FragmentEdit::Children(edit) => {
            for edit in simplified_list_edits(edit, simplified_option_values) {
                push_unique(&mut out, FragmentEdit::Children(edit));
            }
        }
    }
    out
}

fn simplified_fragment_key_modes(mode: &FragmentKeyMode) -> Vec<FragmentKeyMode> {
    let mut out = Vec::new();
    match mode {
        FragmentKeyMode::Unkeyed => {}
        FragmentKeyMode::Keyed { base } => {
            for base in simpler_u8_values(*base) {
                push_unique(&mut out, FragmentKeyMode::Keyed { base });
            }
            push_unique(&mut out, FragmentKeyMode::Unkeyed);
        }
    }
    out
}

fn simplified_attr_specs(attr: &AttrSpec) -> Vec<AttrSpec> {
    let mut out = Vec::new();
    for name in simpler_u8_values(attr.name) {
        let mut candidate = attr.clone();
        candidate.name = name;
        push_unique(&mut out, candidate);
    }
    for namespace in simplified_options(attr.namespace) {
        let mut candidate = attr.clone();
        candidate.namespace = namespace;
        push_unique(&mut out, candidate);
    }
    for value in simplified_attr_values(&attr.value) {
        let mut candidate = attr.clone();
        candidate.value = value;
        push_unique(&mut out, candidate);
    }
    if attr.volatile {
        let mut candidate = attr.clone();
        candidate.volatile = false;
        push_unique(&mut out, candidate);
    }
    out
}

fn simplified_attr_values(value: &AttrValueSpec) -> Vec<AttrValueSpec> {
    let mut out = Vec::new();
    match value {
        AttrValueSpec::Text(value) => {
            for value in simpler_u8_values(*value) {
                push_unique(&mut out, AttrValueSpec::Text(value));
            }
        }
        AttrValueSpec::Float(value) => {
            for value in simpler_u8_values(*value) {
                push_unique(&mut out, AttrValueSpec::Float(value));
            }
            push_unique(&mut out, AttrValueSpec::Int(*value));
            push_unique(&mut out, AttrValueSpec::Text(0));
        }
        AttrValueSpec::Int(value) => {
            for value in simpler_u8_values(*value) {
                push_unique(&mut out, AttrValueSpec::Int(value));
            }
            push_unique(&mut out, AttrValueSpec::Text(0));
        }
        AttrValueSpec::Bool(value) => {
            if *value {
                push_unique(&mut out, AttrValueSpec::Bool(false));
            }
            push_unique(&mut out, AttrValueSpec::Text(0));
        }
        AttrValueSpec::Any(value) => {
            for value in simpler_u8_values(*value) {
                push_unique(&mut out, AttrValueSpec::Any(value));
            }
            push_unique(&mut out, AttrValueSpec::Text(0));
        }
        AttrValueSpec::None => {
            push_unique(&mut out, AttrValueSpec::Text(0));
        }
        AttrValueSpec::Listener => {
            push_unique(&mut out, AttrValueSpec::None);
            push_unique(&mut out, AttrValueSpec::Text(0));
        }
    }
    out
}

fn simplified_wake_mutations(mutation: WakeMutationSpec) -> Vec<WakeMutationSpec> {
    let mut out = Vec::new();
    match mutation {
        WakeMutationSpec::None => {}
        WakeMutationSpec::PrependStaticRoot { tag } => {
            for tag in simpler_u8_values(tag) {
                push_unique(&mut out, WakeMutationSpec::PrependStaticRoot { tag });
            }
            push_unique(&mut out, WakeMutationSpec::None);
        }
    }
    out
}

fn simplified_suspense_modes(mode: SuspenseMode) -> Vec<SuspenseMode> {
    let mut out = Vec::new();
    for candidate in [
        SuspenseMode::Resolved,
        SuspenseMode::Pending,
        SuspenseMode::Ready,
    ] {
        if candidate != mode {
            out.push(candidate);
        }
    }
    out
}

fn simplified_list_edits<T>(edit: &ListEdit<T>, simplify_item: fn(&T) -> Vec<T>) -> Vec<ListEdit<T>>
where
    T: Clone + PartialEq,
{
    let mut out = Vec::new();
    match edit {
        ListEdit::Insert { index, item } => {
            for index in simpler_u8_values(*index) {
                push_unique(
                    &mut out,
                    ListEdit::Insert {
                        index,
                        item: item.clone(),
                    },
                );
            }
            for item in simplify_item(item) {
                push_unique(
                    &mut out,
                    ListEdit::Insert {
                        index: *index,
                        item,
                    },
                );
            }
            push_unique(&mut out, ListEdit::Remove { index: *index });
        }
        ListEdit::Remove { index } => {
            for index in simpler_u8_values(*index) {
                push_unique(&mut out, ListEdit::Remove { index });
            }
        }
        ListEdit::Move { from, to } => {
            for from in simpler_u8_values(*from) {
                push_unique(&mut out, ListEdit::Move { from, to: *to });
            }
            for to in simpler_u8_values(*to) {
                push_unique(&mut out, ListEdit::Move { from: *from, to });
            }
            push_unique(&mut out, ListEdit::Remove { index: *from });
        }
    }
    out
}

fn simplified_options(value: Option<u8>) -> Vec<Option<u8>> {
    let mut out = Vec::new();
    if let Some(value) = value {
        push_unique(&mut out, None);
        for value in simpler_u8_values(value) {
            push_unique(&mut out, Some(value));
        }
    }
    out
}

fn simplified_option_values(value: &Option<u8>) -> Vec<Option<u8>> {
    simplified_options(*value)
}

fn simpler_u8_values(value: u8) -> Vec<u8> {
    let mut out = Vec::new();
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
            push_unique(&mut out, candidate);
        }
    }
    out
}

fn simpler_u16_values(value: u16) -> Vec<u16> {
    let mut out = Vec::new();
    for candidate in [
        0,
        1,
        2,
        8,
        16,
        64,
        128,
        255,
        256,
        value / 2,
        value.saturating_sub(1),
    ] {
        if candidate < value {
            push_unique(&mut out, candidate);
        }
    }
    out
}

fn simpler_u64_values(value: u64) -> Vec<u64> {
    let mut out = Vec::new();
    for candidate in [0, 1, value & 0xff, value / 2, value.saturating_sub(1)] {
        if candidate < value {
            push_unique(&mut out, candidate);
        }
    }
    out
}

fn push_unique<T>(values: &mut Vec<T>, value: T)
where
    T: PartialEq,
{
    if !values.contains(&value) {
        values.push(value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn passing_case_is_not_reduced() {
        let case = FuzzCase::seed();
        assert_eq!(
            reduce_case(case, ReductionOptions::default()).unwrap_err(),
            ReduceError::NotFailing
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
                edit: FragmentEdit::KeyMode(FragmentKeyMode::Keyed { base: 3 }),
            },
        ]);

        let candidates = peephole_cases(&case, 2);
        assert_eq!(candidates.len(), 1);
        assert_eq!(
            candidates[0].ops[1],
            Op::Fragment {
                vnode: 0,
                slot: 0,
                edit: FragmentEdit::Children(ListEdit::Insert {
                    index: 0,
                    item: Some(3),
                }),
            }
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
