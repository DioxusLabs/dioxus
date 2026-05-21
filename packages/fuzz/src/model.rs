use mutatis::Mutate;
use serde::{Deserialize, Serialize};

pub(crate) const MAX_ROOTS: usize = 8;
pub(crate) const MAX_CHILDREN: usize = 8;
pub(crate) const MAX_TEMPLATE_ATTRS: usize = 12;
pub(crate) const MAX_DYNAMIC_ATTRS: usize = 8;
pub(crate) const MAX_FRAGMENT_CHILDREN: usize = 8;
pub(crate) const MAX_MODEL_COST: u64 = 256;
pub(crate) const MAX_READY_WAKE_COUNT: u8 = 4;

// ---------- Spec model ----------------------------------------------------------------------

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

    pub(crate) fn vnode_count(&self) -> usize {
        1 + self.template.vnode_count()
    }

    pub(crate) fn nth_vnode_mut(&mut self, index: &mut usize) -> Option<&mut VNodeSpec> {
        if *index == 0 {
            return Some(self);
        }
        *index -= 1;
        self.template.nth_vnode_mut(index)
    }

    pub(crate) fn node_count(&self) -> u64 {
        1 + self.template.node_count()
    }

    pub(crate) fn suspense_count(&self) -> usize {
        self.template.suspense_count()
    }

    pub(crate) fn nth_suspense_mut(&mut self, index: &mut usize) -> Option<&mut SuspenseSpec> {
        self.template.nth_suspense_mut(index)
    }

    pub(crate) fn collect_ready_suspense_keys(&self, out: &mut Vec<SuspenseReadyKey>) {
        self.template.collect_ready_suspense_keys(out);
    }

    pub(crate) fn wake_ready_suspense(&mut self, key: SuspenseReadyKey) {
        self.template.wake_ready_suspense(key);
    }

    pub(crate) fn wake_mutation_for_ready_key(
        &self,
        key: SuspenseReadyKey,
    ) -> Option<WakeMutationSpec> {
        self.template.wake_mutation_for_ready_key(key)
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

    pub(crate) fn dynamic_count(&self) -> usize {
        self.roots.iter().map(TemplateNodeSpec::dynamic_count).sum()
    }

    pub(crate) fn attr_count(&self) -> usize {
        self.roots.iter().map(TemplateNodeSpec::attr_count).sum()
    }

    pub(crate) fn node_count(&self) -> u64 {
        self.roots.iter().map(TemplateNodeSpec::node_count).sum()
    }

    pub(crate) fn vnode_count(&self) -> usize {
        self.roots.iter().map(TemplateNodeSpec::vnode_count).sum()
    }

    pub(crate) fn nth_vnode_mut(&mut self, index: &mut usize) -> Option<&mut VNodeSpec> {
        for root in &mut self.roots {
            if let Some(found) = root.nth_vnode_mut(index) {
                return Some(found);
            }
        }
        None
    }

    pub(crate) fn nth_dynamic_mut(&mut self, index: &mut usize) -> Option<&mut DynamicSpec> {
        for root in &mut self.roots {
            if let Some(found) = root.nth_dynamic_mut(index) {
                return Some(found);
            }
        }
        None
    }

    pub(crate) fn nth_dynamic_attr_mut(&mut self, index: &mut usize) -> Option<&mut Vec<AttrSpec>> {
        for root in &mut self.roots {
            if let Some(found) = root.nth_dynamic_attr_mut(index) {
                return Some(found);
            }
        }
        None
    }

    pub(crate) fn suspense_count(&self) -> usize {
        self.roots
            .iter()
            .map(TemplateNodeSpec::suspense_count)
            .sum()
    }

    pub(crate) fn nth_suspense_mut(&mut self, index: &mut usize) -> Option<&mut SuspenseSpec> {
        for root in &mut self.roots {
            if let Some(found) = root.nth_suspense_mut(index) {
                return Some(found);
            }
        }
        None
    }

    pub(crate) fn collect_ready_suspense_keys(&self, out: &mut Vec<SuspenseReadyKey>) {
        for root in &self.roots {
            root.collect_ready_suspense_keys(out);
        }
    }

    pub(crate) fn wake_ready_suspense(&mut self, key: SuspenseReadyKey) {
        for root in &mut self.roots {
            root.wake_ready_suspense(key);
        }
    }

    pub(crate) fn wake_mutation_for_ready_key(
        &self,
        key: SuspenseReadyKey,
    ) -> Option<WakeMutationSpec> {
        self.roots
            .iter()
            .find_map(|root| root.wake_mutation_for_ready_key(key))
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

    pub(crate) fn dynamic_count(&self) -> usize {
        match self {
            Self::Element { children, .. } => {
                children.iter().map(TemplateNodeSpec::dynamic_count).sum()
            }
            Self::Text(_) => 0,
            Self::Dynamic(_) => 1,
        }
    }

    pub(crate) fn attr_count(&self) -> usize {
        match self {
            Self::Element {
                attrs, children, ..
            } => {
                attrs
                    .iter()
                    .filter(|attr| matches!(attr, TemplateAttrSpec::Dynamic(_)))
                    .count()
                    + children
                        .iter()
                        .map(TemplateNodeSpec::attr_count)
                        .sum::<usize>()
            }
            Self::Text(_) | Self::Dynamic(_) => 0,
        }
    }

    pub(crate) fn node_count(&self) -> u64 {
        match self {
            Self::Element {
                attrs, children, ..
            } => {
                1 + attrs.len() as u64
                    + attrs.iter().map(TemplateAttrSpec::node_count).sum::<u64>()
                    + children
                        .iter()
                        .map(TemplateNodeSpec::node_count)
                        .sum::<u64>()
            }
            Self::Text(_) => 1,
            Self::Dynamic(dynamic) => 1 + dynamic.node_count(),
        }
    }

    pub(crate) fn vnode_count(&self) -> usize {
        match self {
            Self::Element { children, .. } => {
                children.iter().map(TemplateNodeSpec::vnode_count).sum()
            }
            Self::Text(_) => 0,
            Self::Dynamic(dynamic) => dynamic.vnode_count(),
        }
    }

    pub(crate) fn nth_vnode_mut(&mut self, index: &mut usize) -> Option<&mut VNodeSpec> {
        match self {
            Self::Element { children, .. } => {
                for child in children {
                    if let Some(found) = child.nth_vnode_mut(index) {
                        return Some(found);
                    }
                }
                None
            }
            Self::Text(_) => None,
            Self::Dynamic(dynamic) => dynamic.nth_vnode_mut(index),
        }
    }

    pub(crate) fn nth_dynamic_mut(&mut self, index: &mut usize) -> Option<&mut DynamicSpec> {
        match self {
            Self::Element { children, .. } => {
                for child in children {
                    if let Some(found) = child.nth_dynamic_mut(index) {
                        return Some(found);
                    }
                }
                None
            }
            Self::Text(_) => None,
            Self::Dynamic(dynamic) => {
                if *index == 0 {
                    return Some(dynamic);
                }
                *index -= 1;
                None
            }
        }
    }

    pub(crate) fn nth_dynamic_attr_mut(&mut self, index: &mut usize) -> Option<&mut Vec<AttrSpec>> {
        match self {
            Self::Element {
                attrs, children, ..
            } => {
                for attr in attrs {
                    let TemplateAttrSpec::Dynamic(attrs) = attr else {
                        continue;
                    };
                    if *index == 0 {
                        return Some(attrs);
                    }
                    *index -= 1;
                }

                for child in children {
                    if let Some(found) = child.nth_dynamic_attr_mut(index) {
                        return Some(found);
                    }
                }
                None
            }
            Self::Text(_) | Self::Dynamic(_) => None,
        }
    }

    pub(crate) fn suspense_count(&self) -> usize {
        match self {
            Self::Element { children, .. } => {
                children.iter().map(TemplateNodeSpec::suspense_count).sum()
            }
            Self::Text(_) => 0,
            Self::Dynamic(dynamic) => dynamic.suspense_count(),
        }
    }

    pub(crate) fn nth_suspense_mut(&mut self, index: &mut usize) -> Option<&mut SuspenseSpec> {
        match self {
            Self::Element { children, .. } => {
                for child in children {
                    if let Some(found) = child.nth_suspense_mut(index) {
                        return Some(found);
                    }
                }
                None
            }
            Self::Text(_) => None,
            Self::Dynamic(dynamic) => dynamic.nth_suspense_mut(index),
        }
    }

    pub(crate) fn collect_ready_suspense_keys(&self, out: &mut Vec<SuspenseReadyKey>) {
        match self {
            Self::Element { children, .. } => {
                for child in children {
                    child.collect_ready_suspense_keys(out);
                }
            }
            Self::Text(_) => {}
            Self::Dynamic(dynamic) => dynamic.collect_ready_suspense_keys(out),
        }
    }

    pub(crate) fn wake_ready_suspense(&mut self, key: SuspenseReadyKey) {
        match self {
            Self::Element { children, .. } => {
                for child in children {
                    child.wake_ready_suspense(key);
                }
            }
            Self::Text(_) => {}
            Self::Dynamic(dynamic) => dynamic.wake_ready_suspense(key),
        }
    }

    pub(crate) fn wake_mutation_for_ready_key(
        &self,
        key: SuspenseReadyKey,
    ) -> Option<WakeMutationSpec> {
        match self {
            Self::Element { children, .. } => children
                .iter()
                .find_map(|child| child.wake_mutation_for_ready_key(key)),
            Self::Text(_) => None,
            Self::Dynamic(dynamic) => dynamic.wake_mutation_for_ready_key(key),
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

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Mutate)]
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

    fn node_count(&self) -> u64 {
        match self {
            Self::Static { .. } => 0,
            Self::Dynamic(attrs) => attrs.len() as u64,
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

impl TemplateNodeShape {
    pub(crate) fn dynamic_count(&self) -> usize {
        match self {
            Self::Element { children, .. } => {
                children.iter().map(TemplateNodeShape::dynamic_count).sum()
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
                    .filter(|attr| matches!(attr, TemplateAttrShape::Dynamic))
                    .count()
                    + children
                        .iter()
                        .map(TemplateNodeShape::attr_count)
                        .sum::<usize>()
            }
            Self::Text(_) | Self::Dynamic => 0,
        }
    }
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
        }
    }

    pub(crate) fn vnode_count(&self) -> usize {
        match self {
            Self::Empty | Self::Text(_) | Self::Placeholder => 0,
            Self::Fragment(nodes) => nodes.iter().map(VNodeSpec::vnode_count).sum(),
            Self::ComponentA(component) | Self::ComponentB(component) => {
                component.child.vnode_count()
            }
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
            Self::ComponentA(component) | Self::ComponentB(component) => {
                component.child.nth_vnode_mut(index)
            }
            Self::Suspense(spec) => spec.child.nth_vnode_mut(index),
        }
    }

    pub(crate) fn node_count(&self) -> u64 {
        match self {
            Self::Empty | Self::Text(_) | Self::Placeholder => 1,
            Self::Fragment(nodes) => 1 + nodes.iter().map(VNodeSpec::node_count).sum::<u64>(),
            Self::ComponentA(component) | Self::ComponentB(component) => {
                1 + component.child.node_count()
            }
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
            Self::ComponentA(component) | Self::ComponentB(component) => {
                component.child.suspense_count()
            }
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
            Self::ComponentA(component) | Self::ComponentB(component) => {
                component.child.nth_suspense_mut(index)
            }
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
            Self::ComponentA(component) | Self::ComponentB(component) => {
                component.child.collect_ready_suspense_keys(out)
            }
            Self::Suspense(spec) => {
                if spec.mode.is_ready() {
                    out.push(spec.ready_key());
                }
                spec.child.collect_ready_suspense_keys(out);
            }
        }
    }

    pub(crate) fn wake_ready_suspense(&mut self, key: SuspenseReadyKey) {
        match self {
            Self::Empty | Self::Text(_) | Self::Placeholder => {}
            Self::Fragment(nodes) => {
                for node in nodes {
                    node.wake_ready_suspense(key);
                }
            }
            Self::ComponentA(component) | Self::ComponentB(component) => {
                component.child.wake_ready_suspense(key)
            }
            Self::Suspense(spec) => {
                if spec.mode.is_ready() && spec.ready_key() == key {
                    spec.wake_ready();
                }
                spec.child.wake_ready_suspense(key);
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
            Self::ComponentA(component) | Self::ComponentB(component) => {
                component.child.wake_mutation_for_ready_key(key)
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

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Mutate)]
pub(crate) enum DynamicKind {
    Empty,
    Text(u8),
    Fragment { children: u8, key_base: Option<u8> },
    ComponentA,
    ComponentB,
    Suspense { mode: SuspenseMode },
    Placeholder,
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

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Mutate)]
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
        _ => format!("attr{}", attr.name),
    }
}
