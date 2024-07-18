use crate::innerlude::MountId;
use crate::{Attribute, AttributeValue, DynamicNode::*, Template};
use crate::{VNode, VirtualDom, WriteMutations};
use core::iter::Peekable;

use crate::{
    arena::ElementId,
    innerlude::{ElementPath, ElementRef, VComponent, VNodeMount, VText},
    nodes::DynamicNode,
    scopes::ScopeId,
    TemplateNode,
};

impl VNode {
    pub(crate) fn diff_node(
        &self,
        new: &VNode,
        dom: &mut VirtualDom,
        mut to: Option<&mut impl WriteMutations>,
    ) {
        // The node we are diffing from should always be mounted
        debug_assert!(dom.mounts.get(self.mount.get().0).is_some() || to.is_none());

        // If hot reloading is enabled, we need to make sure we're using the latest template
        #[cfg(debug_assertions)]
        {
            let name = new.template.get().name;
            if let Some(template) = dom.templates.get(name).cloned() {
                new.template.set(template);
                if template != self.template.get() {
                    let mount_id = self.mount.get();
                    let parent = dom.mounts[mount_id.0].parent;
                    self.replace(std::slice::from_ref(new), parent, dom, to);
                    return;
                }
            }
        }

        // If the templates are different by name, we need to replace the entire template
        if self.templates_are_different(new) {
            return self.light_diff_templates(new, dom, to);
        }

        let mount_id = self.mount.get();

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
        let mount_id = self.mount.get();
        new.mount.set(mount_id);

        if mount_id.mounted() {
            let mount = &mut dom.mounts[mount_id.0];

            // Update the reference to the node for bubbling events
            mount.node = new.clone_mounted();
        }
    }

    fn diff_dynamic_node(
        &self,
        mount: MountId,
        idx: usize,
        old_node: &DynamicNode,
        new_node: &DynamicNode,
        dom: &mut VirtualDom,
        to: Option<&mut impl WriteMutations>,
    ) {
        tracing::trace!("diffing dynamic node from {old_node:?} to {new_node:?}");
        match (old_node, new_node) {
            (Text(old), Text(new)) => {
                // Diffing text is just a side effect, if we are diffing suspended nodes and are not outputting mutations, we can skip it
                if let Some(to) = to{
                    let mount = &dom.mounts[mount.0];
                    self.diff_vtext(to, mount, idx, old, new)
                }
            },
            (Placeholder(_), Placeholder(_)) => {},
            (Fragment(old), Fragment(new)) => dom.diff_non_empty_fragment(to, old, new, Some(self.reference_to_dynamic_node(mount, idx))),
            (Component(old), Component(new)) => {
				let scope_id = ScopeId(dom.mounts[mount.0].mounted_dynamic_nodes[idx]);
                self.diff_vcomponent(mount, idx, new, old, scope_id, Some(self.reference_to_dynamic_node(mount, idx)), dom, to)
            },
            (Placeholder(_), Fragment(right)) => {
                let placeholder_id = ElementId(dom.mounts[mount.0].mounted_dynamic_nodes[idx]);
                dom.replace_placeholder(to, placeholder_id, right, Some(self.reference_to_dynamic_node(mount, idx)))},
            (Fragment(left), Placeholder(_)) => {
                dom.nodes_to_placeholder(to, mount, idx, left,)
            },
            _ => todo!("This is an usual custom case for dynamic nodes. We don't know how to handle it yet."),
        };
    }

    /// Try to get the dynamic node and its index for a root node
    pub(crate) fn get_dynamic_root_node_and_id(
        &self,
        root_idx: usize,
    ) -> Option<(usize, &DynamicNode)> {
        self.template.get().roots[root_idx]
            .dynamic_id()
            .map(|id| (id, &self.dynamic_nodes[id]))
    }

    pub(crate) fn find_first_element(&self, dom: &VirtualDom) -> ElementId {
        let mount = &dom.mounts[self.mount.get().0];
        match self.get_dynamic_root_node_and_id(0) {
            // This node is static, just get the root id
            None | Some((_, Placeholder(_) | Text(_))) => mount.root_ids[0],
            // The node is a fragment, so we need to find the first element in the fragment
            Some((_, Fragment(children))) => {
                let child = children.first().unwrap();
                child.find_first_element(dom)
            }
            // The node is a component, so we need to find the first element in the component
            Some((id, Component(_))) => {
                let scope = ScopeId(mount.mounted_dynamic_nodes[id]);
                dom.get_scope(scope)
                    .unwrap()
                    .root_node()
                    .find_first_element(dom)
            }
        }
    }

    pub(crate) fn find_last_element(&self, dom: &VirtualDom) -> ElementId {
        let mount = &dom.mounts[self.mount.get().0];
        let last_root_index = self.template.get().roots.len() - 1;
        match self.get_dynamic_root_node_and_id(last_root_index) {
            // This node is static, just get the root id
            None | Some((_, Placeholder(_) | Text(_))) => mount.root_ids[last_root_index],
            // The node is a fragment, so we need to find the first element in the fragment
            Some((_, Fragment(children))) => {
                let child = children.first().unwrap();
                child.find_first_element(dom)
            }
            // The node is a component, so we need to find the first element in the component
            Some((id, Component(_))) => {
                let scope = ScopeId(mount.mounted_dynamic_nodes[id]);
                dom.get_scope(scope)
                    .unwrap()
                    .root_node()
                    .find_last_element(dom)
            }
        }
    }

    /// Diff the two text nodes
    ///
    /// This just sets the text of the node if it's different.
    fn diff_vtext(
        &self,
        to: &mut impl WriteMutations,
        mount: &VNodeMount,
        idx: usize,
        left: &VText,
        right: &VText,
    ) {
        if left.value != right.value {
            let id = ElementId(mount.mounted_dynamic_nodes[idx]);
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
        let mount = self.mount.get();
        if !mount.mounted() {
            return;
        }

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
            // Remove the mount information
            dom.mounts.remove(mount.0);
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
        let roots = self.template.get().roots;
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
            } else if let Some(to) = to.as_deref_mut() {
                let mount = &dom.mounts[mount.0];
                let id = mount.root_ids[idx];
                if let (true, Some(replace_with)) = (last_node, replace_with) {
                    to.replace_node_with(id, replace_with);
                } else {
                    to.remove_node(id);
                }
                dom.reclaim(id);
            }
        }
    }

    fn remove_nested_dyn_nodes<M: WriteMutations>(
        &self,
        mount: MountId,
        dom: &mut VirtualDom,
        destroy_component_state: bool,
    ) {
        let template = self.template.get();
        for (idx, dyn_node) in self.dynamic_nodes.iter().enumerate() {
            let path_len = template.node_paths.get(idx).map(|path| path.len());
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
                let scope_id = ScopeId(dom.mounts[mount.0].mounted_dynamic_nodes[idx]);
                dom.remove_component_node(to, destroy_component_state, scope_id, replace_with);
            }
            Text(_) | Placeholder(_) => {
                let id = ElementId(dom.mounts[mount.0].mounted_dynamic_nodes[idx]);
                if let Some(to) = to {
                    if let Some(replace_with) = replace_with {
                        to.replace_node_with(id, replace_with);
                    } else {
                        to.remove_node(id);
                    }
                    dom.reclaim(id)
                }
            }
            Fragment(nodes) => {
                for node in &nodes[..nodes.len() - 1] {
                    node.remove_node_inner(dom, to.as_deref_mut(), destroy_component_state, None)
                }
                if let Some(last_node) = nodes.last() {
                    last_node.remove_node_inner(dom, to, destroy_component_state, replace_with)
                }
            }
        };
    }

    fn templates_are_different(&self, other: &VNode) -> bool {
        let self_node_name = self.template.get().id();
        let other_node_name = other.template.get().id();
        self_node_name != other_node_name
    }

    pub(super) fn reclaim_attributes(&self, mount: MountId, dom: &mut VirtualDom) {
        let mut next_id = None;
        for (idx, path) in self.template.get().attr_paths.iter().enumerate() {
            // We clean up the roots in the next step, so don't worry about them here
            if path.len() <= 1 {
                continue;
            }

            // only reclaim the new element if it's different from the previous one
            let new_id = dom.mounts[mount.0].mounted_attributes[idx];
            if Some(new_id) != next_id {
                dom.reclaim(new_id);
                next_id = Some(new_id);
            }
        }
    }

    pub(super) fn diff_attributes(
        &self,
        new: &VNode,
        dom: &mut VirtualDom,
        to: &mut impl WriteMutations,
    ) {
        let mount_id = self.mount.get();
        for (idx, (old_attrs, new_attrs)) in self
            .dynamic_attrs
            .iter()
            .zip(new.dynamic_attrs.iter())
            .enumerate()
        {
            let mut old_attributes_iter = old_attrs.iter().peekable();
            let mut new_attributes_iter = new_attrs.iter().peekable();
            let attribute_id = dom.mounts[mount_id.0].mounted_attributes[idx];
            let path = self.template.get().attr_paths[idx];

            loop {
                match (old_attributes_iter.peek(), new_attributes_iter.peek()) {
                    (Some(old_attribute), Some(new_attribute)) => {
                        // check which name is greater
                        match old_attribute.name.cmp(new_attribute.name) {
                            // The two attributes are the same, so diff them
                            std::cmp::Ordering::Equal => {
                                let old = old_attributes_iter.next().unwrap();
                                let new = new_attributes_iter.next().unwrap();
                                // Volatile attributes are attributes that the browser may override so we always update them
                                let volatile = old.volatile;
                                if volatile || old.value != new.value {
                                    self.write_attribute(
                                        path,
                                        new,
                                        attribute_id,
                                        mount_id,
                                        dom,
                                        to,
                                    );
                                }
                            }
                            // In a sorted list, if the old attribute name is first, then the new attribute is missing
                            std::cmp::Ordering::Less => {
                                let old = old_attributes_iter.next().unwrap();
                                self.remove_attribute(old, attribute_id, to)
                            }
                            // In a sorted list, if the new attribute name is first, then the old attribute is missing
                            std::cmp::Ordering::Greater => {
                                let new = new_attributes_iter.next().unwrap();
                                self.write_attribute(path, new, attribute_id, mount_id, dom, to);
                            }
                        }
                    }
                    (Some(_), None) => {
                        let left = old_attributes_iter.next().unwrap();
                        self.remove_attribute(left, attribute_id, to)
                    }
                    (None, Some(_)) => {
                        let right = new_attributes_iter.next().unwrap();
                        self.write_attribute(path, right, attribute_id, mount_id, dom, to)
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
                dom.elements[id.0] = Some(element_ref);
                to.create_event_listener(&attribute.name[2..], id);
            }
            _ => {
                to.set_attribute(attribute.name, attribute.namespace, &attribute.value, id);
            }
        }
    }

    /// Lightly diff the two templates, checking only their roots.
    ///
    /// The goal here is to preserve any existing component state that might exist. This is to preserve some React-like
    /// behavior where the component state is preserved when the component is re-rendered.
    ///
    /// This is implemented by iterating each root, checking if the component is the same, if it is, then diff it.
    ///
    /// We then pass the new template through "create" which should be smart enough to skip roots.
    ///
    /// Currently, we only handle the case where the roots are the same component list. If there's any sort of deviation,
    /// IE more nodes, less nodes, different nodes, or expressions, then we just replace the whole thing.
    ///
    /// This is mostly implemented to help solve the issue where the same component is rendered under two different
    /// conditions:
    ///
    /// ```rust, no_run
    /// # use dioxus::prelude::*;
    /// # let enabled = true;
    /// # #[component]
    /// # fn Component(enabled_sign: String) -> Element { unimplemented!() }
    /// if enabled {
    ///     rsx!{ Component { enabled_sign: "abc" } }
    /// } else {
    ///     rsx!{ Component { enabled_sign: "xyz" } }
    /// };
    /// ```
    ///
    /// However, we should not that it's explicit in the docs that this is not a guarantee. If you need to preserve state,
    /// then you should be passing in separate props instead.
    ///
    /// ```rust, no_run
    /// # use dioxus::prelude::*;
    /// # #[component]
    /// # fn Component(enabled_sign: String) -> Element { unimplemented!() }
    /// # let enabled = true;
    /// let props = if enabled {
    ///     ComponentProps { enabled_sign: "abc".to_string() }
    /// } else {
    ///     ComponentProps { enabled_sign: "xyz".to_string() }
    /// };
    ///
    /// rsx! {
    ///     Component { ..props }
    /// };
    /// ```
    pub(crate) fn light_diff_templates(
        &self,
        new: &VNode,
        dom: &mut VirtualDom,
        mut to: Option<&mut impl WriteMutations>,
    ) {
        let mount_id = self.mount.get();
        let mount = &dom.mounts[mount_id.0];
        let parent = mount.parent;
        match matching_components(self, new) {
            None => self.replace(std::slice::from_ref(new), parent, dom, to),
            Some(components) => {
                self.move_mount_to(new, dom);

                for (idx, (old_component, new_component)) in components.into_iter().enumerate() {
                    let mount = &dom.mounts[mount_id.0];
                    let scope_id = ScopeId(mount.mounted_dynamic_nodes[idx]);
                    self.diff_vcomponent(
                        mount_id,
                        idx,
                        new_component,
                        old_component,
                        scope_id,
                        parent,
                        dom,
                        to.as_deref_mut(),
                    )
                }
            }
        }
    }

    /// Get the most up to date template for this rsx block
    #[allow(unused)]
    pub(crate) fn template(&self, dom: &VirtualDom) -> Template {
        // check for a overridden template
        #[cfg(debug_assertions)]
        {
            let template = self.template.get();
            if let Some(new_template) = dom.templates.get(template.name) {
                self.template.set(*new_template);
            }
        };

        self.template.get()
    }

    /// Create this rsx block. This will create scopes from components that this rsx block contains, but it will not write anything to the DOM.
    pub(crate) fn create(
        &self,
        dom: &mut VirtualDom,
        parent: Option<ElementRef>,
        mut to: Option<&mut impl WriteMutations>,
    ) -> usize {
        // Get the most up to date template
        let template = self.template(dom);

        // Initialize the mount information for this vnode if it isn't already mounted
        if !self.mount.get().mounted() {
            let entry = dom.mounts.vacant_entry();
            let mount = MountId(entry.key());
            self.mount.set(mount);
            tracing::trace!(?self, ?mount, "creating template");
            entry.insert(VNodeMount {
                node: self.clone_mounted(),
                parent,
                root_ids: vec![ElementId(0); template.roots.len()].into_boxed_slice(),
                mounted_attributes: vec![ElementId(0); template.attr_paths.len()]
                    .into_boxed_slice(),
                mounted_dynamic_nodes: vec![usize::MAX; template.node_paths.len()]
                    .into_boxed_slice(),
            });
        }

        // If we are outputting mutations, mount the node as well
        if let Some(to) = to.as_deref_mut() {
            // The best renderers will have templates pre-hydrated and registered
            // Just in case, let's create the template using instructions anyways
            dom.register_template(to, template);
        }

        // Walk the roots, creating nodes and assigning IDs
        // nodes in an iterator of (dynamic_node_index, path)

        let nodes_sorted = template.breadth_first_node_paths();
        let attrs_sorted = template.breadth_first_attribute_paths();

        let mut nodes = nodes_sorted.peekable();
        let mut attrs = attrs_sorted.peekable();

        // Get the mounted id of this block
        // At this point, we should have already mounted the block
        debug_assert!(
            dom.mounts.contains(
                self.mount
                    .get()
                    .as_usize()
                    .expect("node should already be mounted"),
            ),
            "Node mount should be valid"
        );
        let mount = self.mount.get();

        // Go through each root node and create the node, adding it to the stack.
        // Each node already exists in the template, so we can just clone it from the template
        let nodes_created = template
            .roots
            .iter()
            .enumerate()
            .map(|(root_idx, root)| {
                match root {
                    TemplateNode::Dynamic { id } => {
                        // Take a dynamic node off the depth first iterator
                        nodes.next().unwrap();
                        // Then mount the node
                        self.create_dynamic_node(mount, *id, dom, to.as_deref_mut())
                    }
                    // For static text and element nodes, just load the template root. This may be a placeholder or just a static node. We now know that each root node has a unique id
                    TemplateNode::Text { .. } | TemplateNode::Element { .. } => {
                        if let Some(to) = to.as_deref_mut() {
                            self.load_template_root(mount, root_idx, dom, to);
                        }

                        // If this is an element, load in all of the placeholder or dynamic content under this root element too
                        if matches!(root, TemplateNode::Element { .. }) {
                            // This operation relies on the fact that the root node is the top node on the stack so we need to do it here
                            self.load_placeholders(
                                mount,
                                &mut nodes,
                                root_idx as u8,
                                dom,
                                to.as_deref_mut(),
                            );
                            // Now write out any attributes we need
                            if let Some(to) = to.as_deref_mut() {
                                self.write_attrs(mount, &mut attrs, root_idx as u8, dom, to);
                            }
                        }

                        // This creates one node on the stack
                        1
                    }
                }
            })
            .sum();

        // And return the number of nodes we created on the stack
        nodes_created
    }
}

impl VNode {
    /// Get a reference back into a dynamic node
    fn reference_to_dynamic_node(&self, mount: MountId, dynamic_node_id: usize) -> ElementRef {
        ElementRef {
            path: ElementPath {
                path: self.template.get().node_paths[dynamic_node_id],
            },
            mount,
        }
    }

    pub(crate) fn create_dynamic_node(
        &self,
        mount: MountId,
        dynamic_node_id: usize,
        dom: &mut VirtualDom,
        to: Option<&mut impl WriteMutations>,
    ) -> usize {
        use DynamicNode::*;
        let node = &self.dynamic_nodes[dynamic_node_id];
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
        // Only take nodes that are under this root node
        let from_root_node = |(_, path): &(usize, &[u8])| path.first() == Some(&root_idx);
        while let Some((dynamic_node_id, path)) = dynamic_nodes_iter.next_if(from_root_node) {
            let m = self.create_dynamic_node(mount, dynamic_node_id, dom, to.as_deref_mut());
            if let Some(to) = to.as_deref_mut() {
                // If we actually created real new nodes, we need to replace the placeholder for this dynamic node with the new dynamic nodes
                if m > 0 {
                    // The path is one shorter because the top node is the root
                    let path = &path[1..];
                    to.replace_placeholder_with_nodes(path, m);
                }
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
        dynamic_attrbiutes_iter: &mut Peekable<impl Iterator<Item = (usize, &'static [u8])>>,
        root_idx: u8,
        dom: &mut VirtualDom,
        to: &mut impl WriteMutations,
    ) {
        let mut last_path = None;
        // Only take nodes that are under this root node
        let from_root_node = |(_, path): &(usize, &[u8])| path.first() == Some(&root_idx);
        while let Some((attribute_idx, attribute_path)) =
            dynamic_attrbiutes_iter.next_if(from_root_node)
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

            for attr in &**attribute {
                self.write_attribute(attribute_path, attr, id, mount, dom, to);
                dom.mounts[mount.0].mounted_attributes[attribute_idx] = id;
            }
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
        dom.mounts[mount.0].root_ids[root_idx] = this_id;

        to.load_template(self.template.get().name, root_idx, this_id);

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
            return dom.mounts[mount.0].root_ids[*root_idx as usize];
        }

        // The node is deeper in the template and we should create a new id for it
        let id = dom.next_element();

        to.assign_node_id(&path[1..], id);

        id
    }

    /// Mount a root node and return its ID and the path to the node
    fn create_dynamic_node_with_path(
        &self,
        mount: MountId,
        idx: usize,
        dom: &mut VirtualDom,
    ) -> (ElementId, &'static [u8]) {
        // Add the mutation to the list
        let path = self.template.get().node_paths[idx];

        // Allocate a dynamic element reference for this text node
        let new_id = mount.mount_node(idx, dom);

        (new_id, &path[1..])
    }

    fn create_dynamic_text(
        &self,
        mount: MountId,
        idx: usize,
        text: &VText,
        dom: &mut VirtualDom,
        to: &mut impl WriteMutations,
    ) -> usize {
        let (new_id, path) = self.create_dynamic_node_with_path(mount, idx, dom);

        // If this is a root node, the path is empty and we need to create a new text node
        if path.is_empty() {
            to.create_text_node(&text.value, new_id);
            // We create one node on the stack
            1
        } else {
            // Dynamic text nodes always exist as a placeholder text node in the template, we can just hydrate that text node instead of creating a new one
            to.hydrate_text_node(path, &text.value, new_id);
            // Since we're hydrating an existing node, we don't create any new nodes
            0
        }
    }

    pub(crate) fn create_placeholder(
        &self,
        mount: MountId,
        idx: usize,
        dom: &mut VirtualDom,
        to: &mut impl WriteMutations,
    ) -> usize {
        let (id, path) = self.create_dynamic_node_with_path(mount, idx, dom);

        // If this is a root node, the path is empty and we need to create a new text node
        if path.is_empty() {
            to.create_placeholder(id);
            // We create one node on the stack
            1
        } else {
            // Assign the ID to the existing node in the template
            to.assign_node_id(path, id);
            // Since the placeholder is already in the DOM, we don't create any new nodes
            0
        }
    }
}

impl MountId {
    fn mount_node(self, node_index: usize, dom: &mut VirtualDom) -> ElementId {
        let id = dom.next_element();
        dom.mounts[self.0].mounted_dynamic_nodes[node_index] = id.0;
        id
    }
}

fn matching_components<'a>(
    left: &'a VNode,
    right: &'a VNode,
) -> Option<Vec<(&'a VComponent, &'a VComponent)>> {
    let left_node = left.template.get();
    let right_node = right.template.get();
    if left_node.roots.len() != right_node.roots.len() {
        return None;
    }

    // run through the components, ensuring they're the same
    left_node
        .roots
        .iter()
        .zip(right_node.roots.iter())
        .map(|(l, r)| {
            let (l, r) = match (l, r) {
                (TemplateNode::Dynamic { id: l }, TemplateNode::Dynamic { id: r }) => (l, r),
                _ => return None,
            };

            let (l, r) = match (&left.dynamic_nodes[*l], &right.dynamic_nodes[*r]) {
                (Component(l), Component(r)) => (l, r),
                _ => return None,
            };

            Some((l, r))
        })
        .collect()
}
