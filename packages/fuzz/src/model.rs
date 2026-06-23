// See note in `ops.rs`: `Mutate` derive emits a wide `new` ctor.
#![allow(clippy::too_many_arguments)]

use mutatis::{Candidates, DefaultMutate, Generate, Mutate, Result as MutatisResult};
use serde::{Deserialize, Serialize};

/// Fold every attribute name into a 16-slot pool so static and dynamic
/// attributes on the same element collide on the same `(name, namespace)`
/// key often enough for `remove_attribute_or_restore_static` to fire.
pub(crate) const ATTR_NAME_POOL_MASK: u8 = 0x0F;

pub(crate) const MAX_ROOTS: usize = 8;
pub(crate) const MAX_CHILDREN: usize = 8;
pub(crate) const MAX_TEMPLATE_ATTRS: usize = 12;
pub(crate) const MAX_DYNAMIC_ATTRS: usize = 8;
// Larger than `dioxus_core::diff::iterator::FRAGMENT_WORK_BATCH` so the
// fuzz can drive the batched `component_props_update` code path.
pub(crate) const MAX_FRAGMENT_CHILDREN: usize = 24;
pub(crate) const MAX_MODEL_COST: u64 = 256;
pub(crate) const MAX_READY_WAKE_COUNT: u8 = 4;
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Model {
    pub(crate) root: VNodeSpec,
    pub(crate) next_suspense_id: u64,
    pub(crate) next_component_id: u64,
}

impl Model {
    pub(crate) fn initial() -> Self {
        Self {
            root: VNodeSpec::minimal(),
            next_suspense_id: 0,
            next_component_id: 0,
        }
    }

    pub(crate) fn selected_vnode_mut(&mut self, selector: u8) -> &mut VNodeSpec {
        let count = self.root.vnode_count();
        self.root
            .nth_vnode_mut(selector as usize % count)
            .expect("vnode selector should resolve into the root tree")
    }

    pub(crate) fn can_grow(&self) -> bool {
        self.root.node_count() < MAX_MODEL_COST
    }

    pub(crate) fn selected_ready_suspense_key(&self, selector: u8) -> Option<SuspenseReadyKey> {
        let mut keys = Vec::new();
        self.root.collect_ready_suspense_keys(&mut keys);
        select(keys, selector)
    }

    pub(crate) fn set_selected_suspense_mode(&mut self, selector: u8, mode: SuspenseMode) {
        if let Some(suspense) = self.selected_suspense_mut(selector) {
            suspense.set_mode(mode);
        }
    }

    pub(crate) fn set_selected_suspense_wake_mutation(
        &mut self,
        selector: u8,
        mutation: WakeMutationSpec,
    ) {
        if let Some(suspense) = self.selected_suspense_mut(selector) {
            suspense.set_wake_mutation(mutation);
        }
    }

    fn selected_suspense_mut(&mut self, selector: u8) -> Option<&mut SuspenseSpec> {
        let count = self.root.suspense_count();
        if count == 0 {
            return None;
        }
        self.root.nth_suspense_mut(selector as usize % count)
    }

    pub(crate) fn wake_ready_suspense(&mut self, key: SuspenseReadyKey) {
        self.root.wake_ready_suspense(key);
    }

    pub(crate) fn wake_mutation_for_ready_key(&self, key: SuspenseReadyKey) -> WakeMutationSpec {
        self.root
            .wake_mutation_for_ready_key(key)
            .unwrap_or(WakeMutationSpec::None)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct VNodeSpec {
    pub(crate) key: Option<u8>,
    pub(crate) template: TemplateSpec,
}

impl VNodeSpec {
    pub(crate) fn minimal() -> Self {
        Self {
            key: None,
            template: TemplateSpec {
                cache_key: None,
                roots: vec![TemplateNodeSpec::Element {
                    tag: 0,
                    namespace: None,
                    attrs: Vec::new(),
                    children: Vec::new(),
                }],
            },
        }
    }

    pub(crate) fn normalize(mut self) -> Self {
        self.normalize_in_place();
        self
    }

    pub(crate) fn normalize_in_place(&mut self) {
        self.template.normalize_in_place();
    }

    /// Walk this vnode tree depth-first in document order, reporting every
    /// vnode and dynamic slot together with the ids of the suspense
    /// boundaries that enclose it.
    ///
    /// This is the single traversal that all whole-model queries are built
    /// on, so every consumer agrees on ordering and numbering.
    pub(crate) fn visit<'a>(&'a self, f: &mut impl FnMut(ModelVisit<'a>, &[u64])) {
        let mut suspense_ancestors = Vec::new();
        visit_vnode(self, &mut suspense_ancestors, f);
    }

    pub(crate) fn vnode_count(&self) -> usize {
        let mut count = 0;
        self.visit(&mut |visit, _| {
            if matches!(visit, ModelVisit::VNode(_)) {
                count += 1;
            }
        });
        count
    }

    pub(crate) fn nth_vnode_mut(&mut self, mut index: usize) -> Option<&mut VNodeSpec> {
        find_vnode_mut(self, &mut |_| match index.checked_sub(1) {
            Some(remaining) => {
                index = remaining;
                false
            }
            None => true,
        })
    }

    pub(crate) fn node_count(&self) -> u64 {
        let mut total = 0;
        self.visit(&mut |visit, _| match visit {
            ModelVisit::VNode(vnode) => total += 1 + template_local_cost(&vnode.template.roots),
            ModelVisit::Dynamic(DynamicSpec::Suspense(spec)) => {
                total += u64::from(spec.wake_mutation.adds_root());
            }
            ModelVisit::Dynamic(_) => {}
        });
        total
    }

    pub(crate) fn suspense_count(&self) -> usize {
        let mut count = 0;
        self.visit(&mut |visit, _| {
            if matches!(visit, ModelVisit::Dynamic(DynamicSpec::Suspense(_))) {
                count += 1;
            }
        });
        count
    }

    pub(crate) fn nth_suspense_mut(&mut self, mut index: usize) -> Option<&mut SuspenseSpec> {
        find_suspense_mut(self, &mut |_| match index.checked_sub(1) {
            Some(remaining) => {
                index = remaining;
                false
            }
            None => true,
        })
    }

    pub(crate) fn collect_ready_suspense_keys(&self, out: &mut Vec<SuspenseReadyKey>) {
        self.visit(&mut |visit, _| {
            if let ModelVisit::Dynamic(DynamicSpec::Suspense(spec)) = visit {
                if spec.mode.is_ready() {
                    out.push(spec.ready_key());
                }
            }
        });
    }

    pub(crate) fn wake_ready_suspense(&mut self, key: SuspenseReadyKey) {
        find_suspense_mut(self, &mut |spec| {
            if spec.mode.is_ready() && spec.ready_key() == key {
                spec.wake_ready();
            }
            false
        });
    }

    pub(crate) fn wake_mutation_for_ready_key(
        &self,
        key: SuspenseReadyKey,
    ) -> Option<WakeMutationSpec> {
        let mut found = None;
        self.visit(&mut |visit, _| {
            if let ModelVisit::Dynamic(DynamicSpec::Suspense(spec)) = visit {
                if found.is_none() && spec.ready_key() == key {
                    found = Some(spec.wake_mutation);
                }
            }
        });
        found
    }
}

/// A single event from [`VNodeSpec::visit`]'s depth-first walk.
pub(crate) enum ModelVisit<'a> {
    /// Entered a vnode (the root vnode itself or any nested vnode).
    VNode(&'a VNodeSpec),
    /// Encountered a dynamic slot inside the current template.
    Dynamic(&'a DynamicSpec),
}

fn visit_vnode<'a>(
    vnode: &'a VNodeSpec,
    suspense_ancestors: &mut Vec<u64>,
    f: &mut impl FnMut(ModelVisit<'a>, &[u64]),
) {
    f(ModelVisit::VNode(vnode), suspense_ancestors);
    visit_template_nodes(&vnode.template.roots, suspense_ancestors, f);
}

fn visit_template_nodes<'a>(
    nodes: &'a [TemplateNodeSpec],
    suspense_ancestors: &mut Vec<u64>,
    f: &mut impl FnMut(ModelVisit<'a>, &[u64]),
) {
    for node in nodes {
        match node {
            TemplateNodeSpec::Element { children, .. } => {
                visit_template_nodes(children, suspense_ancestors, f);
            }
            TemplateNodeSpec::Text(_) => {}
            TemplateNodeSpec::Dynamic(dynamic) => {
                f(ModelVisit::Dynamic(dynamic), suspense_ancestors);
                let entered_suspense = if let DynamicSpec::Suspense(spec) = dynamic {
                    suspense_ancestors.push(spec.id);
                    true
                } else {
                    false
                };
                for child in dynamic.child_vnodes() {
                    visit_vnode(child, suspense_ancestors, f);
                }
                if entered_suspense {
                    suspense_ancestors.pop();
                }
            }
        }
    }
}

/// The model cost of one template's own nodes, excluding nested vnodes
/// (which are accounted for by their own [`ModelVisit::VNode`] events).
fn template_local_cost(nodes: &[TemplateNodeSpec]) -> u64 {
    nodes
        .iter()
        .map(|node| match node {
            TemplateNodeSpec::Element {
                attrs, children, ..
            } => {
                1 + attrs.len() as u64
                    + attrs
                        .iter()
                        .map(|attr| match attr {
                            TemplateAttrSpec::Static { .. } => 0,
                            TemplateAttrSpec::Dynamic(attrs) => attrs.len() as u64,
                        })
                        .sum::<u64>()
                    + template_local_cost(children)
            }
            TemplateNodeSpec::Text(_) => 1,
            // One for the template slot itself plus one for the dynamic node value.
            TemplateNodeSpec::Dynamic(_) => 2,
        })
        .sum()
}

/// Find the first vnode (pre-order, including `vnode` itself) for which `f`
/// returns true. `f` may mutate the vnodes it inspects, so this also serves
/// as a visit-all walker when `f` always returns false.
fn find_vnode_mut<'a>(
    vnode: &'a mut VNodeSpec,
    f: &mut impl FnMut(&mut VNodeSpec) -> bool,
) -> Option<&'a mut VNodeSpec> {
    if f(vnode) {
        return Some(vnode);
    }
    find_vnode_in_template_mut(&mut vnode.template.roots, f)
}

fn find_vnode_in_template_mut<'a>(
    nodes: &'a mut [TemplateNodeSpec],
    f: &mut impl FnMut(&mut VNodeSpec) -> bool,
) -> Option<&'a mut VNodeSpec> {
    for node in nodes {
        let nested = match node {
            TemplateNodeSpec::Element { children, .. } => {
                if let Some(found) = find_vnode_in_template_mut(children, f) {
                    return Some(found);
                }
                continue;
            }
            TemplateNodeSpec::Text(_) => continue,
            TemplateNodeSpec::Dynamic(dynamic) => dynamic.child_vnodes_mut(),
        };
        for child in nested {
            if let Some(found) = find_vnode_mut(child, f) {
                return Some(found);
            }
        }
    }
    None
}

/// Find the first suspense spec (pre-order) for which `f` returns true. `f`
/// may mutate the specs it inspects, so this also serves as a visit-all
/// walker when `f` always returns false.
fn find_suspense_mut<'a>(
    vnode: &'a mut VNodeSpec,
    f: &mut impl FnMut(&mut SuspenseSpec) -> bool,
) -> Option<&'a mut SuspenseSpec> {
    find_suspense_in_template_mut(&mut vnode.template.roots, f)
}

fn find_suspense_in_template_mut<'a>(
    nodes: &'a mut [TemplateNodeSpec],
    f: &mut impl FnMut(&mut SuspenseSpec) -> bool,
) -> Option<&'a mut SuspenseSpec> {
    for node in nodes {
        match node {
            TemplateNodeSpec::Element { children, .. } => {
                if let Some(found) = find_suspense_in_template_mut(children, f) {
                    return Some(found);
                }
            }
            TemplateNodeSpec::Text(_) => {}
            TemplateNodeSpec::Dynamic(DynamicSpec::Suspense(spec)) => {
                if f(spec) {
                    return Some(spec);
                }
                if let Some(found) = find_suspense_mut(&mut spec.child, f) {
                    return Some(found);
                }
            }
            TemplateNodeSpec::Dynamic(dynamic) => {
                for child in dynamic.child_vnodes_mut() {
                    if let Some(found) = find_suspense_mut(child, f) {
                        return Some(found);
                    }
                }
            }
        }
    }
    None
}

fn collect_dynamics<'a>(nodes: &'a [TemplateNodeSpec], out: &mut Vec<&'a DynamicSpec>) {
    for node in nodes {
        match node {
            TemplateNodeSpec::Element { children, .. } => collect_dynamics(children, out),
            TemplateNodeSpec::Text(_) => {}
            TemplateNodeSpec::Dynamic(dynamic) => out.push(dynamic),
        }
    }
}

fn collect_dynamics_mut<'a>(nodes: &'a mut [TemplateNodeSpec], out: &mut Vec<&'a mut DynamicSpec>) {
    for node in nodes {
        match node {
            TemplateNodeSpec::Element { children, .. } => collect_dynamics_mut(children, out),
            TemplateNodeSpec::Text(_) => {}
            TemplateNodeSpec::Dynamic(dynamic) => out.push(dynamic),
        }
    }
}

fn collect_dynamic_attr_lists<'a>(nodes: &'a [TemplateNodeSpec], out: &mut Vec<&'a [AttrSpec]>) {
    for node in nodes {
        let TemplateNodeSpec::Element {
            attrs, children, ..
        } = node
        else {
            continue;
        };
        for attr in attrs {
            if let TemplateAttrSpec::Dynamic(attrs) = attr {
                out.push(attrs);
            }
        }
        collect_dynamic_attr_lists(children, out);
    }
}

fn collect_dynamic_attr_lists_mut<'a>(
    nodes: &'a mut [TemplateNodeSpec],
    out: &mut Vec<&'a mut Vec<AttrSpec>>,
) {
    for node in nodes {
        let TemplateNodeSpec::Element {
            attrs, children, ..
        } = node
        else {
            continue;
        };
        for attr in attrs {
            if let TemplateAttrSpec::Dynamic(attrs) = attr {
                out.push(attrs);
            }
        }
        collect_dynamic_attr_lists_mut(children, out);
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) enum TemplateCacheKey {
    Expanded(Vec<TemplateNodeShape>),
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct TemplateSpec {
    pub(crate) cache_key: Option<TemplateCacheKey>,
    pub(crate) roots: Vec<TemplateNodeSpec>,
}

impl TemplateSpec {
    pub(crate) fn normalize_in_place(&mut self) {
        self.roots.truncate(MAX_ROOTS);
        if self.roots.is_empty() {
            self.roots.push(TemplateNodeSpec::Element {
                tag: 0,
                namespace: None,
                attrs: Vec::new(),
                children: Vec::new(),
            });
        }

        let mut attr_slot = 0;
        for root in &mut self.roots {
            root.normalize_in_place(&mut attr_slot);
        }
    }

    /// This template's dynamic node slots in document order. The indices in
    /// the returned list are the slot numbers used by selector-based ops.
    pub(crate) fn dynamics(&self) -> Vec<&DynamicSpec> {
        let mut out = Vec::new();
        collect_dynamics(&self.roots, &mut out);
        out
    }

    /// Mutable variant of [`Self::dynamics`] with identical ordering.
    pub(crate) fn dynamics_mut(&mut self) -> Vec<&mut DynamicSpec> {
        let mut out = Vec::new();
        collect_dynamics_mut(&mut self.roots, &mut out);
        out
    }

    /// This template's dynamic attribute lists in document order. The
    /// indices in the returned list are the attribute slot numbers used by
    /// selector-based ops.
    pub(crate) fn dynamic_attr_lists(&self) -> Vec<&[AttrSpec]> {
        let mut out = Vec::new();
        collect_dynamic_attr_lists(&self.roots, &mut out);
        out
    }

    /// Mutable variant of [`Self::dynamic_attr_lists`] with identical ordering.
    pub(crate) fn dynamic_attr_lists_mut(&mut self) -> Vec<&mut Vec<AttrSpec>> {
        let mut out = Vec::new();
        collect_dynamic_attr_lists_mut(&mut self.roots, &mut out);
        out
    }

    pub(crate) fn cache_key(&self) -> TemplateCacheKey {
        self.cache_key.clone().unwrap_or_else(|| {
            TemplateCacheKey::Expanded(self.roots.iter().map(TemplateNodeSpec::shape).collect())
        })
    }

    pub(crate) fn node_paths(&self) -> Vec<Vec<usize>> {
        let mut out = Vec::new();
        for (index, root) in self.roots.iter().enumerate() {
            let path = vec![index];
            out.push(path.clone());
            root.collect_node_paths(path, &mut out);
        }
        out
    }

    pub(crate) fn element_paths(&self) -> Vec<Vec<usize>> {
        let mut out = Vec::new();
        for (index, root) in self.roots.iter().enumerate() {
            root.collect_element_paths(vec![index], &mut out);
        }
        out
    }

    pub(crate) fn node_mut(&mut self, path: &[usize]) -> Option<&mut TemplateNodeSpec> {
        let (&root, rest) = path.split_first()?;
        let node = self.roots.get_mut(root)?;
        node.descendant_mut(rest)
    }

    pub(crate) fn element_mut(&mut self, path: &[usize]) -> Option<&mut TemplateNodeSpec> {
        self.node_mut(path)
            .filter(|node| matches!(node, TemplateNodeSpec::Element { .. }))
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum TemplateNodeSpec {
    Element {
        tag: u8,
        namespace: Option<u8>,
        attrs: Vec<TemplateAttrSpec>,
        children: Vec<TemplateNodeSpec>,
    },
    Text(u8),
    Dynamic(DynamicSpec),
}

impl TemplateNodeSpec {
    pub(crate) fn from_kind(
        kind: &TemplateNodeKind,
        next_suspense_id: &mut u64,
        next_component_id: &mut u64,
    ) -> Self {
        match kind {
            TemplateNodeKind::Element { tag, namespace } => Self::Element {
                tag: *tag,
                namespace: *namespace,
                attrs: Vec::new(),
                children: Vec::new(),
            },
            TemplateNodeKind::Text(value) => Self::Text(*value),
            TemplateNodeKind::Dynamic(kind) => Self::Dynamic(DynamicSpec::from_kind(
                kind,
                next_suspense_id,
                next_component_id,
            )),
        }
    }

    pub(crate) fn set_kind(
        &mut self,
        kind: &TemplateNodeKind,
        next_suspense_id: &mut u64,
        next_component_id: &mut u64,
    ) {
        match kind {
            TemplateNodeKind::Element { tag, namespace } => match self {
                Self::Element {
                    tag: current_tag,
                    namespace: current_namespace,
                    ..
                } => {
                    *current_tag = *tag;
                    *current_namespace = *namespace;
                }
                _ => *self = Self::from_kind(kind, next_suspense_id, next_component_id),
            },
            TemplateNodeKind::Text(value) => *self = Self::Text(*value),
            TemplateNodeKind::Dynamic(kind) => match self {
                Self::Dynamic(dynamic) => {
                    dynamic.set_kind(kind, next_suspense_id, next_component_id);
                }
                _ => {
                    *self = Self::Dynamic(DynamicSpec::from_kind(
                        kind,
                        next_suspense_id,
                        next_component_id,
                    ));
                }
            },
        }
    }

    pub(crate) fn normalize_in_place(&mut self, next_attr_slot: &mut usize) {
        match self {
            Self::Element {
                attrs, children, ..
            } => {
                attrs.truncate(MAX_TEMPLATE_ATTRS);
                for attr in attrs {
                    if let TemplateAttrSpec::Dynamic(dynamic_attrs) = attr {
                        sort_attrs(*next_attr_slot, dynamic_attrs);
                        dynamic_attrs.truncate(MAX_DYNAMIC_ATTRS);
                        *next_attr_slot += 1;
                    }
                }

                children.truncate(MAX_CHILDREN);
                for child in children {
                    child.normalize_in_place(next_attr_slot);
                }
            }
            Self::Dynamic(dynamic) => dynamic.normalize_in_place(),
            Self::Text(_) => {}
        }
    }

    pub(crate) fn shape(&self) -> TemplateNodeShape {
        match self {
            Self::Element {
                tag,
                namespace,
                attrs,
                children,
            } => TemplateNodeShape::Element {
                tag: *tag,
                namespace: *namespace,
                attrs: attrs.iter().map(TemplateAttrSpec::shape).collect(),
                children: children.iter().map(TemplateNodeSpec::shape).collect(),
            },
            Self::Text(value) => TemplateNodeShape::Text(*value),
            Self::Dynamic(_) => TemplateNodeShape::Dynamic,
        }
    }

    pub(crate) fn descendant_mut(&mut self, path: &[usize]) -> Option<&mut TemplateNodeSpec> {
        let Some((&index, rest)) = path.split_first() else {
            return Some(self);
        };
        let Self::Element { children, .. } = self else {
            return None;
        };
        children.get_mut(index)?.descendant_mut(rest)
    }

    pub(crate) fn collect_node_paths(&self, path: Vec<usize>, out: &mut Vec<Vec<usize>>) {
        let Self::Element { children, .. } = self else {
            return;
        };
        for (index, child) in children.iter().enumerate() {
            let mut child_path = path.clone();
            child_path.push(index);
            out.push(child_path.clone());
            child.collect_node_paths(child_path, out);
        }
    }

    pub(crate) fn collect_element_paths(&self, path: Vec<usize>, out: &mut Vec<Vec<usize>>) {
        let Self::Element { children, .. } = self else {
            return;
        };
        out.push(path.clone());
        for (index, child) in children.iter().enumerate() {
            let mut child_path = path.clone();
            child_path.push(index);
            child.collect_element_paths(child_path, out);
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Mutate)]
pub(crate) enum TemplateNodeKind {
    Element { tag: u8, namespace: Option<u8> },
    Text(u8),
    Dynamic(DynamicKind),
}

// `Mutate` is hand-written below (see `BiasedTemplateAttrSpecMutator`) so the
// `name` byte gets folded into the shared name pool every time it's mutated,
// not just when a new attribute is first generated.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) enum TemplateAttrSpec {
    Static {
        name: u8,
        value: u8,
        namespace: Option<u8>,
    },
    Dynamic(Vec<AttrSpec>),
}

impl TemplateAttrSpec {
    pub(crate) fn shape(&self) -> TemplateAttrShape {
        match self {
            Self::Static {
                name,
                value,
                namespace,
            } => TemplateAttrShape::Static {
                name: *name,
                value: *value,
                namespace: *namespace,
            },
            Self::Dynamic(_) => TemplateAttrShape::Dynamic,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) enum TemplateNodeShape {
    Element {
        tag: u8,
        namespace: Option<u8>,
        attrs: Vec<TemplateAttrShape>,
        children: Vec<TemplateNodeShape>,
    },
    Text(u8),
    Dynamic,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) enum TemplateAttrShape {
    Static {
        name: u8,
        value: u8,
        namespace: Option<u8>,
    },
    Dynamic,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum DynamicSpec {
    Empty,
    Text(u8),
    Placeholder,
    Fragment(Vec<VNodeSpec>),
    ComponentA(ComponentSpec),
    ComponentB(ComponentSpec),
    Suspense(SuspenseSpec),
    /// A `Portal` that renders its `child` into a render target owned by the
    /// portal mount. Each fresh portal mount creates its own target via a
    /// `use_hook`, so this exercises the cross-render-target diff paths.
    Portal(Box<VNodeSpec>),
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ComponentSpec {
    pub(crate) id: u64,
    pub(crate) child: Box<VNodeSpec>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct SuspenseSpec {
    pub(crate) id: u64,
    pub(crate) ready_generation: u64,
    pub(crate) ready_wake_count: u8,
    pub(crate) mode: SuspenseMode,
    pub(crate) wake_mutation: WakeMutationSpec,
    pub(crate) wake_applied: bool,
    pub(crate) child: Box<VNodeSpec>,
}

impl ComponentSpec {
    pub(crate) fn new(id: u64) -> Self {
        Self {
            id,
            child: Box::new(VNodeSpec::minimal()),
        }
    }
}

impl SuspenseSpec {
    pub(crate) fn new(id: u64, mode: SuspenseMode) -> Self {
        Self {
            id,
            ready_generation: 0,
            ready_wake_count: 0,
            mode,
            wake_mutation: WakeMutationSpec::None,
            wake_applied: false,
            child: Box::new(VNodeSpec::minimal()),
        }
    }

    pub(crate) fn ready_key(&self) -> SuspenseReadyKey {
        SuspenseReadyKey {
            id: self.id,
            generation: self.ready_generation,
        }
    }

    pub(crate) fn set_mode(&mut self, mode: SuspenseMode) {
        if mode.is_ready() && self.mode != mode {
            self.ready_generation += 1;
            self.ready_wake_count = 0;
        }
        self.mode = mode;
        self.wake_applied = false;
    }

    pub(crate) fn set_wake_mutation(&mut self, mutation: WakeMutationSpec) {
        self.wake_mutation = mutation;
        self.wake_applied = false;
    }

    pub(crate) fn wake_ready(&mut self) {
        if !self.mode.is_ready() {
            return;
        }
        self.ready_wake_count = self.ready_wake_count.saturating_add(1);
        if self.ready_wake_count >= self.mode.required_ready_wake_count().unwrap_or(1) {
            self.mode = SuspenseMode::Resolved;
            self.wake_applied = self.wake_mutation != WakeMutationSpec::None;
        }
    }
}

impl DynamicSpec {
    pub(crate) fn from_kind(
        kind: &DynamicKind,
        next_suspense_id: &mut u64,
        next_component_id: &mut u64,
    ) -> Self {
        let mut dynamic = Self::Empty;
        dynamic.set_kind(kind, next_suspense_id, next_component_id);
        dynamic
    }

    pub(crate) fn normalize_in_place(&mut self) {
        match self {
            Self::Fragment(nodes) => {
                nodes.truncate(MAX_FRAGMENT_CHILDREN);
                for node in nodes {
                    node.normalize_in_place();
                }
            }
            Self::ComponentA(component) | Self::ComponentB(component) => {
                component.child.normalize_in_place();
            }
            Self::Suspense(spec) => {
                spec.child.normalize_in_place();
            }
            Self::Portal(child) => {
                child.normalize_in_place();
            }
            Self::Empty | Self::Text(_) | Self::Placeholder => {}
        }
    }

    pub(crate) fn set_kind(
        &mut self,
        kind: &DynamicKind,
        next_suspense_id: &mut u64,
        next_component_id: &mut u64,
    ) {
        match kind {
            DynamicKind::Empty => *self = Self::Empty,
            DynamicKind::Text(value) => *self = Self::Text(*value),
            DynamicKind::Placeholder => *self = Self::Placeholder,
            DynamicKind::Fragment { children, key_base } => {
                if !matches!(self, Self::Fragment(_)) {
                    *self = Self::Fragment(Vec::new());
                }
                let Self::Fragment(nodes) = self else {
                    unreachable!();
                };
                let len = (*children as usize).min(MAX_FRAGMENT_CHILDREN);
                nodes.resize_with(len, VNodeSpec::minimal);
                nodes.truncate(len);
                match key_base {
                    Some(base) => {
                        for (index, child) in nodes.iter_mut().enumerate() {
                            child.key = Some(base.wrapping_add(index as u8));
                        }
                    }
                    None => {
                        for child in nodes {
                            child.key = None;
                        }
                    }
                }
            }
            DynamicKind::ComponentA => {
                if !matches!(self, Self::ComponentA(_)) {
                    let id = *next_component_id;
                    *next_component_id += 1;
                    *self = Self::ComponentA(ComponentSpec::new(id));
                }
            }
            DynamicKind::ComponentB => {
                if !matches!(self, Self::ComponentB(_)) {
                    let id = *next_component_id;
                    *next_component_id += 1;
                    *self = Self::ComponentB(ComponentSpec::new(id));
                }
            }
            DynamicKind::Suspense { mode } => match self {
                Self::Suspense(spec) => spec.set_mode(*mode),
                _ => {
                    let id = *next_suspense_id;
                    *next_suspense_id += 1;
                    *self = Self::Suspense(SuspenseSpec::new(id, *mode));
                }
            },
            DynamicKind::Portal => {
                if !matches!(self, Self::Portal(_)) {
                    *self = Self::Portal(Box::new(VNodeSpec::minimal()));
                }
            }
        }
    }

    /// The nested vnodes directly owned by this dynamic slot, in document
    /// order.
    pub(crate) fn child_vnodes(&self) -> &[VNodeSpec] {
        match self {
            Self::Empty | Self::Text(_) | Self::Placeholder => &[],
            Self::Fragment(nodes) => nodes,
            Self::ComponentA(component) | Self::ComponentB(component) => {
                std::slice::from_ref(&component.child)
            }
            Self::Suspense(spec) => std::slice::from_ref(&spec.child),
            Self::Portal(child) => std::slice::from_ref(&**child),
        }
    }

    /// Mutable variant of [`Self::child_vnodes`] with identical ordering.
    pub(crate) fn child_vnodes_mut(&mut self) -> &mut [VNodeSpec] {
        match self {
            Self::Empty | Self::Text(_) | Self::Placeholder => &mut [],
            Self::Fragment(nodes) => nodes,
            Self::ComponentA(component) | Self::ComponentB(component) => {
                std::slice::from_mut(&mut component.child)
            }
            Self::Suspense(spec) => std::slice::from_mut(&mut spec.child),
            Self::Portal(child) => std::slice::from_mut(child),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Mutate)]
pub(crate) enum DynamicKind {
    Empty,
    Text(u8),
    Fragment { children: u8, key_base: Option<u8> },
    ComponentA,
    ComponentB,
    Suspense { mode: SuspenseMode },
    Placeholder,
    Portal,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Mutate)]
pub(crate) enum SuspenseMode {
    Resolved,
    Pending,
    Ready { wake_after: u8 },
}

impl SuspenseMode {
    pub(crate) fn is_ready(self) -> bool {
        matches!(self, Self::Ready { .. })
    }

    pub(crate) fn required_ready_wake_count(self) -> Option<u8> {
        let Self::Ready { wake_after } = self else {
            return None;
        };
        Some((wake_after % MAX_READY_WAKE_COUNT) + 1)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Mutate)]
pub(crate) enum WakeMutationSpec {
    None,
    PrependStaticRoot { tag: u8 },
}

impl WakeMutationSpec {
    fn adds_root(self) -> bool {
        matches!(self, Self::PrependStaticRoot { .. })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SuspenseReadyKey {
    pub(crate) id: u64,
    pub(crate) generation: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SuspenseTaskKey {
    Pending(u64),
    Ready(SuspenseReadyKey),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Mutate)]
pub(crate) enum FragmentKeyMode {
    Unkeyed,
    Keyed { base: u8 },
}

// `Mutate` is hand-written below (see `BiasedAttrSpecMutator`) so the `name`
// byte gets folded into the shared name pool on every in-place mutation.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct AttrSpec {
    pub(crate) name: u8,
    pub(crate) namespace: Option<u8>,
    pub(crate) value: AttrValueSpec,
    pub(crate) volatile: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Mutate)]
pub(crate) enum AttrValueSpec {
    Text(u8),
    Float(u8),
    Int(u8),
    Bool(bool),
    Any(u8),
    None,
    Listener,
}

pub(crate) fn select<T>(items: Vec<T>, selector: u8) -> Option<T> {
    let len = items.len();
    if len == 0 {
        return None;
    }
    items.into_iter().nth(selector as usize % len)
}

pub(crate) fn sort_attrs(slot: usize, attrs: &mut Vec<AttrSpec>) {
    attrs.sort_by_cached_key(|attr| attr_sort_key(slot, attr));
    attrs.dedup_by(|left, right| attr_sort_key(slot, left) == attr_sort_key(slot, right));
}

fn attr_sort_key(slot: usize, attr: &AttrSpec) -> String {
    match attr.value {
        AttrValueSpec::Listener => format!("onevent{slot}_{}", attr.name),
        _ if attr.name & 0x80 != 0 => format!("onevent{slot}_{}", attr.name & 0x7f),
        _ => format!("attr{}", attr.name),
    }
}

// --- Pool-biased mutators for attribute names -------------------------------
//
// The derived `Mutate` impl mutates `u8` fields uniformly across 0..=255,
// which makes static/dynamic name collisions on the same element vanishingly
// rare. These hand-written mutators fold the `name` byte into the shared
// `ATTR_NAME_POOL_MASK` pool on every in-place mutation, while keeping the
// other fields' mutations identical to the derive's behaviour. A rare
// out-of-pool escape preserves coverage of the "no static collides" path.

/// Mutate a `u8` name field. Half the time we fold the byte into the shared
/// pool so static/dynamic collisions on the same element become probable;
/// the other half we keep a uniform byte so the diff merge's "extra on one
/// side" arms (and other diversity-sensitive paths) keep getting exercised.
fn pool_mutate_name(ctx: &mut mutatis::Context, name: &mut u8) {
    let r = ctx.rng().gen_u8();
    *name = if r & 0x80 == 0 {
        r & ATTR_NAME_POOL_MASK
    } else {
        r
    };
}

fn pool_generate_name(ctx: &mut mutatis::Context) -> u8 {
    let r = ctx.rng().gen_u8();
    if r & 0x80 == 0 {
        r & ATTR_NAME_POOL_MASK
    } else {
        r
    }
}

#[derive(Default)]
pub(crate) struct BiasedAttrSpecMutator {
    namespace: <Option<u8> as DefaultMutate>::DefaultMutate,
    value: <AttrValueSpec as DefaultMutate>::DefaultMutate,
    volatile: <bool as DefaultMutate>::DefaultMutate,
}

impl Mutate<AttrSpec> for BiasedAttrSpecMutator {
    fn mutate(
        &mut self,
        candidates: &mut Candidates<'_>,
        value: &mut AttrSpec,
    ) -> MutatisResult<()> {
        candidates.mutation(|ctx| {
            pool_mutate_name(ctx, &mut value.name);
            Ok(())
        })?;
        self.namespace.mutate(candidates, &mut value.namespace)?;
        self.value.mutate(candidates, &mut value.value)?;
        self.volatile.mutate(candidates, &mut value.volatile)?;
        Ok(())
    }
}

impl Generate<AttrSpec> for BiasedAttrSpecMutator {
    fn generate(&mut self, ctx: &mut mutatis::Context) -> MutatisResult<AttrSpec> {
        Ok(AttrSpec {
            name: pool_generate_name(ctx),
            namespace: self.namespace.generate(ctx)?,
            value: self.value.generate(ctx)?,
            volatile: self.volatile.generate(ctx)?,
        })
    }
}

impl DefaultMutate for AttrSpec {
    type DefaultMutate = BiasedAttrSpecMutator;
}

#[derive(Default)]
pub(crate) struct BiasedTemplateAttrSpecMutator {
    static_value: <u8 as DefaultMutate>::DefaultMutate,
    static_namespace: <Option<u8> as DefaultMutate>::DefaultMutate,
    dynamic_attrs: <Vec<AttrSpec> as DefaultMutate>::DefaultMutate,
}

impl Mutate<TemplateAttrSpec> for BiasedTemplateAttrSpecMutator {
    fn mutate(
        &mut self,
        candidates: &mut Candidates<'_>,
        value: &mut TemplateAttrSpec,
    ) -> MutatisResult<()> {
        // Variant-switching candidate: flip between the two variants.
        let current_is_static = matches!(value, TemplateAttrSpec::Static { .. });
        candidates.mutation_group(1, |ctx, _which| {
            *value = if current_is_static {
                TemplateAttrSpec::Dynamic(self.dynamic_attrs.generate(ctx)?)
            } else {
                TemplateAttrSpec::Static {
                    name: pool_generate_name(ctx),
                    value: self.static_value.generate(ctx)?,
                    namespace: self.static_namespace.generate(ctx)?,
                }
            };
            Ok(())
        })?;

        match value {
            TemplateAttrSpec::Static {
                name,
                value,
                namespace,
            } => {
                candidates.mutation(|ctx| {
                    pool_mutate_name(ctx, name);
                    Ok(())
                })?;
                self.static_value.mutate(candidates, value)?;
                self.static_namespace.mutate(candidates, namespace)?;
            }
            TemplateAttrSpec::Dynamic(attrs) => {
                self.dynamic_attrs.mutate(candidates, attrs)?;
            }
        }

        Ok(())
    }
}

impl Generate<TemplateAttrSpec> for BiasedTemplateAttrSpecMutator {
    fn generate(&mut self, ctx: &mut mutatis::Context) -> MutatisResult<TemplateAttrSpec> {
        Ok(if ctx.rng().gen_index(2).unwrap_or(0) == 0 {
            TemplateAttrSpec::Static {
                name: pool_generate_name(ctx),
                value: self.static_value.generate(ctx)?,
                namespace: self.static_namespace.generate(ctx)?,
            }
        } else {
            TemplateAttrSpec::Dynamic(self.dynamic_attrs.generate(ctx)?)
        })
    }
}

impl DefaultMutate for TemplateAttrSpec {
    type DefaultMutate = BiasedTemplateAttrSpecMutator;
}
