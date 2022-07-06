#![warn(clippy::pedantic)]
#![allow(clippy::cast_possible_truncation)]

//! This module contains the stateful [`DiffState`] and all methods to diff [`VNode`]s, their properties, and their children.
//!
//! The [`DiffState`] calculates the diffs between the old and new frames, updates the new nodes, and generates a set
//! of mutations for the renderer to apply.
//!
//! ## Notice:
//!
//! The inspiration and code for this module was originally taken from Dodrio (@fitzgen) and then modified to support
//! Components, Fragments, Suspense, `SubTree` memoization, incremental diffing, cancellation, pausing, priority
//! scheduling, and additional batching operations.
//!
//! ## Implementation Details:
//!
//! ### IDs for elements
//! --------------------
//! All nodes are addressed by their IDs.
//! We don't necessarily require that DOM changes happen instantly during the diffing process, so the implementor may choose
//! to batch nodes if it is more performant for their application. The element IDs are indices into the internal element
//! array. The expectation is that implementors will use the ID as an index into a Vec of real nodes, allowing for passive
//! garbage collection as the [`crate::VirtualDom`] replaces old nodes.
//!
//! When new vnodes are created through `cx.render`, they won't know which real node they correspond to. During diffing,
//! we always make sure to copy over the ID. If we don't do this properly, the [`ElementId`] will be populated incorrectly
//! and brick the user's page.
//!
//! ### Fragment Support
//! --------------------
//! Fragments (nodes without a parent) are supported through a combination of "replace with" and anchor vnodes. Fragments
//! can be particularly challenging when they are empty, so the anchor node lets us "reserve" a spot for the empty
//! fragment to be replaced with when it is no longer empty. This is guaranteed by logic in the [`crate::innerlude::NodeFactory`] - it is
//! impossible to craft a fragment with 0 elements - they must always have at least a single placeholder element. Adding
//! "dummy" nodes _is_ inefficient, but it makes our diffing algorithm faster and the implementation is completely up to
//! the platform.
//!
//! Other implementations either don't support fragments or use a "child + sibling" pattern to represent them. Our code is
//! vastly simpler and more performant when we can just create a placeholder element while the fragment has no children.
//!
//! ### Suspense
//! ------------
//! Dioxus implements Suspense slightly differently than React. In React, each fiber is manually progressed until it runs
//! into a promise-like value. React will then work on the next "ready" fiber, checking back on the previous fiber once
//! it has finished its new work. In Dioxus, we use a similar approach, but try to completely render the tree before
//! switching sub-fibers. Instead, each future is submitted into a futures-queue and the node is manually loaded later on.
//! Due to the frequent calls to [`crate::virtual_dom::VirtualDom::work_with_deadline`] we can get the pure "fetch-as-you-render" behavior of React Fiber.
//!
//! We're able to use this approach because we use placeholder nodes - futures that aren't ready still get submitted to
//! DOM, but as a placeholder.
//!
//! Right now, the "suspense" queue is intertwined with hooks. In the future, we should allow any future to drive attributes
//! and contents, without the need for a `use_suspense` hook. In the interim, this is the quickest way to get Suspense working.
//!
//! ## Subtree Memoization
//! -----------------------
//! We also employ "subtree memoization" which saves us from having to check trees which hold no dynamic content. We can
//! detect if a subtree is "static" by checking if its children are "static". Since we dive into the tree depth-first, the
//! calls to "create" propagate this information upwards. Structures like the one below are entirely static:
//! ```rust, ignore
//! rsx!( div { class: "hello world", "this node is entirely static" } )
//! ```
//! Because the subtrees won't be diffed, their "real node" data will be stale (invalid), so it's up to the reconciler to
//! track nodes created in a scope and clean up all relevant data. Support for this is currently WIP and depends on comp-time
//! hashing of the subtree from the rsx! macro. We do a very limited form of static analysis via static string pointers as
//! a way of short-circuiting the most expensive checks.
//!
//! ## Bloom Filter and Heuristics
//! ------------------------------
//! For all components, we employ some basic heuristics to speed up allocations and pre-size bump arenas. The heuristics are
//! currently very rough, but will get better as time goes on. The information currently tracked includes the size of a
//! bump arena after first render, the number of hooks, and the number of nodes in the tree.
//!
//! ## Garbage Collection
//! ---------------------
//! Dioxus uses a passive garbage collection system to clean up old nodes once the work has been completed. This garbage
//! collection is done internally once the main diffing work is complete. After the "garbage" is collected, Dioxus will then
//! start to re-use old keys for new nodes. This results in a passive memory management system that is very efficient.
//!
//! The IDs used by the key/map are just an index into a Vec. This means that Dioxus will drive the key allocation strategy
//! so the client only needs to maintain a simple list of nodes. By default, Dioxus will not manually clean up old nodes
//! for the client. As new nodes are created, old nodes will be over-written.
//!
//! ## Further Reading and Thoughts
//! ----------------------------
//! There are more ways of increasing diff performance here that are currently not implemented.
//! - Strong memoization of subtrees.
//! - Guided diffing.
//! - Certain web-dom-specific optimizations.
//!
//! More info on how to improve this diffing algorithm:
//!  - <https://hacks.mozilla.org/2019/03/fast-bump-allocated-virtual-doms-with-rust-and-wasm/>

use crate::innerlude::{
    AnyProps, ElementId, Mutations, ScopeArena, ScopeId, VComponent, VElement, VFragment, VNode,
    VPlaceholder, VText,
};
use fxhash::{FxHashMap, FxHashSet};
use smallvec::{smallvec, SmallVec};

pub(crate) struct DiffState<'bump> {
    pub(crate) scopes: &'bump ScopeArena,
    pub(crate) mutations: Mutations<'bump>,
    pub(crate) force_diff: bool,
    pub(crate) element_stack: SmallVec<[ElementId; 10]>,
    pub(crate) scope_stack: SmallVec<[ScopeId; 5]>,
}

impl<'b> DiffState<'b> {
    pub fn new(scopes: &'b ScopeArena) -> Self {
        Self {
            scopes,
            mutations: Mutations::new(),
            force_diff: false,
            element_stack: smallvec![],
            scope_stack: smallvec![],
        }
    }

    pub fn diff_scope(&mut self, scopeid: ScopeId) {
        let (old, new) = (self.scopes.wip_head(scopeid), self.scopes.fin_head(scopeid));
        let scope = self.scopes.get_scope(scopeid).unwrap();

        self.scope_stack.push(scopeid);
        self.element_stack.push(scope.container);
        {
            self.diff_node(old, new);
        }
        self.element_stack.pop();
        self.scope_stack.pop();

        self.mutations.mark_dirty_scope(scopeid);
    }

    pub fn diff_node(&mut self, old_node: &'b VNode<'b>, new_node: &'b VNode<'b>) {
        use VNode::{Component, Element, Fragment, Placeholder, Text};
        match (old_node, new_node) {
            (Text(old), Text(new)) => {
                self.diff_text_nodes(old, new, old_node, new_node);
            }

            (Placeholder(old), Placeholder(new)) => {
                self.diff_placeholder_nodes(old, new, old_node, new_node);
            }

            (Element(old), Element(new)) => {
                self.diff_element_nodes(old, new, old_node, new_node);
            }

            (Component(old), Component(new)) => {
                self.diff_component_nodes(old_node, new_node, *old, *new);
            }

            (Fragment(old), Fragment(new)) => {
                self.diff_fragment_nodes(old, new);
            }

            (
                Component(_) | Fragment(_) | Text(_) | Element(_) | Placeholder(_),
                Component(_) | Fragment(_) | Text(_) | Element(_) | Placeholder(_),
            ) => self.replace_node(old_node, new_node),
        }
    }

    pub fn create_node(&mut self, node: &'b VNode<'b>) -> usize {
        match node {
            VNode::Text(vtext) => self.create_text_node(vtext, node),
            VNode::Placeholder(anchor) => self.create_anchor_node(anchor, node),
            VNode::Element(element) => self.create_element_node(element, node),
            VNode::Fragment(frag) => self.create_fragment_node(frag),
            VNode::Component(component) => self.create_component_node(*component),
        }
    }

    fn create_text_node(&mut self, text: &'b VText<'b>, node: &'b VNode<'b>) -> usize {
        let real_id = self.scopes.reserve_node(node);
        text.id.set(Some(real_id));
        self.mutations.create_text_node(text.text, real_id);
        1
    }

    fn create_anchor_node(&mut self, anchor: &'b VPlaceholder, node: &'b VNode<'b>) -> usize {
        let real_id = self.scopes.reserve_node(node);
        anchor.id.set(Some(real_id));
        self.mutations.create_placeholder(real_id);
        1
    }

    fn create_element_node(&mut self, element: &'b VElement<'b>, node: &'b VNode<'b>) -> usize {
        let VElement {
            tag: tag_name,
            listeners,
            attributes,
            children,
            namespace,
            id: dom_id,
            parent: parent_id,
            ..
        } = &element;

        parent_id.set(self.element_stack.last().copied());

        let real_id = self.scopes.reserve_node(node);

        dom_id.set(Some(real_id));

        self.element_stack.push(real_id);
        {
            self.mutations.create_element(tag_name, *namespace, real_id);

            let cur_scope_id = self.current_scope();

            for listener in listeners.iter() {
                listener.mounted_node.set(Some(real_id));
                self.mutations.new_event_listener(listener, cur_scope_id);
            }

            for attr in attributes.iter() {
                self.mutations.set_attribute(attr, real_id.as_u64());
            }

            if !children.is_empty() {
                self.create_and_append_children(children);
            }
        }
        self.element_stack.pop();

        1
    }

    fn create_fragment_node(&mut self, frag: &'b VFragment<'b>) -> usize {
        self.create_children(frag.children)
    }

    fn create_component_node(&mut self, vcomponent: &'b VComponent<'b>) -> usize {
        let parent_idx = self.current_scope();

        // the component might already exist - if it does, we need to reuse it
        // this makes figure out when to drop the component more complicated
        let new_idx = if let Some(idx) = vcomponent.scope.get() {
            assert!(self.scopes.get_scope(idx).is_some());
            idx
        } else {
            // Insert a new scope into our component list
            let props: Box<dyn AnyProps + 'b> = vcomponent.props.borrow_mut().take().unwrap();
            let props: Box<dyn AnyProps + 'static> = unsafe { std::mem::transmute(props) };
            self.scopes.new_with_key(
                vcomponent.user_fc,
                props,
                Some(parent_idx),
                self.element_stack.last().copied().unwrap(),
                0,
            )
        };

        // Actually initialize the caller's slot with the right address
        vcomponent.scope.set(Some(new_idx));

        log::trace!(
            "created component \"{}\", id: {:?} parent {:?}",
            vcomponent.fn_name,
            new_idx,
            parent_idx,
        );

        // if vcomponent.can_memoize {
        //     // todo: implement promotion logic. save us from boxing props that we don't need
        // } else {
        //     // track this component internally so we know the right drop order
        // }

        self.enter_scope(new_idx);

        let created = {
            // Run the scope for one iteration to initialize it
            self.scopes.run_scope(new_idx);
            self.mutations.mark_dirty_scope(new_idx);

            // Take the node that was just generated from running the component
            let nextnode = self.scopes.fin_head(new_idx);
            self.create_node(nextnode)
        };

        self.leave_scope();

        created
    }

    pub(crate) fn diff_text_nodes(
        &mut self,
        old: &'b VText<'b>,
        new: &'b VText<'b>,
        _old_node: &'b VNode<'b>,
        new_node: &'b VNode<'b>,
    ) {
        if std::ptr::eq(old, new) {
            return;
        }

        // if the node is comming back not assigned, that means it was borrowed but removed
        let root = match old.id.get() {
            Some(id) => id,
            None => self.scopes.reserve_node(new_node),
        };

        if old.text != new.text {
            self.mutations.set_text(new.text, root.as_u64());
        }

        self.scopes.update_node(new_node, root);

        new.id.set(Some(root));
    }

    pub(crate) fn diff_placeholder_nodes(
        &mut self,
        old: &'b VPlaceholder,
        new: &'b VPlaceholder,
        _old_node: &'b VNode<'b>,
        new_node: &'b VNode<'b>,
    ) {
        if std::ptr::eq(old, new) {
            return;
        }

        // if the node is comming back not assigned, that means it was borrowed but removed
        let root = match old.id.get() {
            Some(id) => id,
            None => self.scopes.reserve_node(new_node),
        };

        self.scopes.update_node(new_node, root);
        new.id.set(Some(root));
    }

    fn diff_element_nodes(
        &mut self,
        old: &'b VElement<'b>,
        new: &'b VElement<'b>,
        old_node: &'b VNode<'b>,
        new_node: &'b VNode<'b>,
    ) {
        if std::ptr::eq(old, new) {
            return;
        }

        // if the node is comming back not assigned, that means it was borrowed but removed
        let root = match old.id.get() {
            Some(id) => id,
            None => self.scopes.reserve_node(new_node),
        };

        // If the element type is completely different, the element needs to be re-rendered completely
        // This is an optimization React makes due to how users structure their code
        //
        // This case is rather rare (typically only in non-keyed lists)
        if new.tag != old.tag || new.namespace != old.namespace {
            self.replace_node(old_node, new_node);
            return;
        }

        self.scopes.update_node(new_node, root);

        new.id.set(Some(root));
        new.parent.set(old.parent.get());

        // todo: attributes currently rely on the element on top of the stack, but in theory, we only need the id of the
        // element to modify its attributes.
        // it would result in fewer instructions if we just set the id directly.
        // it would also clean up this code some, but that's not very important anyways

        // Diff Attributes
        //
        // It's extraordinarily rare to have the number/order of attributes change
        // In these cases, we just completely erase the old set and make a new set
        //
        // TODO: take a more efficient path than this
        if old.attributes.len() == new.attributes.len() {
            for (old_attr, new_attr) in old.attributes.iter().zip(new.attributes.iter()) {
                if old_attr.value != new_attr.value || new_attr.is_volatile {
                    self.mutations.set_attribute(new_attr, root.as_u64());
                }
            }
        } else {
            for attribute in old.attributes {
                self.mutations.remove_attribute(attribute, root.as_u64());
            }
            for attribute in new.attributes {
                self.mutations.set_attribute(attribute, root.as_u64());
            }
        }

        // Diff listeners
        //
        // It's extraordinarily rare to have the number/order of listeners change
        // In the cases where the listeners change, we completely wipe the data attributes and add new ones
        //
        // We also need to make sure that all listeners are properly attached to the parent scope (fix_listener)
        //
        // TODO: take a more efficient path than this
        let cur_scope_id = self.current_scope();

        if old.listeners.len() == new.listeners.len() {
            for (old_l, new_l) in old.listeners.iter().zip(new.listeners.iter()) {
                new_l.mounted_node.set(old_l.mounted_node.get());
                if old_l.event != new_l.event {
                    self.mutations
                        .remove_event_listener(old_l.event, root.as_u64());
                    self.mutations.new_event_listener(new_l, cur_scope_id);
                }
            }
        } else {
            for listener in old.listeners {
                self.mutations
                    .remove_event_listener(listener.event, root.as_u64());
            }
            for listener in new.listeners {
                listener.mounted_node.set(Some(root));
                self.mutations.new_event_listener(listener, cur_scope_id);
            }
        }

        match (old.children.len(), new.children.len()) {
            (0, 0) => {}
            (0, _) => {
                self.mutations.push_root(root);
                let created = self.create_children(new.children);
                self.mutations.append_children(created as u32);
                self.mutations.pop_root();
            }
            (_, _) => self.diff_children(old.children, new.children),
        };
    }

    fn diff_component_nodes(
        &mut self,
        old_node: &'b VNode<'b>,
        new_node: &'b VNode<'b>,
        old: &'b VComponent<'b>,
        new: &'b VComponent<'b>,
    ) {
        let scope_addr = old
            .scope
            .get()
            .expect("existing component nodes should have a scope");

        if std::ptr::eq(old, new) {
            return;
        }

        // Make sure we're dealing with the same component (by function pointer)
        if old.user_fc == new.user_fc {
            self.enter_scope(scope_addr);
            {
                // Make sure the new component vnode is referencing the right scope id
                new.scope.set(Some(scope_addr));

                // make sure the component's caller function is up to date
                let scope = self
                    .scopes
                    .get_scope(scope_addr)
                    .unwrap_or_else(|| panic!("could not find {:?}", scope_addr));

                // take the new props out regardless
                // when memoizing, push to the existing scope if memoization happens
                let new_props = new
                    .props
                    .borrow_mut()
                    .take()
                    .expect("new component props should exist");

                let should_diff = {
                    if old.can_memoize {
                        // safety: we trust the implementation of "memoize"
                        let props_are_the_same = unsafe {
                            let new_ref = new_props.as_ref();
                            scope.props.borrow().as_ref().unwrap().memoize(new_ref)
                        };
                        !props_are_the_same || self.force_diff
                    } else {
                        true
                    }
                };

                if should_diff {
                    let _old_props = scope
                        .props
                        .replace(unsafe { std::mem::transmute(Some(new_props)) });

                    // this should auto drop the previous props
                    self.scopes.run_scope(scope_addr);
                    self.mutations.mark_dirty_scope(scope_addr);

                    self.diff_node(
                        self.scopes.wip_head(scope_addr),
                        self.scopes.fin_head(scope_addr),
                    );
                } else {
                    // memoization has taken place
                    drop(new_props);
                };
            }
            self.leave_scope();
        } else {
            self.replace_node(old_node, new_node);
        }
    }

    fn diff_fragment_nodes(&mut self, old: &'b VFragment<'b>, new: &'b VFragment<'b>) {
        if std::ptr::eq(old, new) {
            return;
        }

        // This is the case where options or direct vnodes might be used.
        // In this case, it's faster to just skip ahead to their diff
        if old.children.len() == 1 && new.children.len() == 1 {
            if !std::ptr::eq(old, new) {
                self.diff_node(&old.children[0], &new.children[0]);
            }
            return;
        }

        debug_assert!(!old.children.is_empty());
        debug_assert!(!new.children.is_empty());

        self.diff_children(old.children, new.children);
    }

    // Diff the given set of old and new children.
    //
    // The parent must be on top of the change list stack when this function is
    // entered:
    //
    //     [... parent]
    //
    // the change list stack is in the same state when this function returns.
    //
    // If old no anchors are provided, then it's assumed that we can freely append to the parent.
    //
    // Remember, non-empty lists does not mean that there are real elements, just that there are virtual elements.
    //
    // Fragment nodes cannot generate empty children lists, so we can assume that when a list is empty, it belongs only
    // to an element, and appending makes sense.
    fn diff_children(&mut self, old: &'b [VNode<'b>], new: &'b [VNode<'b>]) {
        if std::ptr::eq(old, new) {
            return;
        }

        // Remember, fragments can never be empty (they always have a single child)
        match (old, new) {
            ([], []) => {}
            ([], _) => self.create_and_append_children(new),
            (_, []) => self.remove_nodes(old, true),
            _ => {
                let new_is_keyed = new[0].key().is_some();
                let old_is_keyed = old[0].key().is_some();

                debug_assert!(
                    new.iter().all(|n| n.key().is_some() == new_is_keyed),
                    "all siblings must be keyed or all siblings must be non-keyed"
                );
                debug_assert!(
                    old.iter().all(|o| o.key().is_some() == old_is_keyed),
                    "all siblings must be keyed or all siblings must be non-keyed"
                );

                if new_is_keyed && old_is_keyed {
                    self.diff_keyed_children(old, new);
                } else {
                    self.diff_non_keyed_children(old, new);
                }
            }
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
    fn diff_non_keyed_children(&mut self, old: &'b [VNode<'b>], new: &'b [VNode<'b>]) {
        use std::cmp::Ordering;

        // Handled these cases in `diff_children` before calling this function.
        debug_assert!(!new.is_empty());
        debug_assert!(!old.is_empty());

        match old.len().cmp(&new.len()) {
            Ordering::Greater => self.remove_nodes(&old[new.len()..], true),
            Ordering::Less => self.create_and_insert_after(&new[old.len()..], old.last().unwrap()),
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
    fn diff_keyed_children(&mut self, old: &'b [VNode<'b>], new: &'b [VNode<'b>]) {
        if cfg!(debug_assertions) {
            let mut keys = fxhash::FxHashSet::default();
            let mut assert_unique_keys = |children: &'b [VNode<'b>]| {
                keys.clear();
                for child in children {
                    let key = child.key();
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
        let (left_offset, right_offset) = match self.diff_keyed_ends(old, new) {
            Some(count) => count,
            None => return,
        };

        // Ok, we now hopefully have a smaller range of children in the middle
        // within which to re-order nodes with the same keys, remove old nodes with
        // now-unused keys, and create new nodes with fresh keys.

        let old_middle = &old[left_offset..(old.len() - right_offset)];
        let new_middle = &new[left_offset..(new.len() - right_offset)];

        debug_assert!(
            !((old_middle.len() == new_middle.len()) && old_middle.is_empty()),
            "keyed children must have the same number of children"
        );

        if new_middle.is_empty() {
            // remove the old elements
            self.remove_nodes(old_middle, true);
        } else if old_middle.is_empty() {
            // there were no old elements, so just create the new elements
            // we need to find the right "foothold" though - we shouldn't use the "append" at all
            if left_offset == 0 {
                // insert at the beginning of the old list
                let foothold = &old[old.len() - right_offset];
                self.create_and_insert_before(new_middle, foothold);
            } else if right_offset == 0 {
                // insert at the end  the old list
                let foothold = old.last().unwrap();
                self.create_and_insert_after(new_middle, foothold);
            } else {
                // inserting in the middle
                let foothold = &old[left_offset - 1];
                self.create_and_insert_after(new_middle, foothold);
            }
        } else {
            self.diff_keyed_middle(old_middle, new_middle);
        }
    }

    /// Diff both ends of the children that share keys.
    ///
    /// Returns a left offset and right offset of that indicates a smaller section to pass onto the middle diffing.
    ///
    /// If there is no offset, then this function returns None and the diffing is complete.
    fn diff_keyed_ends(
        &mut self,
        old: &'b [VNode<'b>],
        new: &'b [VNode<'b>],
    ) -> Option<(usize, usize)> {
        let mut left_offset = 0;

        for (old, new) in old.iter().zip(new.iter()) {
            // abort early if we finally run into nodes with different keys
            if old.key() != new.key() {
                break;
            }
            self.diff_node(old, new);
            left_offset += 1;
        }

        // If that was all of the old children, then create and append the remaining
        // new children and we're finished.
        if left_offset == old.len() {
            self.create_and_insert_after(&new[left_offset..], old.last().unwrap());
            return None;
        }

        // And if that was all of the new children, then remove all of the remaining
        // old children and we're finished.
        if left_offset == new.len() {
            self.remove_nodes(&old[left_offset..], true);
            return None;
        }

        // if the shared prefix is less than either length, then we need to walk backwards
        let mut right_offset = 0;
        for (old, new) in old.iter().rev().zip(new.iter().rev()) {
            // abort early if we finally run into nodes with different keys
            if old.key() != new.key() {
                break;
            }
            self.diff_node(old, new);
            right_offset += 1;
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
    fn diff_keyed_middle(&mut self, old: &'b [VNode<'b>], new: &'b [VNode<'b>]) {
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
        debug_assert_ne!(new.first().map(VNode::key), old.first().map(VNode::key));
        debug_assert_ne!(new.last().map(VNode::key), old.last().map(VNode::key));

        // 1. Map the old keys into a numerical ordering based on indices.
        // 2. Create a map of old key to its index
        // IE if the keys were A B C, then we would have (A, 1) (B, 2) (C, 3).
        let old_key_to_old_index = old
            .iter()
            .enumerate()
            .map(|(i, o)| (o.key().unwrap(), i))
            .collect::<FxHashMap<_, _>>();

        let mut shared_keys = FxHashSet::default();

        // 3. Map each new key to the old key, carrying over the old index.
        let new_index_to_old_index = new
            .iter()
            .map(|node| {
                let key = node.key().unwrap();
                if let Some(&index) = old_key_to_old_index.get(&key) {
                    shared_keys.insert(key);
                    index
                } else {
                    u32::MAX as usize
                }
            })
            .collect::<Vec<_>>();

        // If none of the old keys are reused by the new children, then we remove all the remaining old children and
        // create the new children afresh.
        if shared_keys.is_empty() {
            if let Some(first_old) = old.get(0) {
                self.remove_nodes(&old[1..], true);
                let nodes_created = self.create_children(new);
                self.replace_inner(first_old, nodes_created);
            } else {
                // I think this is wrong - why are we appending?
                // only valid of the if there are no trailing elements
                self.create_and_append_children(new);
            }
            return;
        }

        // remove any old children that are not shared
        // todo: make this an iterator
        for child in old {
            let key = child.key().unwrap();
            if !shared_keys.contains(&key) {
                self.remove_nodes([child], true);
            }
        }

        // 4. Compute the LIS of this list
        let mut lis_sequence = Vec::default();
        lis_sequence.reserve(new_index_to_old_index.len());

        let mut predecessors = vec![0; new_index_to_old_index.len()];
        let mut starts = vec![0; new_index_to_old_index.len()];

        longest_increasing_subsequence::lis_with(
            &new_index_to_old_index,
            &mut lis_sequence,
            |a, b| a < b,
            &mut predecessors,
            &mut starts,
        );

        // the lis comes out backwards, I think. can't quite tell.
        lis_sequence.sort_unstable();

        // if a new node gets u32 max and is at the end, then it might be part of our LIS (because u32 max is a valid LIS)
        if lis_sequence.last().map(|f| new_index_to_old_index[*f]) == Some(u32::MAX as usize) {
            lis_sequence.pop();
        }

        for idx in &lis_sequence {
            self.diff_node(&old[new_index_to_old_index[*idx]], &new[*idx]);
        }

        let mut nodes_created = 0;

        // add mount instruction for the first items not covered by the lis
        let last = *lis_sequence.last().unwrap();
        if last < (new.len() - 1) {
            for (idx, new_node) in new[(last + 1)..].iter().enumerate() {
                let new_idx = idx + last + 1;
                let old_index = new_index_to_old_index[new_idx];
                if old_index == u32::MAX as usize {
                    nodes_created += self.create_node(new_node);
                } else {
                    self.diff_node(&old[old_index], new_node);
                    nodes_created += self.push_all_real_nodes(new_node);
                }
            }

            self.mutations.insert_after(
                self.find_last_element(&new[last]).unwrap(),
                nodes_created as u32,
            );
            nodes_created = 0;
        }

        // for each spacing, generate a mount instruction
        let mut lis_iter = lis_sequence.iter().rev();
        let mut last = *lis_iter.next().unwrap();
        for next in lis_iter {
            if last - next > 1 {
                for (idx, new_node) in new[(next + 1)..last].iter().enumerate() {
                    let new_idx = idx + next + 1;
                    let old_index = new_index_to_old_index[new_idx];
                    if old_index == u32::MAX as usize {
                        nodes_created += self.create_node(new_node);
                    } else {
                        self.diff_node(&old[old_index], new_node);
                        nodes_created += self.push_all_real_nodes(new_node);
                    }
                }

                self.mutations.insert_before(
                    self.find_first_element(&new[last]).unwrap(),
                    nodes_created as u32,
                );

                nodes_created = 0;
            }
            last = *next;
        }

        // add mount instruction for the last items not covered by the lis
        let first_lis = *lis_sequence.first().unwrap();
        if first_lis > 0 {
            for (idx, new_node) in new[..first_lis].iter().enumerate() {
                let old_index = new_index_to_old_index[idx];
                if old_index == u32::MAX as usize {
                    nodes_created += self.create_node(new_node);
                } else {
                    self.diff_node(&old[old_index], new_node);
                    nodes_created += self.push_all_real_nodes(new_node);
                }
            }

            self.mutations.insert_before(
                self.find_first_element(&new[first_lis]).unwrap(),
                nodes_created as u32,
            );
        }
    }

    fn replace_node(&mut self, old: &'b VNode<'b>, new: &'b VNode<'b>) {
        let nodes_created = self.create_node(new);
        self.replace_inner(old, nodes_created);
    }

    fn replace_inner(&mut self, old: &'b VNode<'b>, nodes_created: usize) {
        match old {
            VNode::Element(el) => {
                let id = old
                    .try_mounted_id()
                    .unwrap_or_else(|| panic!("broke on {:?}", old));

                self.mutations.replace_with(id, nodes_created as u32);
                self.remove_nodes(el.children, false);
                self.scopes.collect_garbage(id);
            }

            VNode::Text(_) | VNode::Placeholder(_) => {
                let id = old
                    .try_mounted_id()
                    .unwrap_or_else(|| panic!("broke on {:?}", old));

                self.mutations.replace_with(id, nodes_created as u32);
                self.scopes.collect_garbage(id);
            }

            VNode::Fragment(f) => {
                self.replace_inner(&f.children[0], nodes_created);
                self.remove_nodes(f.children.iter().skip(1), true);
            }

            VNode::Component(c) => {
                log::trace!("Replacing component {:?}", old);
                let scope_id = c.scope.get().unwrap();
                let node = self.scopes.fin_head(scope_id);

                self.enter_scope(scope_id);
                {
                    self.replace_inner(node, nodes_created);

                    log::trace!("Replacing component x2 {:?}", old);

                    let scope = self.scopes.get_scope(scope_id).unwrap();
                    c.scope.set(None);
                    let props = scope.props.take().unwrap();
                    c.props.borrow_mut().replace(props);
                    self.scopes.try_remove(scope_id).unwrap();
                }
                self.leave_scope();
            }
        }
    }

    pub fn remove_nodes(&mut self, nodes: impl IntoIterator<Item = &'b VNode<'b>>, gen_muts: bool) {
        for node in nodes {
            match node {
                VNode::Text(t) => {
                    // this check exists because our null node will be removed but does not have an ID
                    if let Some(id) = t.id.get() {
                        self.scopes.collect_garbage(id);
                        t.id.set(None);

                        if gen_muts {
                            self.mutations.remove(id.as_u64());
                        }
                    }
                }
                VNode::Placeholder(a) => {
                    let id = a.id.get().unwrap();
                    self.scopes.collect_garbage(id);
                    a.id.set(None);

                    if gen_muts {
                        self.mutations.remove(id.as_u64());
                    }
                }
                VNode::Element(e) => {
                    let id = e.id.get().unwrap();

                    if gen_muts {
                        self.mutations.remove(id.as_u64());
                    }

                    self.scopes.collect_garbage(id);
                    e.id.set(None);

                    self.remove_nodes(e.children, false);
                }

                VNode::Fragment(f) => {
                    self.remove_nodes(f.children, gen_muts);
                }

                VNode::Component(c) => {
                    self.enter_scope(c.scope.get().unwrap());
                    {
                        let scope_id = c.scope.get().unwrap();
                        let root = self.scopes.root_node(scope_id);
                        self.remove_nodes([root], gen_muts);

                        let scope = self.scopes.get_scope(scope_id).unwrap();
                        c.scope.set(None);

                        let props = scope.props.take().unwrap();
                        c.props.borrow_mut().replace(props);
                        self.scopes.try_remove(scope_id).unwrap();
                    }
                    self.leave_scope();
                }
            }
        }
    }

    fn create_children(&mut self, nodes: &'b [VNode<'b>]) -> usize {
        let mut created = 0;
        for node in nodes {
            created += self.create_node(node);
        }
        created
    }

    fn create_and_append_children(&mut self, nodes: &'b [VNode<'b>]) {
        let created = self.create_children(nodes);
        self.mutations.append_children(created as u32);
    }

    fn create_and_insert_after(&mut self, nodes: &'b [VNode<'b>], after: &'b VNode<'b>) {
        let created = self.create_children(nodes);
        let last = self.find_last_element(after).unwrap();
        self.mutations.insert_after(last, created as u32);
    }

    fn create_and_insert_before(&mut self, nodes: &'b [VNode<'b>], before: &'b VNode<'b>) {
        let created = self.create_children(nodes);
        let first = self.find_first_element(before).unwrap();
        self.mutations.insert_before(first, created as u32);
    }

    fn current_scope(&self) -> ScopeId {
        self.scope_stack.last().copied().expect("no current scope")
    }

    fn enter_scope(&mut self, scope: ScopeId) {
        self.scope_stack.push(scope);
    }

    fn leave_scope(&mut self) {
        self.scope_stack.pop();
    }

    fn find_last_element(&self, vnode: &'b VNode<'b>) -> Option<ElementId> {
        let mut search_node = Some(vnode);
        loop {
            match &search_node.take().unwrap() {
                VNode::Text(t) => break t.id.get(),
                VNode::Element(t) => break t.id.get(),
                VNode::Placeholder(t) => break t.id.get(),
                VNode::Fragment(frag) => search_node = frag.children.last(),
                VNode::Component(el) => {
                    let scope_id = el.scope.get().unwrap();
                    search_node = Some(self.scopes.root_node(scope_id));
                }
            }
        }
    }

    fn find_first_element(&self, vnode: &'b VNode<'b>) -> Option<ElementId> {
        let mut search_node = Some(vnode);
        loop {
            match &search_node.take().expect("search node to have an ID") {
                VNode::Text(t) => break t.id.get(),
                VNode::Element(t) => break t.id.get(),
                VNode::Placeholder(t) => break t.id.get(),
                VNode::Fragment(frag) => search_node = Some(&frag.children[0]),
                VNode::Component(el) => {
                    let scope = el.scope.get().expect("element to have a scope assigned");
                    search_node = Some(self.scopes.root_node(scope));
                }
            }
        }
    }

    // recursively push all the nodes of a tree onto the stack and return how many are there
    fn push_all_real_nodes(&mut self, node: &'b VNode<'b>) -> usize {
        match node {
            VNode::Text(_) | VNode::Placeholder(_) | VNode::Element(_) => {
                self.mutations.push_root(node.mounted_id());
                1
            }

            VNode::Fragment(frag) => {
                let mut added = 0;
                for child in frag.children {
                    added += self.push_all_real_nodes(child);
                }
                added
            }

            VNode::Component(c) => {
                let scope_id = c.scope.get().unwrap();
                let root = self.scopes.root_node(scope_id);
                self.push_all_real_nodes(root)
            }
        }
    }
}
