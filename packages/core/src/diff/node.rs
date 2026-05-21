use crate::innerlude::MountId;
use crate::{Attribute, AttributeValue, DynamicNode::*, TemplateAttribute};
use crate::{NoOpMutations, VNode, VirtualDom, WriteMutations};
use core::iter::Peekable;

use crate::{
    TemplateNode,
    arena::ElementId,
    innerlude::{ElementPath, ElementRef, VNodeMount, VText},
    nodes::DynamicNode,
    scopes::ScopeId,
};

fn dynamic_node_is_rendered_in_dom(
    node: &DynamicNode,
    mount: MountId,
    idx: usize,
    dom: &VirtualDom,
) -> bool {
    match node {
        Text(_) | Placeholder(_) => mounted_dynamic_node_is_live(dom, mount, idx),
        Fragment(nodes) => nodes.iter().any(|node| vnode_is_rendered_in_dom(node, dom)),
        Component(_) => {
            let scope_id = dom.get_mounted_dyn_node(mount, idx);
            mounted_dynamic_node_is_live(dom, mount, idx)
                && dom
                    .get_scope(ScopeId(scope_id))
                    .map(|scope| vnode_is_rendered_in_dom(scope.root_node(), dom))
                    .unwrap_or(false)
        }
    }
}

fn vnode_is_rendered_in_dom(node: &VNode, dom: &VirtualDom) -> bool {
    let mount = mounted_mount(node, dom);
    node.template
        .roots()
        .iter()
        .enumerate()
        .any(|(root_idx, root)| {
            if let Some(idx) = root.dynamic_id() {
                dynamic_node_is_rendered_in_dom(&node.dynamic_nodes[idx], mount, idx, dom)
            } else {
                let id = dom.get_mounted_root_node(mount, root_idx);
                mounted_root_node_is_live(id)
            }
        })
}

fn mounted_dynamic_node_is_live(dom: &VirtualDom, mount: MountId, idx: usize) -> bool {
    dom.get_mounted_dyn_node(mount, idx) != usize::MAX
}

fn mounted_root_node_is_live(id: ElementId) -> bool {
    id.0 != 0 && id.0 != usize::MAX
}

fn clear_mounted_root_node(dom: &mut VirtualDom, mount: MountId, idx: usize) {
    dom.set_mounted_root_node(mount, idx, ElementId(usize::MAX));
}

fn clear_mounted_dynamic_node(dom: &mut VirtualDom, mount: MountId, idx: usize) {
    dom.set_mounted_dyn_node(mount, idx, usize::MAX);
}

fn clear_mounted_dynamic_attr(dom: &mut VirtualDom, mount: MountId, idx: usize) {
    dom.set_mounted_dyn_attr(mount, idx, ElementId(usize::MAX));
}

fn mounted_mount(node: &VNode, dom: &VirtualDom) -> MountId {
    let mount = node.mount.get();
    let mount = mount
        .as_usize()
        .map(MountId)
        .expect("node should already be mounted");
    debug_assert!(dom.runtime.mounts.borrow().contains(mount.0));
    mount
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct AttributeKey {
    name: &'static str,
    namespace: Option<&'static str>,
}

#[derive(Clone, Copy)]
enum ResolvedAttribute<'a> {
    Missing,
    Static(&'static str),
    Dynamic(&'a Attribute),
}

impl AttributeKey {
    fn from_attribute(attribute: &Attribute) -> Self {
        Self {
            name: attribute.name,
            namespace: attribute.namespace,
        }
    }

    fn matches(self, attribute: &Attribute) -> bool {
        self.name == attribute.name && self.namespace == attribute.namespace
    }
}

impl<'a> ResolvedAttribute<'a> {
    fn is_listener(&self) -> bool {
        matches!(
            self,
            ResolvedAttribute::Dynamic(attribute)
                if matches!(attribute.value, AttributeValue::Listener(_))
        )
    }

    fn volatile(&self) -> bool {
        match self {
            ResolvedAttribute::Dynamic(attribute) => attribute.volatile,
            ResolvedAttribute::Static(_) | ResolvedAttribute::Missing => false,
        }
    }
}

impl VNode {
    pub(crate) fn diff_node(
        &self,
        new: &VNode,
        dom: &mut VirtualDom,
        mut to: Option<&mut impl WriteMutations>,
    ) {
        let mount_id = mounted_mount(self, dom);

        // The node we are diffing from should always be mounted
        debug_assert!(
            dom.runtime
                .mounts
                .borrow()
                .get(self.mount.get().0)
                .is_some()
        );

        // If the templates are different, we need to replace the entire template
        if self.template != new.template {
            let parent = dom.get_mounted_parent(mount_id);
            return self.replace(std::slice::from_ref(new), parent, dom, to);
        }

        self.move_mount_to(new, dom);

        // If the templates are the same, we don't need to do anything, except copy over the mount information
        if self == new {
            return;
        }

        // If the templates are the same, we can diff the attributes and children
        // Start with the attributes
        // Since the attributes are only side effects, we can skip diffing them entirely if the node is suspended and we aren't outputting mutations
        if let Some(to) = to.as_deref_mut() {
            self.diff_attributes(new, dom, to);
        }

        // Now diff the dynamic nodes
        let mount_id = new.mount.get();
        for (dyn_node_idx, (old, new)) in self
            .dynamic_nodes
            .iter()
            .zip(new.dynamic_nodes.iter())
            .enumerate()
        {
            self.diff_dynamic_node(mount_id, dyn_node_idx, old, new, dom, to.as_deref_mut())
        }
    }

    fn move_mount_to(&self, new: &VNode, dom: &mut VirtualDom) {
        // Copy over the mount information
        let mount_id = self.mount.take();
        new.mount.set(mount_id);

        debug_assert!(mount_id.mounted());
        let mut mounts = dom.runtime.mounts.borrow_mut();
        let mount = &mut mounts[mount_id.0];

        // Update the reference to the node for bubbling events
        mount.node = new.clone();
    }

    fn diff_dynamic_node(
        &self,
        mount: MountId,
        idx: usize,
        old_node: &DynamicNode,
        new_node: &DynamicNode,
        dom: &mut VirtualDom,
        mut to: Option<&mut impl WriteMutations>,
    ) {
        tracing::trace!("diffing dynamic node from {old_node:?} to {new_node:?}");
        match (old_node, new_node) {
            (Text(old), Text(new)) => {
                // Diffing text is just a side effect, if we are diffing suspended nodes and are not outputting mutations, we can skip it
                if let Some(to) = to {
                    let id = ElementId(dom.get_mounted_dyn_node(mount, idx));
                    self.diff_vtext(to, id, old, new)
                }
            }
            (Placeholder(_), Placeholder(_)) => {}
            (Fragment(old), Fragment(new)) => dom.diff_non_empty_fragment(
                to,
                old,
                new,
                Some(self.reference_to_dynamic_node(mount, idx)),
            ),
            (Component(old), Component(new)) => {
                let scope_id = ScopeId(dom.get_mounted_dyn_node(mount, idx));
                self.diff_vcomponent(
                    mount,
                    idx,
                    new,
                    old,
                    scope_id,
                    Some(self.reference_to_dynamic_node(mount, idx)),
                    dom,
                    to,
                )
            }
            (old, new) => {
                // TODO: we should pass around the mount instead of the mount id
                // that would make moving the mount around here much easier

                // Mark the mount as unused. When a scope is created, it reads the mount and
                // if it is the placeholder value, it will create the scope, otherwise it will
                // reuse the scope
                let old_mount = dom.get_mounted_dyn_node(mount, idx);
                let old_has_live_dom = dynamic_node_is_rendered_in_dom(old, mount, idx, dom);
                clear_mounted_dynamic_node(dom, mount, idx);

                let new_nodes_on_stack = self.create_dynamic_node(
                    new,
                    mount,
                    idx,
                    dom,
                    if old_has_live_dom {
                        to.as_deref_mut()
                    } else {
                        None
                    },
                );

                // Restore the mount for the scope we are removing
                let new_mount = dom.get_mounted_dyn_node(mount, idx);
                dom.set_mounted_dyn_node(mount, idx, old_mount);

                self.remove_dynamic_node(
                    mount,
                    dom,
                    if old_has_live_dom { to } else { None },
                    true,
                    idx,
                    old,
                    old_has_live_dom.then_some(new_nodes_on_stack),
                );

                // Restore the mount for the node we created
                dom.set_mounted_dyn_node(mount, idx, new_mount);
            }
        };
    }

    /// Try to get the dynamic node and its index for a root node
    pub(crate) fn get_dynamic_root_node_and_id(
        &self,
        root_idx: usize,
    ) -> Option<(usize, &DynamicNode)> {
        self.template.roots()[root_idx]
            .dynamic_id()
            .map(|id| (id, &self.dynamic_nodes[id]))
    }

    pub(crate) fn find_first_element(&self, dom: &VirtualDom) -> ElementId {
        let mount_id = self.mount.get();
        let first = match self.get_dynamic_root_node_and_id(0) {
            // This node is static, just get the root id
            None => dom.get_mounted_root_node(mount_id, 0),
            // If it is dynamic and shallow, grab the id from the mounted dynamic nodes
            Some((idx, Placeholder(_) | Text(_))) => {
                ElementId(dom.get_mounted_dyn_node(mount_id, idx))
            }
            // The node is a fragment, so we need to find the first element in the fragment
            Some((_, Fragment(children))) => {
                let child = children.first().unwrap();
                child.find_first_element(dom)
            }
            // The node is a component, so we need to find the first element in the component
            Some((id, Component(_))) => {
                let scope = ScopeId(dom.get_mounted_dyn_node(mount_id, id));
                dom.get_scope(scope)
                    .unwrap()
                    .root_node()
                    .find_first_element(dom)
            }
        };

        // The first element should never be the default element id (the root element)
        debug_assert_ne!(first, ElementId::default());

        first
    }

    pub(crate) fn find_last_element(&self, dom: &VirtualDom) -> ElementId {
        let mount_id = self.mount.get();
        let last_root_index = self.template.roots().len() - 1;
        let last = match self.get_dynamic_root_node_and_id(last_root_index) {
            // This node is static, just get the root id
            None => dom.get_mounted_root_node(mount_id, last_root_index),
            // If it is dynamic and shallow, grab the id from the mounted dynamic nodes
            Some((idx, Placeholder(_) | Text(_))) => {
                ElementId(dom.get_mounted_dyn_node(mount_id, idx))
            }
            // The node is a fragment, so we need to find the last element in the fragment
            Some((_, Fragment(children))) => {
                let child = children.last().unwrap();
                child.find_last_element(dom)
            }
            // The node is a component, so we need to find the first element in the component
            Some((id, Component(_))) => {
                let scope = ScopeId(dom.get_mounted_dyn_node(mount_id, id));
                dom.get_scope(scope)
                    .unwrap()
                    .root_node()
                    .find_last_element(dom)
            }
        };

        // The last element should never be the default element id (the root element)
        debug_assert_ne!(last, ElementId::default());

        last
    }

    /// Diff the two text nodes
    ///
    /// This just sets the text of the node if it's different.
    fn diff_vtext(&self, to: &mut impl WriteMutations, id: ElementId, left: &VText, right: &VText) {
        if left.value != right.value {
            to.set_node_text(&right.value, id);
        }
    }

    pub(crate) fn replace(
        &self,
        right: &[VNode],
        parent: Option<ElementRef>,
        dom: &mut VirtualDom,
        to: Option<&mut impl WriteMutations>,
    ) {
        self.replace_inner(right, parent, dom, to, true)
    }

    /// Replace this node with new children, but *don't destroy* the old node's component state
    ///
    /// This is useful for moving a node from the rendered nodes into a suspended node
    pub(crate) fn move_node_to_background(
        &self,
        right: &[VNode],
        parent: Option<ElementRef>,
        dom: &mut VirtualDom,
        to: Option<&mut impl WriteMutations>,
    ) {
        self.replace_inner(right, parent, dom, to, false)
    }

    pub(crate) fn replace_inner(
        &self,
        right: &[VNode],
        parent: Option<ElementRef>,
        dom: &mut VirtualDom,
        mut to: Option<&mut impl WriteMutations>,
        destroy_component_state: bool,
    ) {
        if !vnode_is_rendered_in_dom(self, dom) {
            let _ = dom.create_children(None::<&mut NoOpMutations>, right, parent);
            self.remove_node_inner(
                dom,
                None::<&mut NoOpMutations>,
                destroy_component_state,
                None,
            );
            return;
        }

        let m = dom.create_children(to.as_deref_mut(), right, parent);

        // Instead of *just* removing it, we can use the replace mutation
        self.remove_node_inner(dom, to, destroy_component_state, Some(m))
    }

    /// Remove a node from the dom and potentially replace it with the top m nodes from the stack
    pub(crate) fn remove_node<M: WriteMutations>(
        &self,
        dom: &mut VirtualDom,
        to: Option<&mut M>,
        replace_with: Option<usize>,
    ) {
        self.remove_node_inner(dom, to, true, replace_with)
    }

    /// Remove a node, but only maybe destroy the component state of that node. During suspense, we need to remove a node from the real dom without wiping the component state
    pub(crate) fn remove_node_inner<M: WriteMutations>(
        &self,
        dom: &mut VirtualDom,
        to: Option<&mut M>,
        destroy_component_state: bool,
        replace_with: Option<usize>,
    ) {
        let mount = mounted_mount(self, dom);

        // Clean up any attributes that have claimed a static node as dynamic for mount/unmounts
        // Will not generate mutations!
        self.reclaim_attributes(mount, dom);

        // Remove the nested dynamic nodes
        // We don't generate mutations for these, as they will be removed by the parent (in the next line)
        // But we still need to make sure to reclaim them from the arena and drop their hooks, etc
        self.remove_nested_dyn_nodes::<M>(mount, dom, destroy_component_state);

        // Clean up the roots, assuming we need to generate mutations for these
        // This is done last in order to preserve Node ID reclaim order (reclaim in reverse order of claim)
        self.reclaim_roots(mount, dom, to, destroy_component_state, replace_with);

        if destroy_component_state {
            let mount = self.mount.take();
            // Remove the mount information
            dom.runtime.mounts.borrow_mut().remove(mount.0);
        }
    }

    fn reclaim_roots(
        &self,
        mount: MountId,
        dom: &mut VirtualDom,
        mut to: Option<&mut impl WriteMutations>,
        destroy_component_state: bool,
        replace_with: Option<usize>,
    ) {
        let roots = self.template.roots();
        for (idx, node) in roots.iter().enumerate() {
            let last_node = idx == roots.len() - 1;
            if let Some(id) = node.dynamic_id() {
                let dynamic_node = &self.dynamic_nodes[id];
                self.remove_dynamic_node(
                    mount,
                    dom,
                    to.as_deref_mut(),
                    destroy_component_state,
                    id,
                    dynamic_node,
                    replace_with.filter(|_| last_node),
                );
            } else {
                let id = dom.get_mounted_root_node(mount, idx);
                if let Some(to) = to.as_deref_mut() {
                    if let (true, Some(replace_with)) = (last_node, replace_with) {
                        to.replace_node_with(id, replace_with);
                    } else {
                        to.remove_node(id);
                    }
                }
                dom.reclaim(id);
                // Stamp the slot so a later traversal cannot mistake the
                // reclaimed id for a live element.
                clear_mounted_root_node(dom, mount, idx);
            }
        }
    }

    fn remove_nested_dyn_nodes<M: WriteMutations>(
        &self,
        mount: MountId,
        dom: &mut VirtualDom,
        destroy_component_state: bool,
    ) {
        let template = self.template;
        for (idx, dyn_node) in self.dynamic_nodes.iter().enumerate() {
            let path_len = template.node_paths().get(idx).map(|path| path.len());
            // Roots are cleaned up automatically above and nodes with a empty path are placeholders
            if let Some(2..) = path_len {
                self.remove_dynamic_node(
                    mount,
                    dom,
                    Option::<&mut M>::None,
                    destroy_component_state,
                    idx,
                    dyn_node,
                    None,
                )
            }
        }
    }

    fn remove_dynamic_node(
        &self,
        mount: MountId,
        dom: &mut VirtualDom,
        mut to: Option<&mut impl WriteMutations>,
        destroy_component_state: bool,
        idx: usize,
        node: &DynamicNode,
        replace_with: Option<usize>,
    ) {
        match node {
            Component(_comp) => {
                let scope_id = ScopeId(dom.get_mounted_dyn_node(mount, idx));
                dom.remove_component_node(to, destroy_component_state, scope_id, replace_with);
            }
            Text(_) | Placeholder(_) => {
                Self::remove_anchor(dom, to, mount, idx, replace_with);
            }
            Fragment(nodes) => {
                for node in &nodes[..nodes.len() - 1] {
                    node.remove_node_inner(dom, to.as_deref_mut(), destroy_component_state, None)
                }
                let last_node = nodes
                    .last()
                    .expect("fragment dynamic nodes should be normalized to non-empty fragments");
                last_node.remove_node_inner(dom, to, destroy_component_state, replace_with)
            }
        };
    }

    fn remove_anchor(
        dom: &mut VirtualDom,
        to: Option<&mut impl WriteMutations>,
        mount: MountId,
        idx: usize,
        replace_with: Option<usize>,
    ) {
        let id = ElementId(dom.get_mounted_dyn_node(mount, idx));
        let removing_live_anchor = to.is_some() && replace_with.is_none();
        if id != ElementId(usize::MAX) {
            if let Some(to) = to {
                if let Some(replace_with) = replace_with {
                    to.replace_node_with(id, replace_with);
                } else {
                    to.remove_node(id);
                }
            }
        }
        debug_assert!(
            id != ElementId(usize::MAX) || !removing_live_anchor,
            "attempted to remove an unmounted dynamic anchor from the live DOM"
        );
        dom.reclaim(id);
        // Stamp the slot so a later traversal cannot mistake the reclaimed id
        // for a live anchor.
        clear_mounted_dynamic_node(dom, mount, idx);
    }

    pub(super) fn reclaim_attributes(&self, mount: MountId, dom: &mut VirtualDom) {
        let mut next_id = None;
        for (idx, path) in self.template.attr_paths().iter().enumerate() {
            // We clean up the roots in the next step, so don't worry about them here
            if path.len() <= 1 {
                continue;
            }

            // only reclaim the new element if it's different from the previous one
            let new_id = dom.get_mounted_dyn_attr(mount, idx);
            if Some(new_id) != next_id {
                dom.reclaim(new_id);
                next_id = Some(new_id);
            }
            clear_mounted_dynamic_attr(dom, mount, idx);
        }
    }

    pub(super) fn diff_attributes(
        &self,
        new: &VNode,
        dom: &mut VirtualDom,
        to: &mut impl WriteMutations,
    ) {
        let mount_id = new.mount.get();
        let attr_paths = self.template.attr_paths();
        let mut attr_group = 0..0;
        let mut delayed_keys = Vec::new();

        for (idx, (old_attrs, new_attrs)) in self
            .dynamic_attrs
            .iter()
            .zip(new.dynamic_attrs.iter())
            .enumerate()
        {
            let mut old_attributes_iter = old_attrs.iter().peekable();
            let mut new_attributes_iter = new_attrs.iter().peekable();
            let attribute_id = dom.get_mounted_dyn_attr(mount_id, idx);
            let path = attr_paths[idx];
            if idx == attr_group.end {
                attr_group = self.dynamic_attribute_group_starting_at(idx);
                delayed_keys.clear();
            }

            loop {
                match (old_attributes_iter.peek(), new_attributes_iter.peek()) {
                    (Some(old_attribute), Some(new_attribute)) => {
                        match Self::attribute_key_cmp(
                            AttributeKey::from_attribute(old_attribute),
                            AttributeKey::from_attribute(new_attribute),
                        ) {
                            std::cmp::Ordering::Equal => {
                                let old = old_attributes_iter.next().unwrap();
                                let new_attribute = new_attributes_iter.next().unwrap();
                                let key = AttributeKey::from_attribute(new_attribute);
                                if self.diff_resolved_attribute_if_needed(
                                    new,
                                    path,
                                    attr_group.clone(),
                                    key,
                                    attribute_id,
                                    mount_id,
                                    dom,
                                    to,
                                    &mut delayed_keys,
                                ) {
                                    continue;
                                }

                                self.diff_dynamic_attribute(
                                    path,
                                    old,
                                    new_attribute,
                                    attribute_id,
                                    mount_id,
                                    dom,
                                    to,
                                );
                            }
                            std::cmp::Ordering::Less => {
                                let old = old_attributes_iter.next().unwrap();
                                let key = AttributeKey::from_attribute(old);
                                if self.diff_resolved_attribute_if_needed(
                                    new,
                                    path,
                                    attr_group.clone(),
                                    key,
                                    attribute_id,
                                    mount_id,
                                    dom,
                                    to,
                                    &mut delayed_keys,
                                ) {
                                    continue;
                                }

                                self.remove_attribute(old, attribute_id, to)
                            }
                            std::cmp::Ordering::Greater => {
                                let new_attribute = new_attributes_iter.next().unwrap();
                                let key = AttributeKey::from_attribute(new_attribute);
                                if self.diff_resolved_attribute_if_needed(
                                    new,
                                    path,
                                    attr_group.clone(),
                                    key,
                                    attribute_id,
                                    mount_id,
                                    dom,
                                    to,
                                    &mut delayed_keys,
                                ) {
                                    continue;
                                }

                                self.write_attribute(
                                    path,
                                    new_attribute,
                                    attribute_id,
                                    mount_id,
                                    dom,
                                    to,
                                );
                            }
                        }
                    }
                    (Some(_), None) => {
                        let old = old_attributes_iter.next().unwrap();
                        let key = AttributeKey::from_attribute(old);
                        if self.diff_resolved_attribute_if_needed(
                            new,
                            path,
                            attr_group.clone(),
                            key,
                            attribute_id,
                            mount_id,
                            dom,
                            to,
                            &mut delayed_keys,
                        ) {
                            continue;
                        }

                        self.remove_attribute(old, attribute_id, to)
                    }
                    (None, Some(_)) => {
                        let new_attribute = new_attributes_iter.next().unwrap();
                        let key = AttributeKey::from_attribute(new_attribute);
                        if self.diff_resolved_attribute_if_needed(
                            new,
                            path,
                            attr_group.clone(),
                            key,
                            attribute_id,
                            mount_id,
                            dom,
                            to,
                            &mut delayed_keys,
                        ) {
                            continue;
                        }

                        self.write_attribute(path, new_attribute, attribute_id, mount_id, dom, to)
                    }
                    (None, None) => break,
                }
            }
        }
    }

    fn remove_attribute(&self, attribute: &Attribute, id: ElementId, to: &mut impl WriteMutations) {
        match &attribute.value {
            AttributeValue::Listener(_) => {
                to.remove_event_listener(&attribute.name[2..], id);
            }
            _ => {
                to.set_attribute(
                    attribute.name,
                    attribute.namespace,
                    &AttributeValue::None,
                    id,
                );
            }
        }
    }

    fn attribute_key_cmp(left: AttributeKey, right: AttributeKey) -> std::cmp::Ordering {
        left.name
            .cmp(right.name)
            .then_with(|| left.namespace.cmp(&right.namespace))
    }

    fn diff_resolved_attribute_if_needed(
        &self,
        new: &VNode,
        path: &'static [u8],
        attr_group: std::ops::Range<usize>,
        key: AttributeKey,
        id: ElementId,
        mount: MountId,
        dom: &mut VirtualDom,
        to: &mut impl WriteMutations,
        delayed_keys: &mut Vec<AttributeKey>,
    ) -> bool {
        if !self.attribute_key_needs_resolved_diff(new, path, attr_group.clone(), key) {
            return false;
        }

        if delayed_keys.contains(&key) {
            return true;
        }
        delayed_keys.push(key);

        let old = self.resolve_attribute_for_group(path, attr_group.clone(), key);
        let new = new.resolve_attribute_for_group(path, attr_group, key);
        self.diff_resolved_attribute(path, key, id, mount, old, new, dom, to);
        true
    }

    fn dynamic_attribute_group_starting_at(&self, start: usize) -> std::ops::Range<usize> {
        let attr_paths = self.template.attr_paths();
        let path = attr_paths[start];
        let mut end = start + 1;

        while end < attr_paths.len() && attr_paths[end] == path {
            end += 1;
        }

        start..end
    }

    fn attribute_key_needs_resolved_diff(
        &self,
        new: &VNode,
        path: &'static [u8],
        attr_group: std::ops::Range<usize>,
        key: AttributeKey,
    ) -> bool {
        if self.static_template_attribute_value(path, key).is_some() {
            return true;
        }

        self.dynamic_attr_key_is_repeated_in_group(attr_group.clone(), key)
            || new.dynamic_attr_key_is_repeated_in_group(attr_group.clone(), key)
            || matches!(
                (
                    self.first_dynamic_attr_slot_with_key(attr_group.clone(), key),
                    new.first_dynamic_attr_slot_with_key(attr_group, key),
                ),
                (Some(old_idx), Some(new_idx)) if old_idx != new_idx
            )
    }

    fn first_dynamic_attr_slot_with_key(
        &self,
        mut attr_group: std::ops::Range<usize>,
        key: AttributeKey,
    ) -> Option<usize> {
        attr_group.find(|idx| {
            self.dynamic_attrs[*idx]
                .iter()
                .any(|attr| key.matches(attr))
        })
    }

    fn dynamic_attr_key_is_repeated_in_group(
        &self,
        attr_group: std::ops::Range<usize>,
        key: AttributeKey,
    ) -> bool {
        let mut found = false;

        for idx in attr_group {
            for attr in &self.dynamic_attrs[idx][..] {
                if key.matches(attr) {
                    if found {
                        return true;
                    }
                    found = true;
                }
            }
        }

        false
    }

    fn resolve_attribute_for_group(
        &self,
        path: &'static [u8],
        attr_group: std::ops::Range<usize>,
        key: AttributeKey,
    ) -> ResolvedAttribute<'_> {
        let mut resolved = self
            .static_template_attribute_value(path, key)
            .map(ResolvedAttribute::Static)
            .unwrap_or(ResolvedAttribute::Missing);

        for idx in attr_group {
            for attr in &self.dynamic_attrs[idx][..] {
                if key.matches(attr) {
                    resolved = if matches!(attr.value, AttributeValue::None) {
                        ResolvedAttribute::Missing
                    } else {
                        ResolvedAttribute::Dynamic(attr)
                    };
                }
            }
        }

        resolved
    }

    fn diff_dynamic_attribute(
        &self,
        path: &'static [u8],
        old: &Attribute,
        new: &Attribute,
        id: ElementId,
        mount: MountId,
        dom: &mut VirtualDom,
        to: &mut impl WriteMutations,
    ) {
        if Self::attribute_is_listener(old) != Self::attribute_is_listener(new) {
            self.remove_attribute(old, id, to);
            self.write_attribute(path, new, id, mount, dom, to);
            return;
        }

        if Self::attribute_is_listener(new) {
            return;
        }

        if old.volatile || new.volatile || Self::attribute_value_changed(old, new) {
            self.write_attribute(path, new, id, mount, dom, to);
        }
    }

    fn diff_resolved_attribute(
        &self,
        path: &'static [u8],
        key: AttributeKey,
        id: ElementId,
        mount: MountId,
        old: ResolvedAttribute<'_>,
        new: ResolvedAttribute<'_>,
        dom: &mut VirtualDom,
        to: &mut impl WriteMutations,
    ) {
        if old.is_listener() != new.is_listener() {
            self.remove_resolved_attribute(key, old, id, to);
            self.write_resolved_attribute(path, key, new, id, mount, dom, to);
            return;
        }

        if new.is_listener() {
            return;
        }

        if old.volatile() || new.volatile() || Self::resolved_attribute_changed(old, new) {
            match new {
                ResolvedAttribute::Missing => self.remove_resolved_attribute(key, old, id, to),
                _ => self.write_resolved_attribute(path, key, new, id, mount, dom, to),
            }
        }
    }

    fn attribute_is_listener(attribute: &Attribute) -> bool {
        matches!(attribute.value, AttributeValue::Listener(_))
    }

    fn attribute_value_changed(old: &Attribute, new: &Attribute) -> bool {
        match (&old.value, &new.value) {
            (AttributeValue::Text(left), AttributeValue::Text(right)) => left != right,
            (AttributeValue::Float(left), AttributeValue::Float(right)) => left != right,
            (AttributeValue::Int(left), AttributeValue::Int(right)) => left != right,
            (AttributeValue::Bool(left), AttributeValue::Bool(right)) => left != right,
            (AttributeValue::Any(left), AttributeValue::Any(right)) => {
                !left.as_ref().any_cmp(right.as_ref())
            }
            (AttributeValue::None, AttributeValue::None) => false,
            (AttributeValue::Listener(_), AttributeValue::Listener(_)) => false,
            _ => true,
        }
    }

    fn resolved_attribute_changed(old: ResolvedAttribute<'_>, new: ResolvedAttribute<'_>) -> bool {
        match (old, new) {
            (ResolvedAttribute::Missing, ResolvedAttribute::Missing) => false,
            (ResolvedAttribute::Missing, _) | (_, ResolvedAttribute::Missing) => true,
            (ResolvedAttribute::Static(left), ResolvedAttribute::Static(right)) => left != right,
            (ResolvedAttribute::Static(left), ResolvedAttribute::Dynamic(right)) => {
                !matches!(&right.value, AttributeValue::Text(right) if left == right)
            }
            (ResolvedAttribute::Dynamic(left), ResolvedAttribute::Static(right)) => {
                !matches!(&left.value, AttributeValue::Text(left) if left == right)
            }
            (ResolvedAttribute::Dynamic(left), ResolvedAttribute::Dynamic(right)) => {
                Self::attribute_value_changed(left, right)
            }
        }
    }

    fn remove_resolved_attribute(
        &self,
        key: AttributeKey,
        attribute: ResolvedAttribute<'_>,
        id: ElementId,
        to: &mut impl WriteMutations,
    ) {
        match attribute {
            ResolvedAttribute::Missing => {}
            ResolvedAttribute::Dynamic(attribute)
                if matches!(attribute.value, AttributeValue::Listener(_)) =>
            {
                self.remove_attribute(attribute, id, to);
            }
            _ => {
                to.set_attribute(key.name, key.namespace, &AttributeValue::None, id);
            }
        }
    }

    fn write_resolved_attribute(
        &self,
        path: &'static [u8],
        key: AttributeKey,
        attribute: ResolvedAttribute<'_>,
        id: ElementId,
        mount: MountId,
        dom: &mut VirtualDom,
        to: &mut impl WriteMutations,
    ) {
        match attribute {
            ResolvedAttribute::Missing => self.remove_resolved_attribute(key, attribute, id, to),
            ResolvedAttribute::Static(value) => {
                let value = AttributeValue::Text(value.to_string());
                to.set_attribute(key.name, key.namespace, &value, id);
            }
            ResolvedAttribute::Dynamic(attribute) => {
                self.write_attribute(path, attribute, id, mount, dom, to);
            }
        }
    }

    fn static_template_attribute_value(
        &self,
        path: &'static [u8],
        key: AttributeKey,
    ) -> Option<&'static str> {
        let mut value = None;

        if let Some(TemplateNode::Element { attrs, .. }) = self.template_node_at_path(path) {
            for attr in attrs.iter() {
                if let TemplateAttribute::Static {
                    name,
                    value: static_value,
                    namespace,
                } = attr
                    && key.name == *name
                    && key.namespace == *namespace
                {
                    value = Some(*static_value);
                }
            }
        }

        value
    }

    fn template_node_at_path(&self, path: &'static [u8]) -> Option<&'static TemplateNode> {
        let (root_idx, child_path) = path.split_first()?;
        let mut node = self.template.roots().get(*root_idx as usize)?;

        for child_idx in child_path {
            let TemplateNode::Element { children, .. } = node else {
                return None;
            };
            node = children.get(*child_idx as usize)?;
        }

        Some(node)
    }

    fn write_attribute(
        &self,
        path: &'static [u8],
        attribute: &Attribute,
        id: ElementId,
        mount: MountId,
        dom: &mut VirtualDom,
        to: &mut impl WriteMutations,
    ) {
        match &attribute.value {
            AttributeValue::Listener(_) => {
                let element_ref = ElementRef {
                    path: ElementPath { path },
                    mount,
                };
                let mut elements = dom.runtime.elements.borrow_mut();
                elements[id.0] = Some(element_ref);
                to.create_event_listener(&attribute.name[2..], id);
            }
            _ => {
                to.set_attribute(attribute.name, attribute.namespace, &attribute.value, id);
            }
        }
    }

    /// Create this rsx block. This will create scopes from components that this rsx block contains, but it will not write anything to the DOM.
    pub(crate) fn create(
        &self,
        dom: &mut VirtualDom,
        parent: Option<ElementRef>,
        mut to: Option<&mut impl WriteMutations>,
    ) -> usize {
        // Get the most up to date template
        let template = self.template;

        // Initialize the mount information for this vnode if it isn't already mounted
        if !self.mount.get().mounted() {
            let mut mounts = dom.runtime.mounts.borrow_mut();
            let entry = mounts.vacant_entry();
            let mount = MountId(entry.key());
            self.mount.set(mount);
            tracing::trace!(?self, ?mount, "creating template");
            entry.insert(VNodeMount {
                node: self.clone(),
                parent,
                root_ids: vec![ElementId(0); template.roots().len()].into_boxed_slice(),
                mounted_attributes: vec![ElementId(0); template.attr_paths().len()]
                    .into_boxed_slice(),
                mounted_dynamic_nodes: vec![usize::MAX; template.node_paths().len()]
                    .into_boxed_slice(),
            });
        }

        // Walk the roots, creating nodes and assigning IDs
        // nodes in an iterator of (dynamic_node_index, path) and attrs in an iterator of (attr_index, path)
        let mut nodes = template.node_paths().iter().copied().enumerate().peekable();
        let mut attrs = template.attr_paths().iter().copied().enumerate().peekable();

        // Get the mounted id of this block
        // At this point, we should have already mounted the block
        debug_assert!(
            dom.runtime.mounts.borrow().contains(
                self.mount
                    .get()
                    .as_usize()
                    .expect("node should already be mounted"),
            )
        );
        let mount = self.mount.get();

        // Go through each root node and create the node, adding it to the stack.
        // Each node already exists in the template, so we can just clone it from the template

        // And return the number of nodes we created on the stack
        template
            .roots()
            .iter()
            .enumerate()
            .map(|(root_idx, root)| {
                match root {
                    TemplateNode::Dynamic { id } => {
                        // Take a dynamic node off the depth first iterator
                        nodes.next().unwrap();
                        // Then mount the node
                        self.create_dynamic_node(
                            &self.dynamic_nodes[*id],
                            mount,
                            *id,
                            dom,
                            to.as_deref_mut(),
                        )
                    }
                    // For static text and element nodes, just load the template root. This may be a placeholder or just a static node. We now know that each root node has a unique id
                    TemplateNode::Text { .. } | TemplateNode::Element { .. } => {
                        if let Some(to) = to.as_deref_mut() {
                            self.load_template_root(mount, root_idx, dom, to);
                        }

                        // If this is an element, load in all of the placeholder or dynamic content under this root element too
                        if matches!(root, TemplateNode::Element { .. }) {
                            // !!VERY IMPORTANT!!
                            // Write out all attributes before we load the children. Loading the children will change paths we rely on
                            // to assign ids to elements with dynamic attributes
                            if let Some(to) = to.as_deref_mut() {
                                self.write_attrs(mount, &mut attrs, root_idx as u8, dom, to);
                            }
                            // This operation relies on the fact that the root node is the top node on the stack so we need to do it here
                            self.load_placeholders(
                                mount,
                                &mut nodes,
                                root_idx as u8,
                                dom,
                                to.as_deref_mut(),
                            );
                        }

                        // This creates one node on the stack
                        1
                    }
                }
            })
            .sum()
    }
}

impl VNode {
    /// Get a reference back into a dynamic node
    fn reference_to_dynamic_node(&self, mount: MountId, dynamic_node_id: usize) -> ElementRef {
        ElementRef {
            path: ElementPath {
                path: self.template.node_paths()[dynamic_node_id],
            },
            mount,
        }
    }

    pub(crate) fn create_dynamic_node(
        &self,
        node: &DynamicNode,
        mount: MountId,
        dynamic_node_id: usize,
        dom: &mut VirtualDom,
        to: Option<&mut impl WriteMutations>,
    ) -> usize {
        use DynamicNode::*;
        match node {
            Component(component) => {
                let parent = Some(self.reference_to_dynamic_node(mount, dynamic_node_id));
                self.create_component_node(mount, dynamic_node_id, component, parent, dom, to)
            }
            Fragment(frag) => {
                let parent = Some(self.reference_to_dynamic_node(mount, dynamic_node_id));
                dom.create_children(to, frag, parent)
            }
            Text(text) => {
                // If we are diffing suspended nodes and are not outputting mutations, we can skip it
                if let Some(to) = to {
                    self.create_dynamic_text(mount, dynamic_node_id, text, dom, to)
                } else {
                    0
                }
            }
            Placeholder(_) => {
                // If we are diffing suspended nodes and are not outputting mutations, we can skip it
                if let Some(to) = to {
                    tracing::trace!("creating placeholder");
                    self.create_placeholder(mount, dynamic_node_id, dom, to)
                } else {
                    tracing::trace!("skipping creating placeholder");
                    0
                }
            }
        }
    }

    /// Load all of the placeholder nodes for descendent of this root node
    ///
    /// ```rust, no_run
    /// # use dioxus::prelude::*;
    /// # let some_text = "hello world";
    /// # let some_value = "123";
    /// rsx! {
    ///     div { // We just wrote this node
    ///         // This is a placeholder
    ///         {some_value}
    ///
    ///         // Load this too
    ///         "{some_text}"
    ///     }
    /// };
    /// ```
    ///
    /// IMPORTANT: This function assumes that root node is the top node on the stack
    fn load_placeholders(
        &self,
        mount: MountId,
        dynamic_nodes_iter: &mut Peekable<impl Iterator<Item = (usize, &'static [u8])>>,
        root_idx: u8,
        dom: &mut VirtualDom,
        mut to: Option<&mut impl WriteMutations>,
    ) {
        fn collect_dyn_node_range(
            dynamic_nodes: &mut Peekable<impl Iterator<Item = (usize, &'static [u8])>>,
            root_idx: u8,
        ) -> Option<(usize, usize)> {
            let start = match dynamic_nodes.peek() {
                Some((idx, [first, ..])) if *first == root_idx => *idx,
                _ => return None,
            };

            let mut end = start;

            while let Some((idx, p)) =
                dynamic_nodes.next_if(|(_, p)| matches!(p, [idx, ..] if *idx == root_idx))
            {
                debug_assert!(p.len() > 1);
                end = idx;
            }

            Some((start, end))
        }

        let (start, end) = match collect_dyn_node_range(dynamic_nodes_iter, root_idx) {
            Some((a, b)) => (a, b),
            None => return,
        };

        // !!VERY IMPORTANT!!
        //
        // We need to walk the dynamic nodes in reverse order because we are going to replace the
        // placeholder with the new nodes, which will invalidate our paths into the template.
        // If we go in reverse, we leave a "wake of destruction" in our path, but our next iteration
        // will still be "clean" since we only invalidated downstream nodes.
        //
        // Forgetting to do this will cause weird bugs like:
        //  https://github.com/DioxusLabs/dioxus/issues/2809
        //
        // Which are quite serious.
        // There might be more places in this codebase where we need to do `.rev()`
        let reversed_iter = (start..=end).rev();

        for dynamic_node_id in reversed_iter {
            let m = self.create_dynamic_node(
                &self.dynamic_nodes[dynamic_node_id],
                mount,
                dynamic_node_id,
                dom,
                to.as_deref_mut(),
            );
            if let Some(to) = to.as_deref_mut() {
                // If we actually created real new nodes, we need to replace the placeholder for this dynamic node with the new dynamic nodes
                debug_assert!(m > 0);
                // The path is one shorter because the top node is the root
                let path = &self.template.node_paths()[dynamic_node_id][1..];
                to.replace_placeholder_with_nodes(path, m);
            }
        }
    }

    /// After we have written a root element, we need to write all the attributes that are on the root node
    ///
    /// ```rust, ignore
    /// rsx! {
    ///     div { // We just wrote this node
    ///         class: "{class}", // We need to set these attributes
    ///         id: "{id}",
    ///         style: "{style}",
    ///     }
    /// }
    /// ```
    ///
    /// IMPORTANT: This function assumes that root node is the top node on the stack
    fn write_attrs(
        &self,
        mount: MountId,
        dynamic_attributes_iter: &mut Peekable<impl Iterator<Item = (usize, &'static [u8])>>,
        root_idx: u8,
        dom: &mut VirtualDom,
        to: &mut impl WriteMutations,
    ) {
        let mut last_path = None;
        // Only take nodes that are under this root node
        let from_root_node = |(_, path): &(usize, &[u8])| path.first() == Some(&root_idx);
        while let Some((attribute_idx, attribute_path)) =
            dynamic_attributes_iter.next_if(from_root_node)
        {
            let attribute = &self.dynamic_attrs[attribute_idx];

            let id = match last_path {
                // If the last path was exactly the same, we can reuse the id
                Some((path, id)) if path == attribute_path => id,
                // Otherwise, we need to create a new id
                _ => {
                    let id = self.assign_static_node_as_dynamic(mount, attribute_path, dom, to);
                    last_path = Some((attribute_path, id));
                    id
                }
            };

            // Write the value for each attribute in the group
            for attr in &**attribute {
                self.write_attribute(attribute_path, attr, id, mount, dom, to);
            }
            // Set the mounted dynamic attribute once. This must be set even if no actual
            // attributes are present so it is present for renderers like fullstack to look
            // up the position where attributes may be inserted in the future
            dom.set_mounted_dyn_attr(mount, attribute_idx, id);
        }
    }

    fn load_template_root(
        &self,
        mount: MountId,
        root_idx: usize,
        dom: &mut VirtualDom,
        to: &mut impl WriteMutations,
    ) -> ElementId {
        // Get an ID for this root since it's a real root
        let this_id = dom.next_element();
        dom.set_mounted_root_node(mount, root_idx, this_id);

        to.load_template(self.template, root_idx, this_id);

        this_id
    }

    /// We have some dynamic attributes attached to a some node
    ///
    /// That node needs to be loaded at runtime, so we need to give it an ID
    ///
    /// If the node in question is the root node, we just return the ID
    ///
    /// If the node is not on the stack, we create a new ID for it and assign it
    fn assign_static_node_as_dynamic(
        &self,
        mount: MountId,
        path: &'static [u8],
        dom: &mut VirtualDom,
        to: &mut impl WriteMutations,
    ) -> ElementId {
        // This is just the root node. We already know it's id
        if let [root_idx] = path {
            return dom.get_mounted_root_node(mount, *root_idx as usize);
        }

        // The node is deeper in the template and we should create a new id for it
        let id = dom.next_element();

        to.assign_node_id(&path[1..], id);

        id
    }

    fn create_dynamic_text(
        &self,
        mount: MountId,
        idx: usize,
        text: &VText,
        dom: &mut VirtualDom,
        to: &mut impl WriteMutations,
    ) -> usize {
        let new_id = mount.mount_node(idx, dom);

        // If this is a root node, the path is empty and we need to create a new text node
        to.create_text_node(&text.value, new_id);
        // We create one node on the stack
        1
    }

    pub(crate) fn create_placeholder(
        &self,
        mount: MountId,
        idx: usize,
        dom: &mut VirtualDom,
        to: &mut impl WriteMutations,
    ) -> usize {
        let new_id = mount.mount_node(idx, dom);

        // If this is a root node, the path is empty and we need to create a new placeholder node
        to.create_placeholder(new_id);
        // We create one node on the stack
        1
    }
}

impl MountId {
    fn mount_node(self, node_index: usize, dom: &mut VirtualDom) -> ElementId {
        let id = dom.next_element();
        dom.set_mounted_dyn_node(self, node_index, id.0);
        id
    }
}
