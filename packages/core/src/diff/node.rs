use crate::innerlude::MountId;
use crate::{Attribute, AttributeValue, DynamicNode::*};
use crate::{VNode, VirtualDom, WriteMutations};
use core::iter::Peekable;

use crate::{
    arena::ElementId,
    innerlude::{ElementPath, ElementRef, VComponent, VNodeMount, VText},
    nodes::DynamicNode,
    nodes::RenderReturn,
    scopes::ScopeId,
    TemplateNode,
    TemplateNode::*,
};

impl VNode {
    pub(crate) fn diff_node(
        &self,
        new: &VNode,
        dom: &mut VirtualDom,
        to: &mut impl WriteMutations,
    ) {
        // The node we are diffing from should always be mounted
        debug_assert!(dom.mounts.get(self.mount.get().0).is_some());

        // If hot reloading is enabled, we need to make sure we're using the latest template
        #[cfg(debug_assertions)]
        {
            let (path, byte_index) = new.template.get().name.rsplit_once(':').unwrap();
            if let Some(map) = dom.templates.get(path) {
                let byte_index = byte_index.parse::<usize>().unwrap();
                if let Some(&template) = map.get(&byte_index) {
                    new.template.set(template);
                    if template != self.template.get() {
                        let mount_id = self.mount.get();
                        let parent = dom.mounts[mount_id.0].parent;
                        return self.replace([new], parent, dom, to);
                    }
                }
            }
        }

        // Copy over the mount information
        let mount_id = self.mount.get();
        new.mount.set(mount_id);

        let mount = &mut dom.mounts[mount_id.0];

        // Update the reference to the node for bubbling events
        mount.node = new.clone_mounted();

        // If the templates are the same, we don't need to do anything, except copy over the mount information
        if self == new {
            return;
        }

        // If the templates are different by name, we need to replace the entire template
        if self.templates_are_different(new) {
            return self.light_diff_templates(new, dom, to);
        }

        // If the templates are the same, we can diff the attributes and children
        // Start with the attributes
        self.dynamic_attrs
            .iter()
            .zip(new.dynamic_attrs.iter())
            .enumerate()
            .for_each(|(idx, (old_attr, new_attr))| {
                // If the attributes are different (or volatile), we need to update them
                if old_attr.value != new_attr.value || new_attr.volatile {
                    self.update_attribute(mount, idx, new_attr, to);
                }
            });

        // Now diff the dynamic nodes
        self.dynamic_nodes
            .iter()
            .zip(new.dynamic_nodes.iter())
            .enumerate()
            .for_each(|(dyn_node_idx, (old, new))| {
                self.diff_dynamic_node(mount_id, dyn_node_idx, old, new, dom, to)
            });
    }

    fn diff_dynamic_node(
        &self,
        mount: MountId,
        idx: usize,
        old_node: &DynamicNode,
        new_node: &DynamicNode,
        dom: &mut VirtualDom,
        to: &mut impl WriteMutations,
    ) {
        let parent = || ElementRef {
            mount,
            path: ElementPath {
                path: self.template.get().node_paths[idx],
            },
        };
        match (old_node, new_node) {
            (Text(old), Text(new)) => {
                let mount = &dom.mounts[mount.0];
                self.diff_vtext( to, mount, idx, old, new)
            },
            (Placeholder(_), Placeholder(_)) => {},
            (Fragment(old), Fragment(new)) => dom.diff_non_empty_fragment(to, old, new, Some(parent())),
            (Component(old), Component(new)) => {
				let scope_id = ScopeId(dom.mounts[mount.0].mounted_dynamic_nodes[idx]);
                self.diff_vcomponent(mount, idx, new, old, scope_id, Some(parent()), dom, to)
            },
            (Placeholder(_), Fragment(right)) => {
                let placeholder_id = ElementId(dom.mounts[mount.0].mounted_dynamic_nodes[idx]);
                dom.replace_placeholder(to, placeholder_id, right, Some(parent()))},
            (Fragment(left), Placeholder(_)) => {
                dom.nodes_to_placeholder(to, mount, idx, left,)
            },
            _ => todo!("This is an usual custom case for dynamic nodes. We don't know how to handle it yet."),
        };
    }

    pub(crate) fn find_first_element(&self, dom: &VirtualDom) -> ElementId {
        let mount = &dom.mounts[self.mount.get().0];
        match &self.template.get().roots[0] {
            TemplateNode::Element { .. } | TemplateNode::Text { text: _ } => mount.root_ids[0],
            TemplateNode::Dynamic { id } | TemplateNode::DynamicText { id } => {
                match &self.dynamic_nodes[*id] {
                    Placeholder(_) | Text(_) => ElementId(mount.mounted_dynamic_nodes[*id]),
                    Fragment(children) => {
                        let child = children.first().unwrap();
                        child.find_first_element(dom)
                    }
                    Component(_comp) => {
                        let scope = ScopeId(mount.mounted_dynamic_nodes[*id]);
                        dom.get_scope(scope)
                            .unwrap()
                            .root_node()
                            .find_first_element(dom)
                    }
                }
            }
        }
    }

    pub(crate) fn find_last_element(&self, dom: &VirtualDom) -> ElementId {
        let mount = &dom.mounts[self.mount.get().0];
        match &self.template.get().roots.last().unwrap() {
            TemplateNode::Element { .. } | TemplateNode::Text { text: _ } => {
                *mount.root_ids.last().unwrap()
            }
            TemplateNode::Dynamic { id } | TemplateNode::DynamicText { id } => {
                match &self.dynamic_nodes[*id] {
                    Placeholder(_) | Text(_) => ElementId(mount.mounted_dynamic_nodes[*id]),
                    Fragment(t) => t.last().unwrap().find_last_element(dom),
                    Component(_comp) => {
                        let scope = ScopeId(mount.mounted_dynamic_nodes[*id]);
                        dom.get_scope(scope)
                            .unwrap()
                            .root_node()
                            .find_last_element(dom)
                    }
                }
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

    pub(crate) fn replace<'a>(
        &self,
        right: impl IntoIterator<Item = &'a VNode>,
        parent: Option<ElementRef>,
        dom: &mut VirtualDom,
        to: &mut impl WriteMutations,
    ) {
        let m = dom.create_children(to, right, parent);

        // TODO: Instead of *just* removing it, we can use the replace mutation
        let first_element = self.find_first_element(dom);
        to.insert_nodes_before(first_element, m);

        self.remove_node(dom, to, true)
    }

    pub(crate) fn remove_node(
        &self,
        dom: &mut VirtualDom,
        to: &mut impl WriteMutations,
        gen_muts: bool,
    ) {
        let mount = self.mount.get();

        // Clean up any attributes that have claimed a static node as dynamic for mount/unmounts
        // Will not generate mutations!
        self.reclaim_attributes(mount, dom, to);

        // Remove the nested dynamic nodes
        // We don't generate mutations for these, as they will be removed by the parent (in the next line)
        // But we still need to make sure to reclaim them from the arena and drop their hooks, etc
        self.remove_nested_dyn_nodes(mount, dom, to);

        // Clean up the roots, assuming we need to generate mutations for these
        // This is done last in order to preserve Node ID reclaim order (reclaim in reverse order of claim)
        self.reclaim_roots(mount, dom, to, gen_muts);

        // Remove the mount information
        dom.mounts.remove(mount.0);

        tracing::trace!(?self, "removed node");
    }

    fn reclaim_roots(
        &self,
        mount: MountId,
        dom: &mut VirtualDom,
        to: &mut impl WriteMutations,
        gen_muts: bool,
    ) {
        for (idx, node) in self.template.get().roots.iter().enumerate() {
            if let Some(id) = node.dynamic_id() {
                let dynamic_node = &self.dynamic_nodes[id];
                self.remove_dynamic_node(mount, dom, to, idx, dynamic_node, gen_muts);
            } else {
                let mount = &dom.mounts[mount.0];
                let id = mount.root_ids[idx];
                if gen_muts {
                    to.remove_node(id);
                }
                dom.reclaim(id);
            }
        }
    }

    fn remove_nested_dyn_nodes(
        &self,
        mount: MountId,
        dom: &mut VirtualDom,
        to: &mut impl WriteMutations,
    ) {
        let template = self.template.get();
        for (idx, dyn_node) in self.dynamic_nodes.iter().enumerate() {
            let path_len = template.node_paths.get(idx).map(|path| path.len());
            // Roots are cleaned up automatically above and nodes with a empty path are placeholders
            if let Some(2..) = path_len {
                self.remove_dynamic_node(mount, dom, to, idx, dyn_node, false)
            }
        }
    }

    fn remove_dynamic_node(
        &self,
        mount: MountId,
        dom: &mut VirtualDom,
        to: &mut impl WriteMutations,
        idx: usize,
        node: &DynamicNode,
        gen_muts: bool,
    ) {
        match node {
            Component(_comp) => {
                let scope_id = ScopeId(dom.mounts[mount.0].mounted_dynamic_nodes[idx]);
                dom.remove_component_node(to, scope_id, gen_muts);
            }
            Text(_) | Placeholder(_) => {
                let id = ElementId(dom.mounts[mount.0].mounted_dynamic_nodes[idx]);
                dom.remove_element_id(to, id, gen_muts)
            }
            Fragment(nodes) => nodes
                .iter()
                .for_each(|_node| self.remove_node(dom, to, gen_muts)),
        };
    }

    fn templates_are_different(&self, other: &VNode) -> bool {
        let self_node_name = self.template.get().name;
        let other_node_name = other.template.get().name;
        // we want to re-create the node if the template name is different by pointer even if the value is the same so that we can detect when hot reloading changes the template
        !std::ptr::eq(self_node_name, other_node_name)
    }

    pub(super) fn reclaim_attributes(
        &self,
        mount: MountId,
        dom: &mut VirtualDom,
        _to: &mut impl WriteMutations,
    ) {
        let id = None;

        for (idx, path) in self.template.get().attr_paths.iter().enumerate() {
            let _attr = &self.dynamic_attrs[idx];

            // We clean up the roots in the next step, so don't worry about them here
            if path.len() <= 1 {
                continue;
            }

            let next_id = dom.mounts[mount.0].mounted_attributes[idx];

            // only reclaim the new element if it's different from the previous one
            if id != Some(next_id) {
                dom.reclaim(next_id);
            }
        }
    }

    pub(super) fn update_attribute(
        &self,
        mount: &VNodeMount,
        idx: usize,
        new_attr: &Attribute,
        to: &mut impl WriteMutations,
    ) {
        let name = &new_attr.name;
        let value = &new_attr.value;
        let id = mount.mounted_attributes[idx];
        to.set_attribute(name, new_attr.namespace, value, id);
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
    /// ```rust, ignore
    /// if enabled {
    ///     rsx!{ Component { enabled_sign: "abc" } }
    /// } else {
    ///     rsx!{ Component { enabled_sign: "xyz" } }
    /// }
    /// ```
    ///
    /// However, we should not that it's explicit in the docs that this is not a guarantee. If you need to preserve state,
    /// then you should be passing in separate props instead.
    ///
    /// ```rust, ignore
    /// let props = if enabled {
    ///     ComponentProps { enabled_sign: "abc" }
    /// } else {
    ///     ComponentProps { enabled_sign: "xyz" }
    /// };
    ///
    /// rsx! {
    ///     Component { ..props }
    /// }
    /// ```
    pub(crate) fn light_diff_templates(
        &self,
        new: &VNode,
        dom: &mut VirtualDom,
        to: &mut impl WriteMutations,
    ) {
        let mount_id = self.mount.get();
        let mount = &dom.mounts[mount_id.0];
        let parent = mount.parent;
        match matching_components(self, new) {
            None => self.replace([new], parent, dom, to),
            Some(components) => {
                for (idx, (old_component, new_component)) in components.into_iter().enumerate() {
                    let mount = &dom.mounts[mount_id.0];
                    let scope_id = ScopeId(mount.mounted_dynamic_nodes[idx]);
                    self.diff_vcomponent(
                        mount_id,
                        idx,
                        old_component,
                        new_component,
                        scope_id,
                        parent,
                        dom,
                        to,
                    )
                }
            }
        }
    }

    /// Create this template and write its mutations
    pub(crate) fn create(
        &self,
        dom: &mut VirtualDom,
        to: &mut impl WriteMutations,
        parent: Option<ElementRef>,
    ) -> usize {
        // check for a overridden template
        #[cfg(debug_assertions)]
        {
            let template = self.template.get();
            let (path, byte_index) = template.name.rsplit_once(':').unwrap();
            if let Some(new_template) = dom
                .templates
                .get(path)
                .and_then(|map| map.get(&byte_index.parse().unwrap()))
            {
                self.template.set(*new_template);
            }
        };

        let template = self.template.get();

        // The best renderers will have templates pre-hydrated and registered
        // Just in case, let's create the template using instructions anyways
        dom.register_template(to, template);

        // Initialize the mount information for this template
        let entry = dom.mounts.vacant_entry();
        let mount = MountId(entry.key());
        self.mount.set(mount);
        tracing::info!(?self, ?mount, "creating template");
        entry.insert(VNodeMount {
            node: self.clone_mounted(),
            parent,
            root_ids: vec![ElementId(0); template.roots.len()].into_boxed_slice(),
            mounted_attributes: vec![ElementId(0); template.attr_paths.len()].into_boxed_slice(),
            mounted_dynamic_nodes: vec![0; template.node_paths.len()].into_boxed_slice(),
        });

        // Walk the roots, creating nodes and assigning IDs
        // nodes in an iterator of ((dynamic_node_index, sorted_index), path)
        // todo: adjust dynamic nodes to be in the order of roots and then leaves (ie BFS)
        #[cfg(not(debug_assertions))]
        let (mut attrs, mut nodes) = (
            template.attr_paths.iter().copied().enumerate().peekable(),
            template
                .node_paths
                .iter()
                .copied()
                .enumerate()
                .map(|(i, path)| ((i, i), path))
                .peekable(),
        );
        // If this is a debug build, we need to check that the paths are in the correct order because hot reloading can cause scrambled states

        #[cfg(debug_assertions)]
        let (attrs_sorted, nodes_sorted) =
            { (sort_bfs(template.attr_paths), sort_bfs(template.node_paths)) };
        #[cfg(debug_assertions)]
        let (mut attrs, mut nodes) = {
            (
                attrs_sorted.into_iter().peekable(),
                nodes_sorted
                    .iter()
                    .copied()
                    .enumerate()
                    .map(|(i, (id, path))| ((id, i), path))
                    .peekable(),
            )
        };

        template
            .roots
            .iter()
            .enumerate()
            .map(|(idx, root)| match root {
                DynamicText { id } | Dynamic { id } => {
                    nodes.next().unwrap();
                    self.write_dynamic_root(mount, *id, dom, to)
                }
                Element { .. } => {
                    #[cfg(not(debug_assertions))]
                    let id =
                        self.write_element_root(mount, idx, &mut attrs, &mut nodes, &[], dom, to);
                    #[cfg(debug_assertions)]
                    let id = self.write_element_root(
                        mount,
                        idx,
                        &mut attrs,
                        &mut nodes,
                        &nodes_sorted,
                        dom,
                        to,
                    );
                    id
                }
                TemplateNode::Text { .. } => self.write_static_text_root(mount, idx, dom, to),
            })
            .sum()
    }
}

impl VNode {
    fn write_static_text_root(
        &self,
        mount: MountId,
        idx: usize,
        dom: &mut VirtualDom,
        to: &mut impl WriteMutations,
    ) -> usize {
        // Simply just load the template root, no modifications needed
        self.load_template_root(mount, idx, dom, to);

        // Text produces just one node on the stack
        1
    }

    fn write_dynamic_root(
        &self,
        mount: MountId,
        idx: usize,
        dom: &mut VirtualDom,
        to: &mut impl WriteMutations,
    ) -> usize {
        use DynamicNode::*;
        match &self.dynamic_nodes[idx] {
            Component(component) => {
                let parent = Some(ElementRef {
                    path: ElementPath {
                        path: self.template.get().node_paths[idx],
                    },
                    mount,
                });
                self.create_component_node(mount, idx, component, parent, dom, to)
            }
            Fragment(frag) => {
                let parent = Some(ElementRef {
                    path: ElementPath {
                        path: self.template.get().node_paths[idx],
                    },
                    mount,
                });
                dom.create_children(to, frag, parent)
            }
            Placeholder(_) => {
                let id = mount.mount_node(idx, dom);
                to.create_placeholder(id);
                1
            }
            Text(VText { value }) => {
                let id = mount.mount_node(idx, dom);
                to.create_text_node(value, id);
                1
            }
        }
    }

    /// We write all the descendent data for this element
    ///
    /// Elements can contain other nodes - and those nodes can be dynamic or static
    ///
    /// We want to make sure we write these nodes while on top of the root
    fn write_element_root(
        &self,
        mount: MountId,
        root_idx: usize,
        dynamic_attrs: &mut Peekable<impl Iterator<Item = (usize, &'static [u8])>>,
        dynamic_nodes_iter: &mut Peekable<impl Iterator<Item = ((usize, usize), &'static [u8])>>,
        dynamic_nodes: &[(usize, &'static [u8])],
        dom: &mut VirtualDom,
        to: &mut impl WriteMutations,
    ) -> usize {
        // Load the template root and get the ID for the node on the stack
        let root_on_stack = self.load_template_root(mount, root_idx, dom, to);

        // Write all the attributes below this root
        self.write_attrs_on_root(mount, dynamic_attrs, root_idx as u8, root_on_stack, dom, to);

        // Load in all of the placeholder or dynamic content under this root too
        self.load_placeholders(
            mount,
            dynamic_nodes_iter,
            dynamic_nodes,
            root_idx as u8,
            dom,
            to,
        );

        1
    }

    /// Load all of the placeholder nodes for descendents of this root node
    ///
    /// ```rust, ignore
    /// rsx! {
    ///     div {
    ///         // This is a placeholder
    ///         some_value,
    ///
    ///         // Load this too
    ///         "{some_text}"
    ///     }
    /// }
    /// ```
    #[allow(unused)]
    fn load_placeholders(
        &self,
        mount: MountId,
        dynamic_nodes_iter: &mut Peekable<impl Iterator<Item = ((usize, usize), &'static [u8])>>,
        dynamic_nodes: &[(usize, &'static [u8])],
        root_idx: u8,
        dom: &mut VirtualDom,
        to: &mut impl WriteMutations,
    ) {
        let (start, end) = match collect_dyn_node_range(dynamic_nodes_iter, root_idx) {
            Some((a, b)) => (a, b),
            None => return,
        };

        // If hot reloading is enabled, we need to map the sorted index to the original index of the dynamic node. If it is disabled, we can just use the sorted index
        #[cfg(not(debug_assertions))]
        let reversed_iter = (start..=end).rev();
        #[cfg(debug_assertions)]
        let reversed_iter = (start..=end)
            .rev()
            .map(|sorted_index| dynamic_nodes[sorted_index].0);

        for idx in reversed_iter {
            let m = self.create_dynamic_node(mount, idx, dom, to);
            if m > 0 {
                // The path is one shorter because the top node is the root
                let path = &self.template.get().node_paths[idx][1..];
                to.replace_placeholder_with_nodes(path, m);
            }
        }
    }

    fn write_attrs_on_root(
        &self,
        mount: MountId,
        attrs: &mut Peekable<impl Iterator<Item = (usize, &'static [u8])>>,
        root_idx: u8,
        root: ElementId,
        dom: &mut VirtualDom,
        to: &mut impl WriteMutations,
    ) {
        while let Some((mut attr_id, path)) =
            attrs.next_if(|(_, p)| p.first().copied() == Some(root_idx))
        {
            let id = self.assign_static_node_as_dynamic(path, root, dom, to);

            loop {
                self.write_attribute(mount, attr_id, id, dom, to);

                // Only push the dynamic attributes forward if they match the current path (same element)
                match attrs.next_if(|(_, p)| *p == path) {
                    Some((next_attr_id, _)) => attr_id = next_attr_id,
                    None => break,
                }
            }
        }
    }

    fn write_attribute(
        &self,
        mount: MountId,
        idx: usize,
        id: ElementId,
        dom: &mut VirtualDom,
        to: &mut impl WriteMutations,
    ) {
        // Make sure we set the attribute's associated id
        dom.mounts[mount.0].mounted_attributes[idx] = id;

        let attribute = &self.dynamic_attrs[idx];

        match &attribute.value {
            AttributeValue::Listener(_) => {
                // If this is a listener, we need to create an element reference for it so that when we receive an event, we can find the element
                let path = &self.template.get().attr_paths[idx];

                // The mount information should always be in the VDOM at this point
                debug_assert!(dom.mounts.get(mount.0).is_some());

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
    /// If the node in question is on the stack, we just return that ID
    ///
    /// If the node is not on the stack, we create a new ID for it and assign it
    fn assign_static_node_as_dynamic(
        &self,
        path: &'static [u8],
        this_id: ElementId,
        dom: &mut VirtualDom,
        to: &mut impl WriteMutations,
    ) -> ElementId {
        if path.len() == 1 {
            return this_id;
        }

        // if attribute is on a root node, then we've already created the element
        // Else, it's deep in the template and we should create a new id for it
        let id = dom.next_element();

        to.assign_node_id(&path[1..], id);

        id
    }

    pub(crate) fn create_dynamic_node(
        &self,
        mount: MountId,
        index: usize,
        dom: &mut VirtualDom,
        to: &mut impl WriteMutations,
    ) -> usize {
        use DynamicNode::*;
        let node = &self.dynamic_nodes[index];
        match node {
            Text(text) => self.create_dynamic_text(mount, index, text, dom, to),
            Placeholder(_) => self.create_placeholder(mount, index, dom, to),
            Component(component) => {
                let parent = Some(ElementRef {
                    path: ElementPath {
                        path: self.template.get().node_paths[index],
                    },
                    mount,
                });
                self.create_component_node(mount, index, component, parent, dom, to)
            }
            Fragment(frag) => {
                let parent = Some(ElementRef {
                    path: ElementPath {
                        path: self.template.get().node_paths[index],
                    },
                    mount,
                });
                dom.create_children(to, frag, parent)
            }
        }
    }

    /// Mount a root node and return its ID and the path to the node
    fn mount_dynamic_node_with_path(
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
        let (new_id, path) = self.mount_dynamic_node_with_path(mount, idx, dom);

        // Hydrate the text node
        to.hydrate_text_node(path, &text.value, new_id);

        // Since we're hydrating an existing node, we don't create any new nodes
        0
    }

    pub(crate) fn create_placeholder(
        &self,
        mount: MountId,
        idx: usize,
        dom: &mut VirtualDom,
        to: &mut impl WriteMutations,
    ) -> usize {
        let (id, path) = self.mount_dynamic_node_with_path(mount, idx, dom);

        // Assign the ID to the existing node in the template
        to.assign_node_id(path, id);

        // Since the placeholder is already in the DOM, we don't create any new nodes
        0
    }
}

impl MountId {
    fn mount_node(self, node_index: usize, dom: &mut VirtualDom) -> ElementId {
        let id = dom.next_element();
        dom.mounts[self.0].mounted_dynamic_nodes[node_index] = id.0;
        id
    }
}

fn collect_dyn_node_range(
    dynamic_nodes: &mut Peekable<impl Iterator<Item = ((usize, usize), &'static [u8])>>,
    root_idx: u8,
) -> Option<(usize, usize)> {
    let start = match dynamic_nodes.peek() {
        Some(((_, idx), [first, ..])) if *first == root_idx => *idx,
        _ => return None,
    };

    let mut end = start;

    while let Some(((_, idx), p)) =
        dynamic_nodes.next_if(|(_, p)| matches!(p, [idx, ..] if *idx == root_idx))
    {
        if p.len() == 1 {
            continue;
        }

        end = idx;
    }

    Some((start, end))
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

#[cfg(debug_assertions)]
fn sort_bfs(paths: &[&'static [u8]]) -> Vec<(usize, &'static [u8])> {
    let mut with_indecies = paths.iter().copied().enumerate().collect::<Vec<_>>();
    with_indecies.sort_unstable_by(|(_, a), (_, b)| {
        let mut a = a.iter();
        let mut b = b.iter();
        loop {
            match (a.next(), b.next()) {
                (Some(a), Some(b)) => {
                    if a != b {
                        return a.cmp(b);
                    }
                }
                // The shorter path goes first
                (None, Some(_)) => return std::cmp::Ordering::Less,
                (Some(_), None) => return std::cmp::Ordering::Greater,
                (None, None) => return std::cmp::Ordering::Equal,
            }
        }
    });
    with_indecies
}

#[test]
#[cfg(debug_assertions)]
fn sorting() {
    let r: [(usize, &[u8]); 5] = [
        (0, &[0, 1]),
        (1, &[0, 2]),
        (2, &[1, 0]),
        (3, &[1, 0, 1]),
        (4, &[1, 2]),
    ];
    assert_eq!(
        sort_bfs(&[&[0, 1,], &[0, 2,], &[1, 0,], &[1, 0, 1,], &[1, 2,],]),
        r
    );
    let r: [(usize, &[u8]); 6] = [
        (0, &[0]),
        (1, &[0, 1]),
        (2, &[0, 1, 2]),
        (3, &[1]),
        (4, &[1, 2]),
        (5, &[2]),
    ];
    assert_eq!(
        sort_bfs(&[&[0], &[0, 1], &[0, 1, 2], &[1], &[1, 2], &[2],]),
        r
    );
}
