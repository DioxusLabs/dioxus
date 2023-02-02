use dioxus_core::{BorrowedAttributeValue, ElementId, Mutations, TemplateNode};

use rustc_hash::{FxHashMap, FxHashSet};
use std::any::{Any, TypeId};
use std::collections::VecDeque;

use crate::node::{
    ElementNode, FromAnyValue, NodeData, NodeType, OwnedAttributeDiscription, OwnedAttributeValue,
};
use crate::node_ref::{AttributeMask, NodeMask, NodeMaskBuilder};
use crate::passes::{resolve_passes, DirtyNodeStates, TypeErasedPass};
use crate::tree::{EntryBuilder, NodeId, Tree};
use crate::{FxDashSet, SendAnyMap};

/// A Dom that can sync with the VirtualDom mutations intended for use in lazy renderers.
/// The render state passes from parent to children and or accumulates state from children to parents.
/// To get started implement [crate::state::ParentDepState], [crate::state::NodeDepState], or [crate::state::ChildDepState] and call [RealDom::apply_mutations] to update the dom and [RealDom::update_state] to update the state of the nodes.
///
/// # Custom values
/// To allow custom values to be passed into attributes implement FromAnyValue on a type that can represent your custom value and specify the V generic to be that type. If you have many different custom values, it can be useful to use a enum type to represent the varients.
pub struct RealDom<V: FromAnyValue + Send + Sync = ()> {
    pub(crate) tree: Tree,
    /// a map from element id to real node id
    node_id_mapping: Vec<Option<NodeId>>,
    nodes_listening: FxHashMap<String, FxHashSet<NodeId>>,
    stack: Vec<NodeId>,
    templates: FxHashMap<String, Vec<NodeId>>,
    pub(crate) passes: Box<[TypeErasedPass<V>]>,
    passes_updated: FxHashMap<NodeId, FxHashSet<TypeId>>,
    nodes_updated: FxHashMap<NodeId, NodeMask>,
    phantom: std::marker::PhantomData<V>,
}

impl<V: FromAnyValue + Send + Sync> RealDom<V> {
    pub fn new(mut passes: Box<[TypeErasedPass<V>]>) -> RealDom<V> {
        let mut tree = Tree::new();
        tree.insert_slab::<NodeData<V>>();
        for pass in passes.iter() {
            (pass.create)(&mut tree);
        }
        let root_id = tree.root();
        let mut root_node: NodeData<V> = NodeData::new(NodeType::Element(ElementNode {
            tag: "Root".to_string(),
            namespace: Some("Root".to_string()),
            attributes: FxHashMap::default(),
            listeners: FxHashSet::default(),
        }));
        root_node.element_id = Some(ElementId(0));
        root_node.node_id = root_id;
        tree.insert(root_id, root_node);

        // resolve dependants for each pass
        for i in 1..passes.len() {
            let (before, after) = passes.split_at_mut(i);
            let (current, before) = before.split_last_mut().unwrap();
            for pass in before.iter_mut().chain(after.iter_mut()) {
                for dependancy in &current.combined_dependancy_type_ids {
                    if pass.this_type_id == *dependancy {
                        pass.dependants.insert(current.this_type_id);
                    }
                }
            }
        }

        let mut passes_updated = FxHashMap::default();
        let mut nodes_updated = FxHashMap::default();

        let root_id = NodeId(0);
        passes_updated.insert(root_id, passes.iter().map(|x| x.this_type_id).collect());
        nodes_updated.insert(root_id, NodeMaskBuilder::ALL.build());

        RealDom {
            tree,
            node_id_mapping: vec![Some(root_id)],
            nodes_listening: FxHashMap::default(),
            stack: vec![root_id],
            templates: FxHashMap::default(),
            passes,
            passes_updated,
            nodes_updated,
            phantom: std::marker::PhantomData,
        }
    }

    fn mark_dirty(&mut self, node_id: NodeId, mask: NodeMask) {
        self.passes_updated.entry(node_id).or_default().extend(
            self.passes
                .iter()
                .filter_map(|x| x.mask.overlaps(&mask).then_some(x.this_type_id)),
        );
        let nodes_updated = &mut self.nodes_updated;
        if let Some(node) = nodes_updated.get_mut(&node_id) {
            *node = node.union(&mask);
        } else {
            nodes_updated.insert(node_id, mask);
        }
    }

    fn mark_parent_added_or_removed(&mut self, node_id: NodeId) {
        let hm = self.passes_updated.entry(node_id).or_default();
        for pass in &*self.passes {
            if pass.parent_dependant {
                hm.insert(pass.this_type_id);
            }
        }
    }

    fn mark_child_changed(&mut self, node_id: NodeId) {
        let hm = self.passes_updated.entry(node_id).or_default();
        for pass in &*self.passes {
            if pass.child_dependant {
                hm.insert(pass.this_type_id);
            }
        }
    }

    pub fn element_to_node_id(&self, element_id: ElementId) -> NodeId {
        self.node_id_mapping.get(element_id.0).unwrap().unwrap()
    }

    fn set_element_id(&mut self, node_id: NodeId, element_id: ElementId) {
        let mut node = self.tree.get_mut::<NodeData>(node_id).unwrap();
        let node_id = node.node_id;
        node.element_id = Some(element_id);
        if self.node_id_mapping.len() <= element_id.0 {
            self.node_id_mapping.resize(element_id.0 + 1, None);
        }
        self.node_id_mapping[element_id.0] = Some(node_id);
    }

    fn load_child(&self, path: &[u8]) -> NodeId {
        let mut current = *self.stack.last().unwrap();
        for i in path {
            current = self.tree.children_ids(current).unwrap()[*i as usize];
        }
        current
    }

    fn create_node(
        &mut self,
        mut node: NodeData<V>,
        id: Option<ElementId>,
        mark_dirty: bool,
    ) -> EntryBuilder<'_> {
        let mut node_entry = self.tree.create_node();
        let node_id = node_entry.id();
        if mark_dirty {
            self.passes_updated
                .entry(node_id)
                .or_default()
                .extend(self.passes.iter().map(|x| x.this_type_id));
        }
        node.node_id = node_id;
        node.element_id = id;
        node_entry.insert(node);
        node_entry
    }

    fn add_child(&mut self, node_id: NodeId, child_id: NodeId) {
        self.tree.add_child(node_id, child_id);
    }

    fn create_template_node(&mut self, node: &TemplateNode) -> NodeId {
        match node {
            TemplateNode::Element {
                tag,
                namespace,
                attrs,
                children,
            } => {
                let node = NodeData::new(NodeType::Element(ElementNode {
                    tag: tag.to_string(),
                    namespace: namespace.map(|s| s.to_string()),
                    attributes: attrs
                        .iter()
                        .filter_map(|attr| match attr {
                            dioxus_core::TemplateAttribute::Static {
                                name,
                                value,
                                namespace,
                            } => Some((
                                OwnedAttributeDiscription {
                                    namespace: namespace.map(|s| s.to_string()),
                                    name: name.to_string(),
                                    volatile: false,
                                },
                                OwnedAttributeValue::Text(value.to_string()),
                            )),
                            dioxus_core::TemplateAttribute::Dynamic { .. } => None,
                        })
                        .collect(),
                    listeners: FxHashSet::default(),
                }));
                let node_id = self.create_node(node, None, true).id();
                for child in *children {
                    let child_id = self.create_template_node(child);
                    self.add_child(node_id, child_id);
                }
                node_id
            }
            TemplateNode::Text { text } => self
                .create_node(NodeData::new(NodeType::Text(text.to_string())), None, true)
                .id(),
            TemplateNode::Dynamic { .. } => self
                .create_node(NodeData::new(NodeType::Placeholder), None, true)
                .id(),
            TemplateNode::DynamicText { .. } => self
                .create_node(NodeData::new(NodeType::Text(String::new())), None, true)
                .id(),
        }
    }

    /// Updates the dom with some mutations and return a set of nodes that were updated. Pass the dirty nodes to update_state.
    pub fn apply_mutations(&mut self, mutations: Mutations) {
        for template in mutations.templates {
            let mut template_root_ids = Vec::new();
            for root in template.roots {
                let id = self.create_template_node(root);
                template_root_ids.push(id);
            }
            self.templates
                .insert(template.name.to_string(), template_root_ids);
        }

        for e in mutations.edits {
            use dioxus_core::Mutation::*;
            match e {
                AppendChildren { id, m } => {
                    let children = self.stack.split_off(self.stack.len() - m);
                    let parent = self.element_to_node_id(id);
                    for child in children {
                        self.add_child(parent, child);
                    }
                }
                AssignId { path, id } => {
                    self.set_element_id(self.load_child(path), id);
                }
                CreatePlaceholder { id } => {
                    let node = NodeData::new(NodeType::Placeholder);
                    let node_id = self.create_node(node, None, true).id();
                    self.set_element_id(node_id, id);
                    self.stack.push(node_id);
                }
                CreateTextNode { value, id } => {
                    let node_data = NodeData::new(NodeType::Text(value.to_string()));
                    let node_id = self.create_node(node_data, None, true).id();
                    self.set_element_id(node_id, id);
                    self.stack.push(node_id);
                }
                HydrateText { path, value, id } => {
                    let node_id = self.load_child(path);
                    self.set_element_id(node_id, id);
                    let mut node = self.get_mut(node_id).unwrap();
                    if let NodeTypeMut::Text(text) = node.node_type_mut() {
                        *text = value.to_string();
                    } else {
                        node.set_type(NodeType::Text(value.to_string()));
                    }
                }
                LoadTemplate { name, index, id } => {
                    let template_id = self.templates[name][index];
                    let clone_id = self.clone_node(template_id);
                    self.set_element_id(clone_id, id);
                    self.stack.push(clone_id);
                }
                ReplaceWith { id, m } => {
                    let new_nodes = self.stack.split_off(self.stack.len() - m);
                    let old_node_id = self.element_to_node_id(id);
                    for new in new_nodes {
                        self.insert_before(old_node_id, new);
                    }
                    self.remove(old_node_id);
                }
                ReplacePlaceholder { path, m } => {
                    let new_nodes = self.stack.split_off(self.stack.len() - m);
                    let old_node_id = self.load_child(path);
                    for new in new_nodes {
                        self.insert_before(old_node_id, new);
                    }
                    self.remove(old_node_id);
                }
                InsertAfter { id, m } => {
                    let new_nodes = self.stack.split_off(self.stack.len() - m);
                    let old_node_id = self.element_to_node_id(id);
                    for new in new_nodes.into_iter().rev() {
                        self.insert_after(old_node_id, new);
                    }
                }
                InsertBefore { id, m } => {
                    let new_nodes = self.stack.split_off(self.stack.len() - m);
                    let old_node_id = self.element_to_node_id(id);
                    for new in new_nodes {
                        self.tree.insert_before(old_node_id, new);
                    }
                }
                SetAttribute {
                    name,
                    value,
                    id,
                    ns,
                } => {
                    let node_id = self.element_to_node_id(id);
                    let mut node = self.get_mut(node_id).unwrap();
                    if let NodeTypeMut::Element(element) = &mut node.node_type_mut() {
                        if let BorrowedAttributeValue::None = &value {
                            element.remove_attributes(&OwnedAttributeDiscription {
                                name: name.to_string(),
                                namespace: ns.map(|s| s.to_string()),
                                volatile: false,
                            });
                        } else {
                            element.set_attribute(
                                OwnedAttributeDiscription {
                                    name: name.to_string(),
                                    namespace: ns.map(|s| s.to_string()),
                                    volatile: false,
                                },
                                OwnedAttributeValue::from(value),
                            );
                        }
                    }
                }
                SetText { value, id } => {
                    let node_id = self.element_to_node_id(id);
                    let mut node = self.get_mut(node_id).unwrap();
                    if let NodeTypeMut::Text(text) = node.node_type_mut() {
                        *text = value.to_string();
                    }
                }
                NewEventListener { name, id } => {
                    let node_id = self.element_to_node_id(id);
                    let mut node = self.get_mut(node_id).unwrap();
                    if let NodeTypeMut::Element(mut element) = node.node_type_mut() {
                        element.listeners_mut().insert(name.to_string());
                        drop(node);
                        match self.nodes_listening.get_mut(name) {
                            Some(hs) => {
                                hs.insert(node_id);
                            }
                            None => {
                                let mut hs = FxHashSet::default();
                                hs.insert(node_id);
                                self.nodes_listening.insert(name.to_string(), hs);
                            }
                        }
                    }
                }
                RemoveEventListener { id, name } => {
                    let node_id = self.element_to_node_id(id);
                    {
                        let mut node = self.get_mut(node_id).unwrap();
                        if let NodeTypeMut::Element(mut element) = node.node_type_mut() {
                            element.listeners_mut().remove(name);
                        }
                    }
                    self.nodes_listening.get_mut(name).unwrap().remove(&node_id);
                }
                Remove { id } => {
                    let node_id = self.element_to_node_id(id);
                    self.remove(node_id);
                }
                PushRoot { id } => {
                    let node_id = self.element_to_node_id(id);
                    self.stack.push(node_id);
                }
            }
        }
    }

    /// Find all nodes that are listening for an event, sorted by there height in the dom progressing starting at the bottom and progressing up.
    /// This can be useful to avoid creating duplicate events.
    pub fn get_listening_sorted(&self, event: &'static str) -> Vec<NodeRef<V>> {
        if let Some(nodes) = self.nodes_listening.get(event) {
            let mut listening: Vec<_> = nodes
                .iter()
                .map(|id| (*id, self.tree.height(*id).unwrap()))
                .collect();
            listening.sort_by(|(_, h1), (_, h2)| h1.cmp(h2).reverse());
            listening
                .into_iter()
                .map(|(id, _)| NodeRef { id, dom: self })
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Return the number of nodes in the dom.
    pub fn size(&self) -> usize {
        // The dom has a root node, ignore it.
        self.tree.size() - 1
    }

    /// Returns the id of the root node.
    pub fn root_id(&self) -> NodeId {
        self.tree.root()
    }

    fn clone_node(&mut self, node_id: NodeId) -> NodeId {
        let node = self.get(node_id).unwrap();
        let new_node = node.node_data().clone();
        let new_id = self.create_node(new_node, None, true).id();

        let children = self.tree.children_ids(node_id).unwrap().to_vec();
        for child in children {
            let child_id = self.clone_node(child);
            self.add_child(new_id, child_id);
        }
        new_id
    }

    pub fn root(&self) -> NodeId {
        self.tree.root()
    }

    pub fn get(&self, id: impl IntoNodeId<V>) -> Option<NodeRef<'_, V>> {
        let id = id.into_node_id(self);
        self.tree.contains(id).then_some(NodeRef { id, dom: self })
    }

    pub fn get_mut(&mut self, id: impl IntoNodeId<V>) -> Option<NodeMut<'_, V>> {
        let id = id.into_node_id(self);
        self.tree.contains(id).then(|| NodeMut {
            id,
            dirty: NodeMask::default(),
            dom: self,
        })
    }

    /// WARNING: This escapes the reactive system that the real dom uses. Any changes made with this method will not trigger updates in states when [RealDom::update_state] is called.
    pub fn get_mut_raw(&mut self, id: impl IntoNodeId<V>) -> Option<NodeMutRaw<V>> {
        let id = id.into_node_id(self);
        self.tree
            .contains(id)
            .then_some(NodeMutRaw { id, dom: self })
    }

    /// Update the state of the dom, after appling some mutations. This will keep the nodes in the dom up to date with their VNode counterparts.
    pub fn update_state(
        &mut self,
        ctx: SendAnyMap,
    ) -> (FxDashSet<NodeId>, FxHashMap<NodeId, NodeMask>) {
        let passes = std::mem::take(&mut self.passes_updated);
        let nodes_updated = std::mem::take(&mut self.nodes_updated);
        let dirty_nodes = DirtyNodeStates::with_passes(self.passes.iter().map(|p| p.this_type_id));
        for (node_id, passes) in passes {
            // remove any nodes that were created and then removed in the same mutations from the dirty nodes list
            if let Some(height) = self.tree.height(node_id) {
                for pass in passes {
                    dirty_nodes.insert(pass, node_id, height);
                }
            }
        }

        (resolve_passes(self, dirty_nodes, ctx), nodes_updated)
    }

    fn remove(&mut self, id: NodeId) {
        if let Some(parent_id) = self.tree.parent_id(id) {
            self.mark_child_changed(parent_id);
        }
        self.tree.remove(id)
    }

    pub fn replace(&mut self, old: NodeId, new: NodeId) {
        if let Some(parent_id) = self.tree.parent_id(old) {
            self.mark_child_changed(parent_id);
            self.mark_parent_added_or_removed(new);
        }
        self.tree.replace(old, new);
    }

    pub fn insert_before(&mut self, id: NodeId, new: NodeId) {
        if let Some(parent_id) = self.tree.parent_id(id) {
            self.mark_child_changed(parent_id);
            self.mark_parent_added_or_removed(new);
        }
        self.tree.insert_before(id, new);
    }

    pub fn insert_after(&mut self, id: NodeId, new: NodeId) {
        if let Some(parent_id) = self.tree.parent_id(id) {
            self.mark_child_changed(parent_id);
            self.mark_parent_added_or_removed(new);
        }
        self.tree.insert_after(id, new);
    }

    pub fn traverse_depth_first(&self, mut f: impl FnMut(NodeRef<V>)) {
        let mut stack = vec![self.root()];
        while let Some(id) = stack.pop() {
            if let Some(node) = self.get(id) {
                f(node);
                if let Some(children) = self.tree.children_ids(id) {
                    stack.extend(children.iter().copied().rev());
                }
            }
        }
    }

    pub fn traverse_breadth_first(&self, mut f: impl FnMut(NodeRef<V>)) {
        let mut queue = VecDeque::new();
        queue.push_back(self.root());
        while let Some(id) = queue.pop_front() {
            if let Some(node) = self.get(id) {
                f(node);
                if let Some(children) = self.tree.children_ids(id) {
                    for id in children {
                        queue.push_back(*id);
                    }
                }
            }
        }
    }

    pub fn traverse_depth_first_mut(&mut self, mut f: impl FnMut(NodeMut<V>)) {
        let mut stack = vec![self.root()];
        while let Some(id) = stack.pop() {
            if let Some(children) = self.tree.children_ids(id) {
                let children = children.iter().copied().rev().collect::<Vec<_>>();
                if let Some(node) = self.get_mut(id) {
                    let node = node;
                    f(node);
                    stack.extend(children.iter());
                }
            }
        }
    }

    pub fn traverse_breadth_first_mut(&mut self, mut f: impl FnMut(NodeMut<V>)) {
        let mut queue = VecDeque::new();
        queue.push_back(self.root());
        while let Some(id) = queue.pop_front() {
            if let Some(children) = self.tree.children_ids(id) {
                let children = children.to_vec();
                if let Some(node) = self.get_mut(id) {
                    f(node);
                    for id in children {
                        queue.push_back(id);
                    }
                }
            }
        }
    }
}

pub trait IntoNodeId<V: FromAnyValue + Send + Sync> {
    fn into_node_id(self, rdom: &RealDom<V>) -> NodeId;
}

impl<V: FromAnyValue + Send + Sync> IntoNodeId<V> for NodeId {
    fn into_node_id(self, _rdom: &RealDom<V>) -> NodeId {
        self
    }
}

impl<V: FromAnyValue + Send + Sync> IntoNodeId<V> for ElementId {
    fn into_node_id(self, rdom: &RealDom<V>) -> NodeId {
        rdom.element_to_node_id(self)
    }
}

pub trait NodeImmutable<V: FromAnyValue + Send + Sync>: Sized {
    fn real_dom(&self) -> &RealDom<V>;

    fn id(&self) -> NodeId;

    fn mounted_id(&self) -> Option<ElementId> {
        self.node_data().mounted_id()
    }

    fn node_data(&self) -> &NodeData<V> {
        self.get().unwrap()
    }

    fn node_type(&self) -> &NodeType<V> {
        &self.node_data().node_type
    }

    fn get<T: Any + Sync + Send>(&self) -> Option<&T> {
        self.real_dom().tree.get(self.id())
    }

    fn child_ids(&self) -> Option<&[NodeId]> {
        self.real_dom().tree.children_ids(self.id())
    }

    fn children(&self) -> Option<Vec<NodeRef<V>>> {
        self.child_ids().map(|ids| {
            ids.iter()
                .map(|id| NodeRef {
                    id: *id,
                    dom: self.real_dom(),
                })
                .collect()
        })
    }

    fn parent_id(&self) -> Option<NodeId> {
        self.real_dom().tree.parent_id(self.id())
    }

    fn parent(&self) -> Option<NodeRef<V>> {
        self.parent_id().map(|id| NodeRef {
            id,
            dom: self.real_dom(),
        })
    }
}

pub trait NodeMutable<V: FromAnyValue + Send + Sync>: Sized + NodeImmutable<V> {
    fn get_mut<T: Any + Sync + Send>(&mut self) -> Option<&mut T>;
    fn insert<T: Any + Sync + Send>(&mut self, value: T);
}

#[derive(Clone, Copy)]
pub struct NodeRef<'a, V: FromAnyValue + Send + Sync = ()> {
    id: NodeId,
    dom: &'a RealDom<V>,
}

impl<'a, V: FromAnyValue + Send + Sync> NodeImmutable<V> for NodeRef<'a, V> {
    fn real_dom(&self) -> &RealDom<V> {
        self.dom
    }

    fn id(&self) -> NodeId {
        self.id
    }
}

pub struct NodeMut<'a, V: FromAnyValue + Send + Sync = ()> {
    id: NodeId,
    dom: &'a mut RealDom<V>,
    dirty: NodeMask,
}

impl<'a, V: FromAnyValue + Send + Sync> NodeImmutable<V> for NodeMut<'a, V> {
    fn real_dom(&self) -> &RealDom<V> {
        self.dom
    }

    fn id(&self) -> NodeId {
        self.id
    }
}

impl<'a, V: FromAnyValue + Send + Sync> NodeMutable<V> for NodeMut<'a, V> {
    fn get_mut<T: Any + Sync + Send>(&mut self) -> Option<&mut T> {
        todo!("get_mut with mark as dirty")
    }

    fn insert<T: Any + Sync + Send>(&mut self, _: T) {
        todo!("insert with mark as dirty")
    }
}

impl<'a, V: FromAnyValue + Send + Sync> NodeMut<'a, V> {
    pub fn node_type_mut(&mut self) -> NodeTypeMut<'_, V> {
        let Self { id, dom, dirty } = self;
        let node_type = &mut dom.tree.get_mut::<NodeData<V>>(*id).unwrap().node_type;
        match node_type {
            NodeType::Element(element) => NodeTypeMut::Element(ElementNodeMut { element, dirty }),
            NodeType::Text(text) => {
                dirty.set_text();
                NodeTypeMut::Text(text)
            }
            NodeType::Placeholder => NodeTypeMut::Placeholder,
        }
    }

    pub fn set_type(&mut self, new: NodeType<V>) {
        self.dom
            .tree
            .get_mut::<NodeData<V>>(self.id)
            .unwrap()
            .node_type = new;
        self.dirty = NodeMaskBuilder::ALL.build();
    }
}

impl<V: FromAnyValue + Send + Sync> Drop for NodeMut<'_, V> {
    fn drop(&mut self) {
        let node_id = self.node_data().node_id;
        let mask = std::mem::take(&mut self.dirty);
        self.dom.mark_dirty(node_id, mask);
    }
}

pub enum NodeTypeMut<'a, V: FromAnyValue = ()> {
    Element(ElementNodeMut<'a, V>),
    Text(&'a mut String),
    Placeholder,
}

pub struct ElementNodeMut<'a, V: FromAnyValue = ()> {
    element: &'a mut ElementNode<V>,
    dirty: &'a mut NodeMask,
}

impl<V: FromAnyValue> ElementNodeMut<'_, V> {
    pub fn tag(&self) -> &str {
        &self.element.tag
    }

    pub fn tag_mut(&mut self) -> &mut String {
        self.dirty.set_tag();
        &mut self.element.tag
    }

    pub fn namespace(&self) -> Option<&str> {
        self.element.namespace.as_deref()
    }

    pub fn namespace_mut(&mut self) -> &mut Option<String> {
        self.dirty.set_namespace();
        &mut self.element.namespace
    }

    pub fn attributes(&self) -> &FxHashMap<OwnedAttributeDiscription, OwnedAttributeValue<V>> {
        &self.element.attributes
    }

    pub fn attributes_mut(
        &mut self,
    ) -> &mut FxHashMap<OwnedAttributeDiscription, OwnedAttributeValue<V>> {
        self.dirty.add_attributes(AttributeMask::All);
        &mut self.element.attributes
    }

    pub fn set_attribute(
        &mut self,
        name: OwnedAttributeDiscription,
        value: OwnedAttributeValue<V>,
    ) -> Option<OwnedAttributeValue<V>> {
        self.dirty.add_attributes(AttributeMask::single(&name.name));
        self.element.attributes.insert(name, value)
    }

    pub fn remove_attributes(
        &mut self,
        name: &OwnedAttributeDiscription,
    ) -> Option<OwnedAttributeValue<V>> {
        self.dirty.add_attributes(AttributeMask::single(&name.name));
        self.element.attributes.remove(name)
    }

    pub fn get_attribute_mut(
        &mut self,
        name: &OwnedAttributeDiscription,
    ) -> Option<&mut OwnedAttributeValue<V>> {
        self.dirty.add_attributes(AttributeMask::single(&name.name));
        self.element.attributes.get_mut(name)
    }

    pub fn listeners(&self) -> &FxHashSet<String> {
        &self.element.listeners
    }

    pub fn listeners_mut(&mut self) -> &mut FxHashSet<String> {
        self.dirty.set_listeners();
        &mut self.element.listeners
    }
}

pub struct NodeMutRaw<'a, V: FromAnyValue + Send + Sync = ()> {
    id: NodeId,
    dom: &'a mut RealDom<V>,
}

impl<'a, V: FromAnyValue + Send + Sync> NodeImmutable<V> for NodeMutRaw<'a, V> {
    fn real_dom(&self) -> &RealDom<V> {
        self.dom
    }

    fn id(&self) -> NodeId {
        self.id
    }
}

impl<'a, V: FromAnyValue + Send + Sync> NodeMutable<V> for NodeMutRaw<'a, V> {
    fn get_mut<T: Any + Sync + Send>(&mut self) -> Option<&mut T> {
        self.dom.tree.get_mut::<T>(self.id)
    }

    fn insert<T: Any + Sync + Send>(&mut self, value: T) {
        self.dom.tree.insert(self.id, value);
    }
}
