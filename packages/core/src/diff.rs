//! This module contains all the code for creating and diffing nodes.
//!
//! For suspense there are three different cases we need to handle:
//! - Creating nodes/scopes without mounting them
//! - Diffing nodes that are not mounted
//! - Mounted nodes that have already been created

#![allow(clippy::too_many_arguments)]

use crate::{
    any_props::AnyProps,
    arena::ElementPath,
    innerlude::{
        ElementRef, MountId, ScopeOrder, SuspenseBoundaryProps, SuspenseBoundaryPropsWithOwner,
    },
    nodes::{VNode, VNodeMount},
    scopes::{LastRenderedNode, ScopeId},
    Attribute, AttributeValue, DynamicNode, Element, ElementId, RenderError, Runtime,
    SuspenseContext, TemplateNode, VComponent, VText, VirtualDom, WriteMutations,
};
use rustc_hash::{FxHashMap, FxHashSet};
use std::{
    any::TypeId,
    iter::Peekable,
    ops::{Deref, DerefMut},
    rc::Rc,
    usize,
};

/// A fiber progresses a given work tree by running scopes and diffing nodes.
/// It queues work internally such that suspended scopes can be paused and resumed.
///
/// Suspended nodes are flushed to the DOM just like regular nodes, but they are unmounted and thus
/// not interactive. Only once the suspense tree is fully resolved are the nodes mounted and made interactive.
pub(crate) struct Fiber<'a, 'b, M: WriteMutations> {
    dom: &'a mut VirtualDom,
    to: &'b mut M,
}

impl<'a, 'b, M: WriteMutations> Fiber<'a, 'b, M> {
    /// When we create a new fiber, we keep track of the current suspense context
    pub(crate) fn new(dom: &'a mut VirtualDom, to: &'b mut M) -> Self {
        Self { dom, to }
    }

    /// Swaps in the real tree for a suspense boundary that has resolved
    pub(crate) fn swap_suspense_tree(&mut self, ctx: &SuspenseContext) {
        let placeholder_ui = ctx
            .inner
            .rendered_fallback_ui
            .borrow_mut()
            .take()
            .expect("suspense boundary to have a rendered fallback UI");

        let last_rendered = self.dom.scopes[ctx.inner.id.get().0]
            .last_rendered_node
            .as_ref()
            .unwrap()
            .clone();

        let num_old_nodes = self.push_all_root_nodes(last_rendered.as_vnode());
        self.remove_node_inner(&placeholder_ui, false, Some(num_old_nodes), true);
    }

    /// Run and diff a scope, creating the scope if it doesn't already exist
    pub(crate) fn run_and_diff_scope(&mut self, scope_id: ScopeId) {
        let new_nodes = self.dom.run_scope(scope_id);
        self.diff_scope(scope_id, new_nodes);
    }

    fn diff_scope(&mut self, scope: ScopeId, new_nodes: LastRenderedNode) {
        self.dom.runtime.clone().with_scope_on_stack(scope, || {
            // Load the old and new rendered nodes
            let old = self.dom.scopes[scope.0].last_rendered_node.take().unwrap();

            // We load up the new node as its placeholder or real node
            let new_real_nodes = new_nodes.as_vnode();

            // If there are suspended scopes, we need to check if the scope is suspended before we diff it
            // If it is suspended, we need to diff it but write the mutations nothing
            // Note: It is important that we still diff the scope even if it is suspended, because the scope may render other child components which may change between renders
            self.diff_node(old.as_vnode(), new_real_nodes);
            self.dom.scopes[scope.0].last_rendered_node = Some(new_nodes);

            if self.dom.runtime.should_run_mount_tasks(scope) {
                self.dom.runtime.get_scope(scope).mount(&self.dom.runtime);
            }
        })
    }

    /// Create a new [`Scope`](crate::scope_context::Scope) for a component.
    ///
    /// Returns the number of nodes created on the stack
    pub(crate) fn create_scope(
        &mut self,
        scope: ScopeId,
        new_nodes: LastRenderedNode,
        parent: Option<ElementRef>,
    ) -> usize {
        // If there are suspended scopes, we need to check if the scope is suspended before we diff it
        // If it is suspended, we need to diff it but write the mutations to nothing
        //
        // Note: It is important that we still diff the scope even if it is suspended, because the
        // scope may render other child components which may change between renders
        self.dom.runtime.clone().with_scope_on_stack(scope, || {
            // Create the node. This will run the fiber until we hit a suspense scope which returns a placeholder
            let num_nodes = self.create(new_nodes.as_vnode(), parent);

            // Then set the new node as the last rendered node
            self.dom.scopes[scope.0].last_rendered_node = Some(new_nodes);

            // Run mount tasks
            if self.dom.runtime.should_run_mount_tasks(scope) {
                self.dom.runtime.get_scope(scope).mount(&self.dom.runtime);
            }

            // Check if this scope is a suspense boundary. If it is, and its suspended after we run it,
            // then we need to swap it with its fallback UI.
            if let Some(ctx) = self.dom.runtime.has_context::<SuspenseContext>(scope) {
                if ctx.has_suspended_tasks() {
                    // Pop the nodes we just created, saving them into a fragment so we can retrieve them once they're ready
                    self.to.save_nodes(num_nodes);

                    // assert!(ctx.inner.rendered_fallback_ui.borrow().is_none());
                    tracing::error!("Suspense boundary in scope {:#?} has suspended tasks, rendering fallback UI", scope);

                    let fallback_ui = (ctx.inner.fallback.as_ref())(ctx.clone())
                        .expect("Fallback UI cannot return suspense or errors!");

                    tracing::debug!("fallback ui: {:#?}", fallback_ui);

                    // Create the placeholder in place, returning that instead
                    let num_created = self.create(&fallback_ui, parent);

                    // Attach the fallback UI to the suspense context so we can remove it later
                    *ctx.inner.rendered_fallback_ui.borrow_mut() = Some(fallback_ui);

                    return num_created;
                }
            }

            // Otherwise, just return the number of nodes we created
            num_nodes
        })
    }

    fn remove_component_node(
        &mut self,
        destroy_component_state: bool,
        scope_id: ScopeId,
        replace_with: Option<usize>,
        emit: bool,
    ) {
        // If this is a suspense boundary, remove the suspended nodes as well
        self.remove_suspended_nodes(scope_id, destroy_component_state);

        // Remove the component from the dom
        if let Some(node) = self.dom.scopes[scope_id.0].last_rendered_node.as_ref() {
            self.remove_node_inner(
                node.clone().as_vnode(),
                destroy_component_state,
                replace_with,
                emit,
            );
        };

        // Now drop all the resources
        if destroy_component_state {
            self.drop_scope(scope_id);
        }
    }

    fn diff_non_empty_fragment(
        &mut self,
        old: &[VNode],
        new: &[VNode],
        parent: Option<ElementRef>,
    ) {
        let new_is_keyed = new[0].key.is_some();
        let old_is_keyed = old[0].key.is_some();
        debug_assert!(
            new.iter().all(|n| n.key.is_some() == new_is_keyed),
            "all siblings must be keyed or all siblings must be non-keyed"
        );
        debug_assert!(
            old.iter().all(|o| o.key.is_some() == old_is_keyed),
            "all siblings must be keyed or all siblings must be non-keyed"
        );

        if new_is_keyed && old_is_keyed {
            self.diff_keyed_children(old, new, parent);
        } else {
            self.diff_non_keyed_children(old, new, parent);
        }
    }

    // Diff children that are not keyed.
    //
    // The parent must be on the top of the change list stack when entering this
    // function:
    //
    //     [... parent]
    //
    // the change list stack is in the same state when this function returns.
    fn diff_non_keyed_children(
        &mut self,
        old: &[VNode],
        new: &[VNode],
        parent: Option<ElementRef>,
    ) {
        use std::cmp::Ordering;

        // Handled these cases in `diff_children` before calling this function.
        debug_assert!(!new.is_empty());
        debug_assert!(!old.is_empty());

        match old.len().cmp(&new.len()) {
            Ordering::Greater => self.remove_nodes(&old[new.len()..], None),
            Ordering::Less => {
                self.create_and_insert_after(&new[old.len()..], old.last().unwrap(), parent)
            }
            Ordering::Equal => {}
        }

        for (new, old) in new.iter().zip(old.iter()) {
            self.diff_node(old, new);
        }
    }

    // Diffing "keyed" children.
    //
    // With keyed children, we care about whether we delete, move, or create nodes
    // versus mutate existing nodes in place. Presumably there is some sort of CSS
    // transition animation that makes the virtual DOM diffing algorithm
    // observable. By specifying keys for nodes, we know which virtual DOM nodes
    // must reuse (or not reuse) the same physical DOM nodes.
    //
    // This is loosely based on Inferno's keyed patching implementation. However, we
    // have to modify the algorithm since we are compiling the diff down into change
    // list instructions that will be executed later, rather than applying the
    // changes to the DOM directly as we compare virtual DOMs.
    //
    // https://github.com/infernojs/inferno/blob/36fd96/packages/inferno/src/DOM/patching.ts#L530-L739
    //
    // The stack is empty upon entry.
    fn diff_keyed_children(&mut self, old: &[VNode], new: &[VNode], parent: Option<ElementRef>) {
        if cfg!(debug_assertions) {
            let mut keys = rustc_hash::FxHashSet::default();
            let mut assert_unique_keys = |children: &[VNode]| {
                keys.clear();
                for child in children {
                    let key = child.key.clone();
                    debug_assert!(
                        key.is_some(),
                        "if any sibling is keyed, all siblings must be keyed"
                    );
                    keys.insert(key);
                }
                debug_assert_eq!(
                    children.len(),
                    keys.len(),
                    "keyed siblings must each have a unique key"
                );
            };
            assert_unique_keys(old);
            assert_unique_keys(new);
        }

        // First up, we diff all the nodes with the same key at the beginning of the
        // children.
        //
        // `shared_prefix_count` is the count of how many nodes at the start of
        // `new` and `old` share the same keys.
        let (left_offset, right_offset) = match self.diff_keyed_ends(old, new, parent) {
            Some(count) => count,
            None => return,
        };

        // Ok, we now hopefully have a smaller range of children in the middle
        // within which to re-order nodes with the same keys, remove old nodes with
        // now-unused keys, and create new nodes with fresh keys.

        let old_middle = &old[left_offset..(old.len() - right_offset)];
        let new_middle = &new[left_offset..(new.len() - right_offset)];

        debug_assert!(
            !old_middle.is_empty(),
            "Old middle returned from `diff_keyed_ends` should not be empty"
        );
        debug_assert!(
            !new_middle.is_empty(),
            "New middle returned from `diff_keyed_ends` should not be empty"
        );

        // A few nodes in the middle were removed, just remove the old nodes
        if new_middle.is_empty() {
            self.remove_nodes(old_middle, None);
        } else {
            self.diff_keyed_middle(old_middle, new_middle, parent);
        }
    }

    /// Diff both ends of the children that share keys.
    ///
    /// Returns a left offset and right offset of that indicates a smaller section to pass onto the middle diffing.
    ///
    /// If there is no offset, then this function returns None and the diffing is complete.
    fn diff_keyed_ends(
        &mut self,
        old: &[VNode],
        new: &[VNode],
        parent: Option<ElementRef>,
    ) -> Option<(usize, usize)> {
        let mut left_offset = 0;

        for (old, new) in old.iter().zip(new.iter()) {
            // abort early if we finally run into nodes with different keys
            if old.key != new.key {
                break;
            }
            self.diff_node(old, new);
            left_offset += 1;
        }

        // If that was all of the old children, then create and append the remaining
        // new children and we're finished.
        if left_offset == old.len() {
            self.create_and_insert_after(&new[left_offset..], &new[left_offset - 1], parent);
            return None;
        }

        // if the shared prefix is less than either length, then we need to walk backwards
        let mut right_offset = 0;
        for (old, new) in old.iter().rev().zip(new.iter().rev()) {
            // abort early if we finally run into nodes with different keys
            if old.key != new.key {
                break;
            }
            self.diff_node(old, new);
            right_offset += 1;
        }

        // If that was all of the old children, then create and prepend the remaining
        // new children and we're finished.
        if right_offset == old.len() {
            self.create_and_insert_before(
                &new[..new.len() - right_offset],
                &new[new.len() - right_offset],
                parent,
            );
            return None;
        }

        // If the right offset + the left offset is the same as the new length, then we just need to remove the old nodes
        if right_offset + left_offset == new.len() {
            self.remove_nodes(&old[left_offset..old.len() - right_offset], None);
            return None;
        }

        // If the right offset + the left offset is the same as the old length, then we just need to add the new nodes
        if right_offset + left_offset == old.len() {
            self.create_and_insert_before(
                &new[left_offset..new.len() - right_offset],
                &new[new.len() - right_offset],
                parent,
            );
            return None;
        }

        Some((left_offset, right_offset))
    }

    // The most-general, expensive code path for keyed children diffing.
    //
    // We find the longest subsequence within `old` of children that are relatively
    // ordered the same way in `new` (via finding a longest-increasing-subsequence
    // of the old child's index within `new`). The children that are elements of
    // this subsequence will remain in place, minimizing the number of DOM moves we
    // will have to do.
    //
    // Upon entry to this function, the change list stack must be empty.
    //
    // This function will load the appropriate nodes onto the stack and do diffing in place.
    //
    // Upon exit from this function, it will be restored to that same self.
    #[allow(clippy::too_many_lines)]
    fn diff_keyed_middle(&mut self, old: &[VNode], new: &[VNode], parent: Option<ElementRef>) {
        /*
        1. Map the old keys into a numerical ordering based on indices.
        2. Create a map of old key to its index
        3. Map each new key to the old key, carrying over the old index.
            - IE if we have ABCD becomes BACD, our sequence would be 1,0,2,3
            - if we have ABCD to ABDE, our sequence would be 0,1,3,MAX because E doesn't exist

        now, we should have a list of integers that indicates where in the old list the new items map to.

        4. Compute the LIS of this list
            - this indicates the longest list of new children that won't need to be moved.

        5. Identify which nodes need to be removed
        6. Identify which nodes will need to be diffed

        7. Going along each item in the new list, create it and insert it before the next closest item in the LIS.
            - if the item already existed, just move it to the right place.

        8. Finally, generate instructions to remove any old children.
        9. Generate instructions to finally diff children that are the same between both
        */
        // 0. Debug sanity checks
        // Should have already diffed the shared-key prefixes and suffixes.
        debug_assert_ne!(new.first().map(|i| &i.key), old.first().map(|i| &i.key));
        debug_assert_ne!(new.last().map(|i| &i.key), old.last().map(|i| &i.key));

        // 1. Map the old keys into a numerical ordering based on indices.
        // 2. Create a map of old key to its index
        // IE if the keys were A B C, then we would have (A, 0) (B, 1) (C, 2).
        let old_key_to_old_index = old
            .iter()
            .enumerate()
            .map(|(i, o)| (o.key.as_ref().unwrap().as_str(), i))
            .collect::<FxHashMap<_, _>>();

        let mut shared_keys = FxHashSet::default();

        // 3. Map each new key to the old key, carrying over the old index.
        let new_index_to_old_index = new
            .iter()
            .map(|node| {
                let key = node.key.as_ref().unwrap();
                if let Some(&index) = old_key_to_old_index.get(key.as_str()) {
                    shared_keys.insert(key);
                    index
                } else {
                    usize::MAX
                }
            })
            .collect::<Box<[_]>>();

        // If none of the old keys are reused by the new children, then we remove all the remaining old children and
        // create the new children afresh.
        if shared_keys.is_empty() {
            debug_assert!(
                !old.is_empty(),
                "we should never be appending - just creating N"
            );

            let m = self.create_children(new, parent);
            self.remove_nodes(old, Some(m));

            return;
        }

        // remove any old children that are not shared
        for child_to_remove in old
            .iter()
            .filter(|child| !shared_keys.contains(child.key.as_ref().unwrap()))
        {
            self.remove_node(child_to_remove, None);
        }

        // 4. Compute the LIS of this list
        let mut lis_sequence = Vec::with_capacity(new_index_to_old_index.len());

        let mut allocation = vec![0; new_index_to_old_index.len() * 2];
        let (predecessors, starts) = allocation.split_at_mut(new_index_to_old_index.len());

        longest_increasing_subsequence::lis_with(
            &new_index_to_old_index,
            &mut lis_sequence,
            |a, b| a < b,
            predecessors,
            starts,
        );

        // if a new node gets u32 max and is at the end, then it might be part of our LIS (because u32 max is a valid LIS)
        if lis_sequence.first().map(|f| new_index_to_old_index[*f]) == Some(usize::MAX) {
            lis_sequence.remove(0);
        }

        // Diff each nod in the LIS
        for idx in &lis_sequence {
            self.diff_node(&old[new_index_to_old_index[*idx]], &new[*idx]);
        }

        // add mount instruction for the items before the LIS
        let last = *lis_sequence.first().unwrap();
        if last < (new.len() - 1) {
            let nodes_created = self.create_or_diff(
                new,
                old,
                parent,
                &new_index_to_old_index,
                (last + 1)..new.len(),
            );

            // Insert all the nodes that we just created after the last node in the LIS
            self.insert_after(nodes_created, &new[last]);
        }

        // For each node inside of the LIS, but not included in the LIS, generate a mount instruction
        // We loop over the LIS in reverse order and insert any nodes we find in the gaps between indexes
        let mut lis_iter = lis_sequence.iter();
        let mut last = *lis_iter.next().unwrap();
        for next in lis_iter {
            if last - next > 1 {
                let nodes_created = self.create_or_diff(
                    new,
                    old,
                    parent,
                    &new_index_to_old_index,
                    (next + 1)..last,
                );

                self.insert_before(nodes_created, &new[last]);
            }
            last = *next;
        }

        // add mount instruction for the items after the LIS
        let first_lis = *lis_sequence.last().unwrap();
        if first_lis > 0 {
            let nodes_created =
                self.create_or_diff(new, old, parent, &new_index_to_old_index, 0..first_lis);

            self.insert_before(nodes_created, &new[first_lis]);
        }
    }

    /// Create or diff each node in a range depending on whether it is in the LIS or not
    /// Returns the number of nodes created on the stack
    fn create_or_diff(
        &mut self,
        new: &[VNode],
        old: &[VNode],
        parent: Option<ElementRef>,
        new_index_to_old_index: &[usize],
        range: std::ops::Range<usize>,
    ) -> usize {
        let range_start = range.start;
        new[range]
            .iter()
            .enumerate()
            .map(|(idx, new_node)| {
                let new_idx = range_start + idx;
                let old_index = new_index_to_old_index[new_idx];
                // If the node existed in the old list, diff it
                if let Some(old_node) = old.get(old_index) {
                    self.diff_node(old_node, new_node);
                    self.push_all_root_nodes(new_node)
                } else {
                    // Otherwise, just add it to the stack
                    self.create(new_node, parent)
                }
            })
            .sum()
    }

    fn create_and_insert_before(
        &mut self,
        new: &[VNode],
        before: &VNode,
        parent: Option<ElementRef>,
    ) {
        let m = self.create_children(new, parent);
        self.insert_before(m, before);
    }

    fn insert_before(&mut self, new: usize, before: &VNode) {
        if new > 0 {
            self.to
                .insert_nodes_before(self.find_first_element(before), new);
        }
    }

    fn create_and_insert_after(
        &mut self,
        new: &[VNode],
        after: &VNode,
        parent: Option<ElementRef>,
    ) {
        let m = self.create_children(new, parent);
        self.insert_after(m, after);
    }

    fn insert_after(&mut self, new: usize, after: &VNode) {
        if new > 0 {
            self.to
                .insert_nodes_after(self.find_last_element(after), new);
        }
    }

    /// Remove these nodes from the dom
    /// Wont generate mutations for the inner nodes
    fn remove_nodes(&mut self, nodes: &[VNode], replace_with: Option<usize>) {
        for (i, node) in nodes.iter().rev().enumerate() {
            let last_node = i == nodes.len() - 1;
            self.remove_node(node, replace_with.filter(|_| last_node));
        }
    }

    fn diff_vcomponent(
        &mut self,
        mount: MountId,
        idx: usize,
        new: &VComponent,
        old: &VComponent,
        scope_id: ScopeId,
        parent: Option<ElementRef>,
    ) {
        // Replace components that have different render fns
        if old.render_fn != new.render_fn {
            return self.replace_vcomponent(mount, idx, new, parent);
        }

        // copy out the box for both
        let old_scope = &mut self.dom.scopes[scope_id.0];
        let old_props: &mut dyn AnyProps = old_scope.props.deref_mut();
        let new_props: &dyn AnyProps = new.props.deref();

        // If the props are static, then we try to memoize by setting the new with the old
        // The target ScopeState still has the reference to the old props, so there's no need to update anything
        // This also implicitly drops the new props since they're not used
        if old_props.memoize(new_props.props()) {
            tracing::trace!("Memoized props for component {:#?}", scope_id,);
            return;
        }

        // Now diff the scope
        self.run_and_diff_scope(scope_id);

        let height = self.dom.runtime.get_scope(scope_id).height;
        self.dom
            .dirty_scopes
            .remove(&ScopeOrder::new(height, scope_id));
    }

    fn replace_vcomponent(
        &mut self,
        mount: MountId,
        idx: usize,
        new: &VComponent,
        parent: Option<ElementRef>,
    ) {
        let scope = ScopeId(self.get_mounted_dyn_node(mount, idx));

        // Remove the scope id from the mount
        self.set_mounted_dyn_node(mount, idx, ScopeId::PLACEHOLDER.0);
        let m = self.create_component_node(mount, idx, new, parent);

        // Instead of *just* removing it, we can use the replace mutation
        self.remove_component_node(true, scope, Some(m), true);
    }

    /// Create a new component (if it doesn't already exist) node and then mount the [`crate::ScopeState`] for a component
    ///
    /// Returns the number of nodes created on the stack
    pub(super) fn create_component_node(
        &mut self,
        mount: MountId,
        idx: usize,
        component: &VComponent,
        parent: Option<ElementRef>,
    ) -> usize {
        // // If this is a suspense boundary, run our suspense creation logic instead of running the component
        // if component.props.props().type_id() == TypeId::of::<SuspenseBoundaryPropsWithOwner>() {
        //     return self.create_suspense_boundary(mount, idx, component, parent);
        // }

        let mut scope_id = ScopeId(self.get_mounted_dyn_node(mount, idx));

        // If the scopeid is a placeholder, we need to load up a new scope for this vcomponent. If it's already mounted, then we can just use that
        if scope_id.is_placeholder() {
            let parent_id = self.dom.runtime.current_scope_id();

            scope_id = self
                .dom
                .new_scope(component.props.duplicate(), component.name, Some(parent_id))
                .state()
                .id;

            // Store the scope id for the next render
            self.set_mounted_dyn_node(mount, idx, scope_id.0);

            // If this is a new scope, we also need to run it once to get the initial state
            let new = self.dom.run_scope(scope_id);

            // Then set the new node as the last rendered node
            self.dom.scopes[scope_id.0].last_rendered_node = Some(new);
        }

        let scope = ScopeId(self.get_mounted_dyn_node(mount, idx));

        let new_node = self.dom.scopes[scope.0]
            .last_rendered_node
            .as_ref()
            .expect("Component to be mounted")
            .clone();

        self.create_scope(scope, new_node, parent)
    }

    fn diff_node(&mut self, old: &VNode, new: &VNode) {
        // The node we are diffing from should always be mounted
        debug_assert!(
            self.dom
                .runtime
                .mounts
                .borrow()
                .get(old.mount.get().0)
                .is_some() // || !self.write
                           // || !self.write
        );

        // If the templates are different, we need to replace the entire template
        if old.template != new.template {
            let mount_id = old.mount.get();
            let parent = self.get_mounted_parent(mount_id);
            return self.replace(old, std::slice::from_ref(new), parent);
        }

        self.move_mount_to(old, new);

        // If the templates are the same, we don't need to do anything, except copy over the mount information
        if old == new {
            return;
        }

        // If the templates are the same, we can diff the attributes and children
        // Start with the attributes
        // Since the attributes are only side effects, we can skip diffing them entirely if the node is suspended and we aren't outputting mutations
        self.diff_attributes(old, new);

        // Now diff the dynamic nodes
        let mount_id = new.mount.get();
        for (dyn_node_idx, (old_dyn, new_dyn)) in old
            .dynamic_nodes
            .iter()
            .zip(new.dynamic_nodes.iter())
            .enumerate()
        {
            self.diff_dynamic_node(old, mount_id, dyn_node_idx, old_dyn, new_dyn)
        }
    }

    fn move_mount_to(&mut self, old: &VNode, new: &VNode) {
        // Copy over the mount information
        let mount_id = old.mount.take();
        new.mount.set(mount_id);

        if mount_id.mounted() {
            let mut mounts = self.dom.runtime.mounts.borrow_mut();
            let mount = &mut mounts[mount_id.0];

            // Update the reference to the node for bubbling events
            mount.node = new.clone();
        }
    }

    fn diff_dynamic_node(
        &mut self,
        node: &VNode,
        mount: MountId,
        idx: usize,
        old_node: &DynamicNode,
        new_node: &DynamicNode,
    ) {
        use DynamicNode::*;
        match (old_node, new_node) {
            (Text(old), Text(new)) => {
                // Diffing text is just a side effect, if we are diffing suspended nodes and are not outputting mutations, we can skip it
                let id = ElementId(self.get_mounted_dyn_node(mount, idx));
                self.diff_vtext(id, old, new)
            }
            (Placeholder(_), Placeholder(_)) => {}
            (Fragment(old), Fragment(new)) => self.diff_non_empty_fragment(
                old,
                new,
                Some(self.reference_to_dynamic_node(node, mount, idx)),
            ),
            (Component(old), Component(new)) => {
                let scope_id = ScopeId(self.get_mounted_dyn_node(mount, idx));
                self.diff_vcomponent(
                    mount,
                    idx,
                    new,
                    old,
                    scope_id,
                    Some(self.reference_to_dynamic_node(node, mount, idx)),
                )
            }
            (old, new) => {
                // TODO: we should pass around the mount instead of the mount id
                // that would make moving the mount around here much easier

                // Mark the mount as unused. When a scope is created, it reads the mount and
                // if it is the placeholder value, it will create the scope, otherwise it will
                // reuse the scope
                let old_mount = self.get_mounted_dyn_node(mount, idx);
                self.set_mounted_dyn_node(mount, idx, usize::MAX);

                let new_nodes_on_stack = self.create_dynamic_node(node, new, mount, idx);

                // Restore the mount for the scope we are removing
                let new_mount = self.get_mounted_dyn_node(mount, idx);
                self.set_mounted_dyn_node(mount, idx, old_mount);

                // Remvoe the dynamic node, emittin
                self.remove_dynamic_node(mount, true, idx, old, Some(new_nodes_on_stack), true);

                // Restore the mount for the node we created
                self.set_mounted_dyn_node(mount, idx, new_mount);
            }
        };
    }

    fn find_first_element(&self, node: &VNode) -> ElementId {
        use DynamicNode::*;

        let mount_id = node.mount.get();
        let first = match get_dynamic_root_node_and_id(node, 0) {
            // This node is static, just get the root id
            None => self.get_mounted_root_node(mount_id, 0),

            // If it is dynamic and shallow, grab the id from the mounted dynamic nodes
            Some((idx, Placeholder(_) | Text(_))) => {
                ElementId(self.get_mounted_dyn_node(mount_id, idx))
            }

            // The node is a fragment, so we need to find the first element in the fragment
            Some((_, Fragment(children))) => {
                let child = children.first().unwrap();
                self.find_first_element(child)
            }

            // The node is a component, so we need to find the first element in the component
            Some((id, Component(_))) => {
                let scope = ScopeId(self.get_mounted_dyn_node(mount_id, id));
                self.find_first_element(
                    self.dom
                        .get_scope(scope)
                        .expect("Scope should exist")
                        .root_node(),
                )
            }
        };

        // The first element should never be the default element id (the root element)
        debug_assert_ne!(first, ElementId::default());

        first
    }

    fn find_last_element(&self, node: &VNode) -> ElementId {
        use DynamicNode::*;

        let mount_id = node.mount.get();
        let last_root_index = node.template.roots.len() - 1;
        let last = match get_dynamic_root_node_and_id(node, last_root_index) {
            // This node is static, just get the root id
            None => self.get_mounted_root_node(mount_id, last_root_index),

            // If it is dynamic and shallow, grab the id from the mounted dynamic nodes
            Some((idx, Placeholder(_) | Text(_))) => {
                ElementId(self.get_mounted_dyn_node(mount_id, idx))
            }

            // The node is a fragment, so we need to find the first element in the fragment
            Some((_, Fragment(children))) => self.find_first_element(children.first().unwrap()),

            // The node is a component, so we need to find the first element in the component
            Some((id, Component(_))) => {
                let scope = ScopeId(self.get_mounted_dyn_node(mount_id, id));
                let root = self.dom.get_scope(scope).unwrap().root_node();
                self.find_last_element(root)
            }
        };

        // The last element should never be the default element id (the root element)
        debug_assert_ne!(last, ElementId::default());

        last
    }

    /// Diff the two text nodes
    ///
    /// This just sets the text of the node if it's different.
    fn diff_vtext(&mut self, id: ElementId, left: &VText, right: &VText) {
        if left.value != right.value {
            self.to.set_node_text(&right.value, id);
        }
    }

    fn replace(&mut self, old: &VNode, right: &[VNode], parent: Option<ElementRef>) {
        self.replace_inner(old, right, parent, true)
    }

    /// Replace this node with new children, but *don't destroy* the old node's component state
    ///
    /// This is useful for moving a node from the rendered nodes into a suspended node
    fn move_node_to_background(
        &mut self,
        old: &VNode,
        right: &[VNode],
        parent: Option<ElementRef>,
    ) {
        self.replace_inner(old, right, parent, false)
    }

    fn replace_inner(
        &mut self,
        old: &VNode,
        right: &[VNode],
        parent: Option<ElementRef>,
        destroy_component_state: bool,
    ) {
        let m = self.create_children(right, parent);

        // Instead of *just* removing it, we can use the replace mutation
        self.remove_node_inner(old, destroy_component_state, Some(m), true)
    }

    /// Remove a node from the dom and potentially replace it with the top m nodes from the stack
    fn remove_node(&mut self, node: &VNode, replace_with: Option<usize>) {
        self.remove_node_inner(node, true, replace_with, true)
    }

    /// Remove a node, but only maybe destroy the component state of that node. During suspense, we need to remove a node from the real dom without wiping the component state
    fn remove_node_inner(
        &mut self,
        node: &VNode,
        destroy_component_state: bool,
        replace_with: Option<usize>,
        emit: bool,
    ) {
        let mount = node.mount.get();
        if !mount.mounted() {
            return;
        }

        // Clean up any attributes that have claimed a static node as dynamic for mount/unmounts
        // Will not generate mutations!
        self.reclaim_attributes(node, mount);

        // Remove the nested dynamic nodes
        // We don't generate mutations for these, as they will be removed by the parent (in the next line)
        // But we still need to make sure to reclaim them from the arena and drop their hooks, etc
        self.remove_nested_dyn_nodes(node, mount, destroy_component_state);

        // Clean up the roots, assuming we need to generate mutations for these
        // This is done last in order to preserve Node ID reclaim order (reclaim in reverse order of claim)
        self.reclaim_roots(node, mount, destroy_component_state, replace_with, emit);

        if destroy_component_state {
            let mount = node.mount.take();
            // Remove the mount information
            self.dom.runtime.mounts.borrow_mut().remove(mount.0);
        }
    }

    fn reclaim_roots(
        &mut self,
        node: &VNode,
        mount: MountId,
        destroy_component_state: bool,
        replace_with: Option<usize>,
        emit: bool,
    ) {
        let roots = node.template.roots;
        for (idx, new) in roots.iter().enumerate() {
            let last_node = idx == roots.len() - 1;
            if let Some(id) = new.dynamic_id() {
                let dynamic_node = &node.dynamic_nodes[id];
                self.remove_dynamic_node(
                    mount,
                    destroy_component_state,
                    id,
                    dynamic_node,
                    replace_with.filter(|_| last_node),
                    emit,
                );
            } else {
                let id = self.get_mounted_root_node(mount, idx);
                if emit {
                    if let (true, Some(replace_with)) = (last_node, replace_with) {
                        self.to.replace_node_with(id, replace_with);
                    } else {
                        self.to.remove_node(id);
                    }
                    self.reclaim(id);
                }
            }
        }
    }

    fn remove_nested_dyn_nodes(
        &mut self,
        node: &VNode,
        mount: MountId,
        destroy_component_state: bool,
    ) {
        let template = node.template;
        for (idx, dyn_node) in node.dynamic_nodes.iter().enumerate() {
            let path_len = template.node_paths.get(idx).map(|path| path.len());
            // Roots are cleaned up automatically above and nodes with a empty path are placeholders
            if let Some(2..) = path_len {
                self.remove_dynamic_node(
                    mount,
                    destroy_component_state,
                    idx,
                    dyn_node,
                    None,
                    false,
                );
            }
        }
    }

    fn remove_dynamic_node(
        &mut self,
        mount: MountId,
        destroy_component_state: bool,
        idx: usize,
        node: &DynamicNode,
        replace_with: Option<usize>,
        emit: bool,
    ) {
        use DynamicNode::*;
        match node {
            Component(_comp) => {
                let scope_id = ScopeId(self.get_mounted_dyn_node(mount, idx));
                self.remove_component_node(destroy_component_state, scope_id, replace_with, emit);
            }
            Text(_) | Placeholder(_) => {
                let id = ElementId(self.get_mounted_dyn_node(mount, idx));
                if emit {
                    if let Some(replace_with) = replace_with {
                        self.to.replace_node_with(id, replace_with);
                    } else {
                        self.to.remove_node(id);
                    }
                }
                self.reclaim(id)
            }
            Fragment(nodes) => {
                for node in &nodes[..nodes.len() - 1] {
                    self.remove_node_inner(node, destroy_component_state, None, emit)
                }
                if let Some(last_node) = nodes.last() {
                    self.remove_node_inner(last_node, destroy_component_state, replace_with, emit)
                }
            }
        };
    }

    pub(super) fn reclaim_attributes(&mut self, node: &VNode, mount: MountId) {
        let mut next_id = None;
        for (idx, path) in node.template.attr_paths.iter().enumerate() {
            // We clean up the roots in the next step, so don't worry about them here
            if path.len() <= 1 {
                continue;
            }

            // only reclaim the new element if it's different from the previous one
            let new_id = self.get_mounted_dyn_attr(mount, idx);
            if Some(new_id) != next_id {
                self.reclaim(new_id);
                next_id = Some(new_id);
            }
        }
    }

    pub(super) fn diff_attributes(&mut self, old: &VNode, new: &VNode) {
        let mount_id = new.mount.get();
        for (idx, (old_attrs, new_attrs)) in old
            .dynamic_attrs
            .iter()
            .zip(new.dynamic_attrs.iter())
            .enumerate()
        {
            let mut old_attributes_iter = old_attrs.iter().peekable();
            let mut new_attributes_iter = new_attrs.iter().peekable();
            let attribute_id = self.get_mounted_dyn_attr(mount_id, idx);
            let path = old.template.attr_paths[idx];

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
                                // We only need to write the attribute if the attribute is volatile or the value has changed
                                // and this is not an event listener.
                                // Interpreters reference event listeners by name and element id, so we don't need to write them
                                // even if the closure has changed.
                                let attribute_changed = match (&old.value, &new.value) {
                                    (AttributeValue::Text(l), AttributeValue::Text(r)) => l != r,
                                    (AttributeValue::Float(l), AttributeValue::Float(r)) => l != r,
                                    (AttributeValue::Int(l), AttributeValue::Int(r)) => l != r,
                                    (AttributeValue::Bool(l), AttributeValue::Bool(r)) => l != r,
                                    (AttributeValue::Any(l), AttributeValue::Any(r)) => {
                                        !l.as_ref().any_cmp(r.as_ref())
                                    }
                                    (AttributeValue::None, AttributeValue::None) => false,
                                    (AttributeValue::Listener(_), AttributeValue::Listener(_)) => {
                                        false
                                    }
                                    _ => true,
                                };
                                if volatile || attribute_changed {
                                    self.write_attribute(path, new, attribute_id, mount_id);
                                }
                            }
                            // In a sorted list, if the old attribute name is first, then the new attribute is missing
                            std::cmp::Ordering::Less => {
                                let old = old_attributes_iter.next().unwrap();
                                self.remove_attribute(old, attribute_id);
                            }
                            // In a sorted list, if the new attribute name is first, then the old attribute is missing
                            std::cmp::Ordering::Greater => {
                                let new = new_attributes_iter.next().unwrap();
                                self.write_attribute(path, new, attribute_id, mount_id);
                            }
                        }
                    }
                    (Some(_), None) => {
                        let left = old_attributes_iter.next().unwrap();
                        self.remove_attribute(left, attribute_id)
                    }
                    (None, Some(_)) => {
                        let right = new_attributes_iter.next().unwrap();
                        self.write_attribute(path, right, attribute_id, mount_id)
                    }
                    (None, None) => break,
                }
            }
        }
    }

    fn remove_attribute(&mut self, attribute: &Attribute, id: ElementId) {
        match &attribute.value {
            AttributeValue::Listener(_) => {
                self.to.remove_event_listener(&attribute.name[2..], id);
            }
            _ => {
                self.to.set_attribute(
                    attribute.name,
                    attribute.namespace,
                    &AttributeValue::None,
                    id,
                );
            }
        }
    }

    fn write_attribute(
        &mut self,
        path: &'static [u8],
        attribute: &Attribute,
        id: ElementId,
        mount: MountId,
    ) {
        match &attribute.value {
            AttributeValue::Listener(_) => {
                let element_ref = ElementRef {
                    path: ElementPath { path },
                    mount,
                };
                let mut elements = self.dom.runtime.elements.borrow_mut();
                elements[id.0] = Some(element_ref);
                self.to.create_event_listener(&attribute.name[2..], id);
            }
            _ => {
                self.to
                    .set_attribute(attribute.name, attribute.namespace, &attribute.value, id);
            }
        }
    }

    /// Create this rsx block. This will create scopes from components that this rsx block contains, but it will not write anything to the self.
    fn create(&mut self, node: &VNode, parent: Option<ElementRef>) -> usize {
        // Get the most up to date template
        let template = node.template;

        // Initialize the mount information for this vnode if it isn't already mounted
        if !node.mount.get().mounted() {
            let mut mounts = self.dom.runtime.mounts.borrow_mut();
            let entry = mounts.vacant_entry();
            let mount = MountId(entry.key());
            node.mount.set(mount);
            entry.insert(VNodeMount {
                node: node.clone(),
                parent,
                root_ids: vec![ElementId(usize::MAX - 1); template.roots.len()].into_boxed_slice(),
                mounted_attributes: vec![ElementId(usize::MAX - 1); template.attr_paths.len()]
                    .into_boxed_slice(),
                mounted_dynamic_nodes: vec![usize::MAX; template.node_paths.len()]
                    .into_boxed_slice(),
            });
        }

        // Walk the roots, creating nodes and assigning IDs
        // nodes in an iterator of (dynamic_node_index, path) and attrs in an iterator of (attr_index, path)
        let mut nodes = template.node_paths.iter().copied().enumerate().peekable();
        let mut attrs = template.attr_paths.iter().copied().enumerate().peekable();

        // Get the mounted id of this block
        // At this point, we should have already mounted the block
        debug_assert!(
            self.dom.runtime.mounts.borrow().contains(
                node.mount
                    .get()
                    .as_usize()
                    .expect("node should already be mounted"),
            ),
            "Tried to find mount {:?} in node.mounts, but it wasn't there",
            node.mount.get()
        );
        let mount = node.mount.get();

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
                        self.create_dynamic_node(node, &node.dynamic_nodes[*id], mount, *id)
                    }
                    // For static text and element nodes, just load the template root. This may be a placeholder or just a static node. We now know that each root node has a unique id
                    TemplateNode::Text { .. } | TemplateNode::Element { .. } => {
                        self.load_template_root(node, mount, root_idx);

                        // If this is an element, load in all of the placeholder or dynamic content under this root element too
                        if matches!(root, TemplateNode::Element { .. }) {
                            // !!VERY IMPORTANT!!
                            // Write out all attributes before we load the children. Loading the children will change paths we rely on
                            // to assign ids to elements with dynamic attributes
                            self.write_attrs(node, mount, &mut attrs, root_idx as u8);

                            // This operation relies on the fact that the root node is the top node on the stack so we need to do it here
                            self.load_placeholders(node, mount, &mut nodes, root_idx as u8);
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

    /// Get a reference back into a dynamic node
    fn reference_to_dynamic_node(
        &self,
        node: &VNode,
        mount: MountId,
        dynamic_node_id: usize,
    ) -> ElementRef {
        ElementRef {
            path: ElementPath {
                path: node.template.node_paths[dynamic_node_id],
            },
            mount,
        }
    }

    fn create_dynamic_node(
        &mut self,
        node: &VNode,
        dyn_node: &DynamicNode,
        mount: MountId,
        dynamic_node_id: usize,
    ) -> usize {
        use crate::DynamicNode::*;
        match dyn_node {
            Component(component) => {
                let parent = Some(self.reference_to_dynamic_node(node, mount, dynamic_node_id));
                self.create_component_node(mount, dynamic_node_id, component, parent)
            }
            Fragment(frag) => {
                let parent = Some(self.reference_to_dynamic_node(node, mount, dynamic_node_id));
                self.create_children(frag, parent)
            }
            Text(text) => {
                // If we are diffing suspended nodes and are not outputting mutations, we can skip it
                self.create_dynamic_text(mount, dynamic_node_id, text)
            }
            Placeholder(_) => {
                // If we are diffing suspended nodes and are not outputting mutations, we can skip it
                self.create_placeholder(mount, dynamic_node_id)
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
        &mut self,
        node: &VNode,
        mount: MountId,
        dynamic_nodes_iter: &mut Peekable<impl Iterator<Item = (usize, &'static [u8])>>,
        root_idx: u8,
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
                if p.len() == 1 {
                    continue;
                }

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
                node,
                &node.dynamic_nodes[dynamic_node_id],
                mount,
                dynamic_node_id,
            );

            // If we actually created real new nodes, we need to replace the placeholder for this dynamic node with the new dynamic nodes
            if m > 0 {
                // The path is one shorter because the top node is the root
                let path = &node.template.node_paths[dynamic_node_id][1..];
                self.to.replace_placeholder_with_nodes(path, m);
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
        &mut self,
        node: &VNode,
        mount: MountId,
        dynamic_attrbiutes_iter: &mut Peekable<impl Iterator<Item = (usize, &'static [u8])>>,
        root_idx: u8,
    ) {
        let mut last_path = None;
        // Only take nodes that are under this root node
        let from_root_node = |(_, path): &(usize, &[u8])| path.first() == Some(&root_idx);
        while let Some((attribute_idx, attribute_path)) =
            dynamic_attrbiutes_iter.next_if(from_root_node)
        {
            let attribute = &node.dynamic_attrs[attribute_idx];

            let id = match last_path {
                // If the last path was exactly the same, we can reuse the id
                Some((path, id)) if path == attribute_path => id,
                // Otherwise, we need to create a new id
                _ => {
                    let id = self.assign_static_node_as_dynamic(mount, attribute_path);
                    last_path = Some((attribute_path, id));
                    id
                }
            };

            for attr in &**attribute {
                self.write_attribute(attribute_path, attr, id, mount);
                self.set_mounted_dyn_attr(mount, attribute_idx, id);
            }
        }
    }

    fn create_children(&mut self, nodes: &[VNode], parent: Option<ElementRef>) -> usize {
        nodes.iter().map(|child| self.create(child, parent)).sum()
    }

    fn get_mounted_parent(&self, mount: MountId) -> Option<ElementRef> {
        self.dom.runtime.mounts.borrow()[mount.0].parent
    }

    fn get_mounted_dyn_node(&self, mount: MountId, dyn_node_idx: usize) -> usize {
        self.dom.runtime.mounts.borrow()[mount.0].mounted_dynamic_nodes[dyn_node_idx]
    }

    fn set_mounted_dyn_node(&self, mount: MountId, dyn_node_idx: usize, value: usize) {
        self.dom.runtime.mounts.borrow_mut()[mount.0].mounted_dynamic_nodes[dyn_node_idx] = value;
    }

    fn get_mounted_dyn_attr(&self, mount: MountId, dyn_attr_idx: usize) -> ElementId {
        self.dom.runtime.mounts.borrow()[mount.0].mounted_attributes[dyn_attr_idx]
    }

    fn set_mounted_dyn_attr(&self, mount: MountId, dyn_attr_idx: usize, value: ElementId) {
        self.dom.runtime.mounts.borrow_mut()[mount.0].mounted_attributes[dyn_attr_idx] = value;
    }

    fn get_mounted_root_node(&self, mount: MountId, root_idx: usize) -> ElementId {
        self.dom.runtime.mounts.borrow()[mount.0].root_ids[root_idx]
    }

    fn set_mounted_root_node(&self, mount: MountId, root_idx: usize, value: ElementId) {
        self.dom.runtime.mounts.borrow_mut()[mount.0].root_ids[root_idx] = value;
    }

    fn load_template_root(&mut self, node: &VNode, mount: MountId, root_idx: usize) -> ElementId {
        // Get an ID for this root since it's a real root
        let this_id = self.next_element();
        self.set_mounted_root_node(mount, root_idx, this_id);

        self.to.load_template(node.template, root_idx, this_id);

        this_id
    }

    /// We have some dynamic attributes attached to a some node
    ///
    /// That node needs to be loaded at runtime, so we need to give it an ID
    ///
    /// If the node in question is the root node, we just return the ID
    ///
    /// If the node is not on the stack, we create a new ID for it and assign it
    fn assign_static_node_as_dynamic(&mut self, mount: MountId, path: &'static [u8]) -> ElementId {
        // This is just the root node. We already know it's id
        if let [root_idx] = path {
            return self.get_mounted_root_node(mount, *root_idx as usize);
        }

        // The node is deeper in the template and we should create a new id for it
        let id = self.next_element();

        self.to.assign_node_id(&path[1..], id);

        id
    }

    fn create_dynamic_text(&mut self, mount: MountId, idx: usize, text: &VText) -> usize {
        let new_id = self.mount_node(mount, idx);

        // If this is a root node, the path is empty and we need to create a new text node
        self.to.create_text_node(&text.value, new_id);

        // We create one node on the stack
        1
    }

    fn create_placeholder(&mut self, mount: MountId, idx: usize) -> usize {
        let new_id = self.mount_node(mount, idx);

        // If this is a root node, the path is empty and we need to create a new placeholder node
        self.to.create_placeholder(new_id);
        // We create one node on the stack
        1
    }

    /// Push all the root nodes on the stack
    fn push_all_root_nodes(&mut self, node: &VNode) -> usize {
        fn push_all_inner(node: &VNode, dom: &VirtualDom, to: &mut impl WriteMutations) -> usize {
            let template = node.template;

            let mounts = dom.runtime.mounts.borrow();
            let mount = mounts.get(node.mount.get().0).unwrap();

            template
                .roots
                .iter()
                .enumerate()
                .map(
                    |(root_idx, _)| match get_dynamic_root_node_and_id(node, root_idx) {
                        Some((_, DynamicNode::Fragment(nodes))) => {
                            let mut accumulated = 0;
                            for node in nodes {
                                accumulated += push_all_inner(node, dom, to);
                            }
                            accumulated
                        }
                        Some((idx, DynamicNode::Component(_))) => {
                            let scope = ScopeId(mount.mounted_dynamic_nodes[idx]);
                            let node = dom.get_scope(scope).unwrap().root_node();
                            push_all_inner(node, dom, to)
                        }
                        // This is a static root node or a single dynamic node, just push it
                        None | Some((_, DynamicNode::Placeholder(_) | DynamicNode::Text(_))) => {
                            to.push_root(mount.root_ids[root_idx]);
                            1
                        }
                    },
                )
                .sum()
        }

        push_all_inner(node, self.dom, self.to)
    }

    fn remove_suspended_nodes(&mut self, scope_id: ScopeId, destroy_component_state: bool) {
        // todo!()
        // todo!()
        // let Some(scope) =
        //     SuspenseContext::downcast_suspense_boundary_from_scope(self.dom.runtime, scope_id)
        // else {
        //     return;
        // };

        // let Some(ctxx)

        // // Remove the suspended nodes
        // if let Some(node) = self.take_suspended_nodes() {
        //     self.remove_node_inner(&node, destroy_component_state, None)
        // }
    }

    fn mount_node(&mut self, mount: MountId, node_index: usize) -> ElementId {
        let id = self.next_element();
        self.set_mounted_dyn_node(mount, node_index, id.0);
        id
    }

    fn next_element(&mut self) -> ElementId {
        let mut elements = self.dom.runtime.elements.borrow_mut();
        ElementId(elements.insert(None))
    }

    fn reclaim(&mut self, el: ElementId) {
        if !self.try_reclaim(el) {
            tracing::error!("cannot reclaim {:?}", el);
        }
    }

    fn try_reclaim(&mut self, el: ElementId) -> bool {
        // We never reclaim the unmounted elements or the root element
        if el.0 == 0 || el.0 == usize::MAX {
            return true;
        }

        let mut elements = self.dom.runtime.elements.borrow_mut();
        elements.try_remove(el.0).is_some()
    }

    // Drop a scope without dropping its children
    //
    // Note: This will not remove any ids from the arena
    fn drop_scope(&mut self, id: ScopeId) {
        let height = {
            let scope = self.dom.scopes.remove(id.0);
            let context = scope.state();
            context.height
        };

        self.dom.dirty_scopes.remove(&ScopeOrder::new(height, id));

        // If this scope was a suspense boundary, remove it from the resolved scopes
        self.dom.resolved_scopes.retain(|s| s != &id);
    }
}

/// We can apply various optimizations to dynamic nodes that are the single child of their parent.
///
/// IE
///  - for text - we can use SetTextContent
///  - for clearing children we can use RemoveChildren
///  - for appending children we can use AppendChildren
#[allow(dead_code)]
fn is_dyn_node_only_child(node: &VNode, idx: usize) -> bool {
    let template = node.template;
    let path = template.node_paths[idx];

    // use a loop to index every static node's children until the path has run out
    // only break if the last path index is a dynamic node
    let mut static_node = &template.roots[path[0] as usize];

    for i in 1..path.len() - 1 {
        match static_node {
            TemplateNode::Element { children, .. } => static_node = &children[path[i] as usize],
            _ => return false,
        }
    }

    match static_node {
        TemplateNode::Element { children, .. } => children.len() == 1,
        _ => false,
    }
}

/// Try to get the dynamic node and its index for a root node
fn get_dynamic_root_node_and_id(node: &VNode, root_idx: usize) -> Option<(usize, &DynamicNode)> {
    node.template.roots[root_idx]
        .dynamic_id()
        .map(|id| (id, &node.dynamic_nodes[id]))
}
