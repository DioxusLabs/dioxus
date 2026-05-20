use mutatis::Mutate;
use serde::{Deserialize, Serialize};

pub(crate) const MAX_ROOTS: usize = 8;
pub(crate) const MAX_CHILDREN: usize = 8;
pub(crate) const MAX_TEMPLATE_ATTRS: usize = 12;
pub(crate) const MAX_DYNAMIC_ATTRS: usize = 8;
pub(crate) const MAX_FRAGMENT_CHILDREN: usize = 8;
pub(crate) const MAX_MODEL_COST: u64 = 256;

// ---------- Spec model ----------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Model {
    pub(crate) root: VNodeSpec,
    pub(crate) next_suspense_id: u64,
}

impl Model {
    pub(crate) fn initial() -> Self {
        Self {
            root: VNodeSpec::minimal(),
            next_suspense_id: 0,
        }
    }

    pub(crate) fn selected_vnode_mut(&mut self, selector: u8) -> &mut VNodeSpec {
        let count = self.root.vnode_count();
        let mut index = selector as usize % count;
        self.root
            .nth_vnode_mut(&mut index)
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
        let count = self.root.suspense_count();
        if count == 0 {
            return;
        }
        let mut index = selector as usize % count;
        if let Some(suspense) = self.root.nth_suspense_mut(&mut index) {
            suspense.set_mode(mode);
        }
    }

    pub(crate) fn set_selected_suspense_wake_mutation(
        &mut self,
        selector: u8,
        mutation: WakeMutationSpec,
    ) {
        let count = self.root.suspense_count();
        if count == 0 {
            return;
        }
        let mut index = selector as usize % count;
        if let Some(suspense) = self.root.nth_suspense_mut(&mut index) {
            suspense.set_wake_mutation(mutation);
        }
    }

    pub(crate) fn resolve_ready_suspense(&mut self, key: SuspenseReadyKey) {
        self.root.resolve_ready_suspense(key);
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
    pub(crate) dynamics: Vec<DynamicSpec>,
    pub(crate) attrs: Vec<Vec<AttrSpec>>,
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
            dynamics: Vec::new(),
            attrs: Vec::new(),
        }
    }

    pub(crate) fn normalize(mut self) -> Self {
        self.normalize_in_place();
        self
    }

    pub(crate) fn normalize_in_place(&mut self) {
        let dynamic_count = self.template.dynamic_count();
        self.dynamics.resize(dynamic_count, DynamicSpec::Empty);
        self.dynamics.truncate(dynamic_count);

        let attr_count = self.template.attr_count();
        self.attrs.resize(attr_count, Vec::new());
        self.attrs.truncate(attr_count);
        for (slot, attrs) in self.attrs.iter_mut().enumerate() {
            sort_attrs(slot, attrs);
            attrs.truncate(MAX_DYNAMIC_ATTRS);
        }
    }

    pub(crate) fn vnode_count(&self) -> usize {
        1 + self
            .dynamics
            .iter()
            .map(DynamicSpec::vnode_count)
            .sum::<usize>()
    }

    pub(crate) fn nth_vnode_mut(&mut self, index: &mut usize) -> Option<&mut VNodeSpec> {
        if *index == 0 {
            return Some(self);
        }
        *index -= 1;
        for dynamic in &mut self.dynamics {
            if let Some(node) = dynamic.nth_vnode_mut(index) {
                return Some(node);
            }
        }
        None
    }

    pub(crate) fn node_count(&self) -> u64 {
        1 + self.template.node_count()
            + self
                .dynamics
                .iter()
                .map(DynamicSpec::node_count)
                .sum::<u64>()
            + self
                .attrs
                .iter()
                .map(|attrs| attrs.len() as u64)
                .sum::<u64>()
    }

    pub(crate) fn suspense_count(&self) -> usize {
        self.dynamics.iter().map(DynamicSpec::suspense_count).sum()
    }

    pub(crate) fn nth_suspense_mut(&mut self, index: &mut usize) -> Option<&mut SuspenseSpec> {
        for dynamic in &mut self.dynamics {
            if let Some(found) = dynamic.nth_suspense_mut(index) {
                return Some(found);
            }
        }
        None
    }

    pub(crate) fn collect_ready_suspense_keys(&self, out: &mut Vec<SuspenseReadyKey>) {
        for dynamic in &self.dynamics {
            dynamic.collect_ready_suspense_keys(out);
        }
    }

    pub(crate) fn resolve_ready_suspense(&mut self, key: SuspenseReadyKey) {
        for dynamic in &mut self.dynamics {
            dynamic.resolve_ready_suspense(key);
        }
    }

    pub(crate) fn wake_mutation_for_ready_key(
        &self,
        key: SuspenseReadyKey,
    ) -> Option<WakeMutationSpec> {
        self.dynamics
            .iter()
            .find_map(|dynamic| dynamic.wake_mutation_for_ready_key(key))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) enum TemplateCacheKey {
    Expanded(Vec<TemplateNodeSpec>),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct TemplateSpec {
    pub(crate) cache_key: Option<TemplateCacheKey>,
    pub(crate) roots: Vec<TemplateNodeSpec>,
}

impl TemplateSpec {
    pub(crate) fn dynamic_count(&self) -> usize {
        self.roots.iter().map(TemplateNodeSpec::dynamic_count).sum()
    }

    pub(crate) fn attr_count(&self) -> usize {
        self.roots.iter().map(TemplateNodeSpec::attr_count).sum()
    }

    pub(crate) fn node_count(&self) -> u64 {
        self.roots.iter().map(TemplateNodeSpec::node_count).sum()
    }

    pub(crate) fn cache_key(&self) -> TemplateCacheKey {
        self.cache_key
            .clone()
            .unwrap_or_else(|| TemplateCacheKey::Expanded(self.roots.clone()))
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

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) enum TemplateNodeSpec {
    Element {
        tag: u8,
        namespace: Option<u8>,
        attrs: Vec<TemplateAttrSpec>,
        children: Vec<TemplateNodeSpec>,
    },
    Text(u8),
    Dynamic,
}

impl TemplateNodeSpec {
    pub(crate) fn from_kind(kind: &TemplateNodeKind) -> Self {
        match kind {
            TemplateNodeKind::Element { tag, namespace } => Self::Element {
                tag: *tag,
                namespace: *namespace,
                attrs: Vec::new(),
                children: Vec::new(),
            },
            TemplateNodeKind::Text(value) => Self::Text(*value),
            TemplateNodeKind::Dynamic => Self::Dynamic,
        }
    }

    pub(crate) fn set_kind(&mut self, kind: &TemplateNodeKind) {
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
                _ => *self = Self::from_kind(kind),
            },
            TemplateNodeKind::Text(value) => *self = Self::Text(*value),
            TemplateNodeKind::Dynamic => *self = Self::Dynamic,
        }
    }

    pub(crate) fn dynamic_count(&self) -> usize {
        match self {
            Self::Element { children, .. } => {
                children.iter().map(TemplateNodeSpec::dynamic_count).sum()
            }
            Self::Text(_) => 0,
            Self::Dynamic => 1,
        }
    }

    pub(crate) fn attr_count(&self) -> usize {
        match self {
            Self::Element {
                attrs, children, ..
            } => {
                attrs
                    .iter()
                    .filter(|attr| matches!(attr, TemplateAttrSpec::Dynamic))
                    .count()
                    + children
                        .iter()
                        .map(TemplateNodeSpec::attr_count)
                        .sum::<usize>()
            }
            Self::Text(_) | Self::Dynamic => 0,
        }
    }

    pub(crate) fn node_count(&self) -> u64 {
        match self {
            Self::Element {
                attrs, children, ..
            } => {
                1 + attrs.len() as u64
                    + children
                        .iter()
                        .map(TemplateNodeSpec::node_count)
                        .sum::<u64>()
            }
            Self::Text(_) | Self::Dynamic => 1,
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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Mutate)]
pub(crate) enum TemplateNodeKind {
    Element { tag: u8, namespace: Option<u8> },
    Text(u8),
    Dynamic,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Mutate)]
pub(crate) enum TemplateAttrSpec {
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
    ComponentA(Box<VNodeSpec>),
    ComponentB(Box<VNodeSpec>),
    Suspense(SuspenseSpec),
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct SuspenseSpec {
    pub(crate) id: u64,
    pub(crate) ready_generation: u64,
    pub(crate) mode: SuspenseMode,
    pub(crate) wake_mutation: WakeMutationSpec,
    pub(crate) wake_applied: bool,
    pub(crate) child: Box<VNodeSpec>,
}

impl SuspenseSpec {
    pub(crate) fn new(id: u64, mode: SuspenseMode) -> Self {
        Self {
            id,
            ready_generation: 0,
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
        if self.mode != SuspenseMode::Ready && mode == SuspenseMode::Ready {
            self.ready_generation += 1;
        }
        self.mode = mode;
        self.wake_applied = false;
    }

    pub(crate) fn set_wake_mutation(&mut self, mutation: WakeMutationSpec) {
        self.wake_mutation = mutation;
        self.wake_applied = false;
    }

    pub(crate) fn resolve_ready(&mut self) {
        self.mode = SuspenseMode::Resolved;
        self.wake_applied = self.wake_mutation != WakeMutationSpec::None;
    }
}

impl DynamicSpec {
    pub(crate) fn set_kind(&mut self, kind: &DynamicKind, next_suspense_id: &mut u64) {
        match kind {
            DynamicKind::Empty => *self = Self::Empty,
            DynamicKind::Text(value) => *self = Self::Text(*value),
            DynamicKind::Placeholder => *self = Self::Placeholder,
            DynamicKind::Fragment => {
                if !matches!(self, Self::Fragment(_)) {
                    *self = Self::Fragment(Vec::new());
                }
            }
            DynamicKind::ComponentA => {
                if !matches!(self, Self::ComponentA(_)) {
                    *self = Self::ComponentA(Box::new(VNodeSpec::minimal()));
                }
            }
            DynamicKind::ComponentB => {
                if !matches!(self, Self::ComponentB(_)) {
                    *self = Self::ComponentB(Box::new(VNodeSpec::minimal()));
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
        }
    }

    pub(crate) fn vnode_count(&self) -> usize {
        match self {
            Self::Empty | Self::Text(_) | Self::Placeholder => 0,
            Self::Fragment(nodes) => nodes.iter().map(VNodeSpec::vnode_count).sum(),
            Self::ComponentA(node) | Self::ComponentB(node) => node.vnode_count(),
            Self::Suspense(spec) => spec.child.vnode_count(),
        }
    }

    pub(crate) fn nth_vnode_mut(&mut self, index: &mut usize) -> Option<&mut VNodeSpec> {
        match self {
            Self::Empty | Self::Text(_) | Self::Placeholder => None,
            Self::Fragment(nodes) => {
                for node in nodes {
                    if let Some(found) = node.nth_vnode_mut(index) {
                        return Some(found);
                    }
                }
                None
            }
            Self::ComponentA(node) | Self::ComponentB(node) => node.nth_vnode_mut(index),
            Self::Suspense(spec) => spec.child.nth_vnode_mut(index),
        }
    }

    pub(crate) fn node_count(&self) -> u64 {
        match self {
            Self::Empty | Self::Text(_) | Self::Placeholder => 1,
            Self::Fragment(nodes) => 1 + nodes.iter().map(VNodeSpec::node_count).sum::<u64>(),
            Self::ComponentA(node) | Self::ComponentB(node) => 1 + node.node_count(),
            Self::Suspense(spec) => {
                let wake_roots = if spec.wake_mutation.adds_root() { 1 } else { 0 };
                1 + wake_roots + spec.child.node_count()
            }
        }
    }

    pub(crate) fn suspense_count(&self) -> usize {
        match self {
            Self::Empty | Self::Text(_) | Self::Placeholder => 0,
            Self::Fragment(nodes) => nodes.iter().map(VNodeSpec::suspense_count).sum(),
            Self::ComponentA(node) | Self::ComponentB(node) => node.suspense_count(),
            Self::Suspense(spec) => 1 + spec.child.suspense_count(),
        }
    }

    pub(crate) fn nth_suspense_mut(&mut self, index: &mut usize) -> Option<&mut SuspenseSpec> {
        match self {
            Self::Empty | Self::Text(_) | Self::Placeholder => None,
            Self::Fragment(nodes) => {
                for node in nodes {
                    if let Some(found) = node.nth_suspense_mut(index) {
                        return Some(found);
                    }
                }
                None
            }
            Self::ComponentA(node) | Self::ComponentB(node) => node.nth_suspense_mut(index),
            Self::Suspense(spec) => {
                if *index == 0 {
                    return Some(spec);
                }
                *index -= 1;
                spec.child.nth_suspense_mut(index)
            }
        }
    }

    pub(crate) fn collect_ready_suspense_keys(&self, out: &mut Vec<SuspenseReadyKey>) {
        match self {
            Self::Empty | Self::Text(_) | Self::Placeholder => {}
            Self::Fragment(nodes) => {
                for node in nodes {
                    node.collect_ready_suspense_keys(out);
                }
            }
            Self::ComponentA(node) | Self::ComponentB(node) => {
                node.collect_ready_suspense_keys(out)
            }
            Self::Suspense(spec) => {
                if spec.mode == SuspenseMode::Ready {
                    out.push(spec.ready_key());
                }
                spec.child.collect_ready_suspense_keys(out);
            }
        }
    }

    pub(crate) fn resolve_ready_suspense(&mut self, key: SuspenseReadyKey) {
        match self {
            Self::Empty | Self::Text(_) | Self::Placeholder => {}
            Self::Fragment(nodes) => {
                for node in nodes {
                    node.resolve_ready_suspense(key);
                }
            }
            Self::ComponentA(node) | Self::ComponentB(node) => node.resolve_ready_suspense(key),
            Self::Suspense(spec) => {
                if spec.mode == SuspenseMode::Ready && spec.ready_key() == key {
                    spec.resolve_ready();
                }
                spec.child.resolve_ready_suspense(key);
            }
        }
    }

    pub(crate) fn wake_mutation_for_ready_key(
        &self,
        key: SuspenseReadyKey,
    ) -> Option<WakeMutationSpec> {
        match self {
            Self::Empty | Self::Text(_) | Self::Placeholder => None,
            Self::Fragment(nodes) => nodes
                .iter()
                .find_map(|node| node.wake_mutation_for_ready_key(key)),
            Self::ComponentA(node) | Self::ComponentB(node) => {
                node.wake_mutation_for_ready_key(key)
            }
            Self::Suspense(spec) => {
                if spec.ready_key() == key {
                    Some(spec.wake_mutation)
                } else {
                    spec.child.wake_mutation_for_ready_key(key)
                }
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Mutate)]
pub(crate) enum DynamicKind {
    Empty,
    Text(u8),
    Fragment,
    ComponentA,
    ComponentB,
    Suspense { mode: SuspenseMode },
    Placeholder,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Mutate)]
pub(crate) enum SuspenseMode {
    Resolved,
    Pending,
    Ready,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Mutate)]
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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Mutate)]
pub(crate) enum FragmentKeyMode {
    Unkeyed,
    Keyed { base: u8 },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Mutate)]
pub(crate) struct AttrSpec {
    pub(crate) name: u8,
    pub(crate) namespace: Option<u8>,
    pub(crate) value: AttrValueSpec,
    pub(crate) volatile: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Mutate)]
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
        _ => format!("attr{}", attr.name),
    }
}
