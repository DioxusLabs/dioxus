//! This module contains the stateful DiffMachine and all methods to diff VNodes, their properties, and their children.
//!
//! The [`DiffMachine`] calculates the diffs between the old and new frames, updates the new nodes, and generates a set
//! of mutations for the RealDom to apply.
//!
//! ## Notice:
//!
//! The inspiration and code for this module was originally taken from Dodrio (@fitzgen) and then modified to support
//! Components, Fragments, Suspense, SubTree memoization, incremental diffing, cancelation, NodeRefs, and additional
//! batching operations.
//!
//! ## Implementation Details:
//!
//! ### IDs for elements
//! --------------------
//! All nodes are addressed by their IDs. The RealDom provides an imperative interface for making changes to these nodes.
//! We don't necessarily require that DOM changes happen instantly during the diffing process, so the implementor may choose
//! to batch nodes if it is more performant for their application. The element IDs are indicies into the internal element
//! array. The expectation is that implemenetors will use the ID as an index into a Vec of real nodes, allowing for passive
//! garbage collection as the VirtualDOM replaces old nodes.
//!
//! When new vnodes are created through `cx.render`, they won't know which real node they correspond to. During diffing,
//! we always make sure to copy over the ID. If we don't do this properly, the ElementId will be populated incorrectly
//! and brick the user's page.
//!
//! ### Fragment Support
//! --------------------
//! Fragments (nodes without a parent) are supported through a combination of "replace with" and anchor vnodes. Fragments
//! can be particularly challenging when they are empty, so the anchor node lets us "reserve" a spot for the empty
//! fragment to be replaced with when it is no longer empty. This is guaranteed by logic in the NodeFactory - it is
//! impossible to craft a fragment with 0 elements - they must always have at least a single placeholder element. Adding
//! "dummy" nodes _is_ inefficient, but it makes our diffing algorithm faster and the implementation is completely up to
//!  the platform.
//!
//! Other implementations either don't support fragments or use a "child + sibling" pattern to represent them. Our code is
//! vastly simpler and more performant when we can just create a placeholder element while the fragment has no children.
//!
//! ### Suspense
//! ------------
//! Dioxus implements suspense slightly differently than React. In React, each fiber is manually progressed until it runs
//! into a promise-like value. React will then work on the next "ready" fiber, checking back on the previous fiber once
//! it has finished its new work. In Dioxus, we use a similar approach, but try to completely render the tree before
//! switching sub-fibers. Instead, each future is submitted into a futures-queue and the node is manually loaded later on.
//!
//! We're able to use this approach because we use placeholder nodes - futures that aren't ready still get submitted to
//! DOM, but as a placeholder.
//!
//! Right now, the "suspense" queue is intertwined the hooks. In the future, we should allow any future to drive attributes
//! and contents, without the need for the "use_suspense" hook. For now, this is the quickest way to get suspense working.
//!
//! ## Subtree Memoization
//! -----------------------
//! We also employ "subtree memoization" which saves us from having to check trees which take no dynamic content. We can
//! detect if a subtree is "static" by checking if its children are "static". Since we dive into the tree depth-first, the
//! calls to "create" propogate this information upwards. Structures like the one below are entirely static:
//! ```rust
//! rsx!( div { class: "hello world", "this node is entirely static" } )
//! ```
//! Because the subtrees won't be diffed, their "real node" data will be stale (invalid), so its up to the reconciler to
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
//! Dioxus uses a passive garbage collection system to clean up old nodes once the work has been completed. This garabge
//! collection is done internally once the main diffing work is complete. After the "garbage" is collected, Dioxus will then
//! start to re-use old keys for new nodes. This results in a passive memory management system that is very efficient.
//!
//! The IDs used by the key/map are just an index into a vec. This means that Dioxus will drive the key allocation strategy
//! so the client only needs to maintain a simple list of nodes. By default, Dioxus will not manually clean up old nodes
//! for the client. As new nodes are created, old nodes will be over-written.
//!
//! ## Further Reading and Thoughts
//! ----------------------------
//! There are more ways of increasing diff performance here that are currently not implemented.
//! More info on how to improve this diffing algorithm:
//!  - https://hacks.mozilla.org/2019/03/fast-bump-allocated-virtual-doms-with-rust-and-wasm/

use crate::{arena::SharedResources, innerlude::*};
use futures_util::{Future, FutureExt};
use fxhash::{FxBuildHasher, FxHashMap, FxHashSet};
use smallvec::{smallvec, SmallVec};

use std::{
    any::Any, cell::Cell, cmp::Ordering, collections::HashSet, marker::PhantomData, pin::Pin,
};
use DomEdit::*;

/// Our DiffMachine is an iterative tree differ.
///
/// It uses techniques of a stack machine to allow pausing and restarting of the diff algorithm. This
/// was origially implemented using recursive techniques, but Rust lacks the abilty to call async functions recursively,
/// meaning we could not "pause" the original diffing algorithm.
///
/// Instead, we use a traditional stack machine approach to diff and create new nodes. The diff algorithm periodically
/// calls "yield_now" which allows the machine to pause and return control to the caller. The caller can then wait for
/// the next period of idle time, preventing our diff algorithm from blocking the main thread.
///
/// Funnily enough, this stack machine's entire job is to create instructions for another stack machine to execute. It's
/// stack machines all the way down!
pub struct DiffMachine<'bump> {
    vdom: &'bump SharedResources,

    pub mutations: Mutations<'bump>,

    pub stack: DiffStack<'bump>,

    pub diffed: FxHashSet<ScopeId>,

    pub seen_scopes: FxHashSet<ScopeId>,
}

impl<'bump> DiffMachine<'bump> {
    pub(crate) fn new(
        edits: Mutations<'bump>,
        cur_scope: ScopeId,
        shared: &'bump SharedResources,
    ) -> Self {
        Self {
            stack: DiffStack::new(cur_scope),
            mutations: edits,
            vdom: shared,
            diffed: FxHashSet::default(),
            seen_scopes: FxHashSet::default(),
        }
    }

    pub fn new_headless(shared: &'bump SharedResources) -> Self {
        let edits = Mutations::new();
        let cur_scope = ScopeId(0);
        Self::new(edits, cur_scope, shared)
    }

    //
    pub async fn diff_scope(&mut self, id: ScopeId) -> Result<()> {
        let component = self
            .vdom
            .get_scope_mut(id)
            .ok_or_else(|| Error::NotMounted)?;
        let (old, new) = (component.frames.wip_head(), component.frames.fin_head());
        self.diff_node(old, new);
        Ok(())
    }

    /// Progress the diffing for this "fiber"
    ///
    /// This method implements a depth-first iterative tree traversal.
    ///
    /// We do depth-first to maintain high cache locality (nodes were originally generated recursively).
    pub async fn work(&mut self) -> Result<()> {
        // defer to individual functions so the compiler produces better code
        // large functions tend to be difficult for the compiler to work with
        while let Some(instruction) = self.stack.pop() {
            log::debug!("Handling diff instruction: {:?}", instruction);

            // todo: call this less frequently, there is a bit of overhead involved
            yield_now().await;

            match instruction {
                DiffInstruction::PopScope => {
                    self.stack.pop_scope();
                }
                DiffInstruction::PopElement => {
                    self.mutations.pop();
                }

                DiffInstruction::DiffNode { old, new, .. } => {
                    self.diff_node(old, new);
                }

                DiffInstruction::DiffChildren { old, new } => {
                    self.diff_children(old, new);
                }

                DiffInstruction::Create { node, and } => {
                    self.create_node(node);
                }

                DiffInstruction::Remove { child } => {
                    for child in RealChildIterator::new(child, self.vdom) {
                        self.mutations.remove(child.direct_id().as_u64());
                    }
                }

                DiffInstruction::RemoveChildren { children } => {
                    for child in RealChildIterator::new_from_slice(children, self.vdom) {
                        self.mutations.remove(child.direct_id().as_u64());
                    }
                }

                DiffInstruction::Mount { and } => {
                    self.mount(and);
                }
            };
        }

        Ok(())
    }

    fn mount(&mut self, and: MountType) {
        let nodes_created = self.stack.nodes_created_stack.pop().unwrap();
        match and {
            // add the nodes from this virtual list to the parent
            // used by fragments and components
            MountType::Absorb => {
                *self.stack.nodes_created_stack.last_mut().unwrap() += nodes_created;
            }
            MountType::Append => {
                self.mutations.edits.push(AppendChildren {
                    many: nodes_created as u32,
                });
            }
            MountType::Replace { old } => {
                todo!()
                // self.mutations.replace_with(with as u32, many as u32);
            }
            MountType::InsertAfter { other_node } => {
                self.mutations.insert_after(nodes_created as u32);
            }

            MountType::InsertBefore { other_node } => {
                self.mutations.insert_before(nodes_created as u32);
            }
        }
    }

    // =================================
    //  Tools for creating new nodes
    // =================================

    fn create_node(&mut self, node: &'bump VNode<'bump>) {
        match node {
            VNode::Text(vtext) => self.create_text_node(vtext),
            VNode::Suspended(suspended) => self.create_suspended_node(suspended),
            VNode::Anchor(anchor) => self.create_anchor_node(anchor),
            VNode::Element(element) => self.create_element_node(element),
            VNode::Fragment(frag) => self.create_fragment_node(frag),
            VNode::Component(component) => self.create_component_node(component),
        }
    }

    fn create_text_node(&mut self, vtext: &'bump VText<'bump>) {
        let real_id = self.vdom.reserve_node();
        self.mutations.create_text_node(vtext.text, real_id);
        vtext.dom_id.set(Some(real_id));
        self.stack.add_child_count(1);
    }

    fn create_suspended_node(&mut self, suspended: &'bump VSuspended) {
        let real_id = self.vdom.reserve_node();
        self.mutations.create_placeholder(real_id);
        suspended.node.set(Some(real_id));
        self.stack.add_child_count(1);
    }

    fn create_anchor_node(&mut self, anchor: &'bump VAnchor) {
        let real_id = self.vdom.reserve_node();
        self.mutations.create_placeholder(real_id);
        anchor.dom_id.set(Some(real_id));
        self.stack.add_child_count(1);
    }

    fn create_element_node(&mut self, element: &'bump VElement<'bump>) {
        let VElement {
            tag_name,
            listeners,
            attributes,
            children,
            namespace,
            dom_id,
            ..
        } = element;

        let real_id = self.vdom.reserve_node();
        self.mutations.create_element(tag_name, *namespace, real_id);

        self.stack.add_child_count(1);

        dom_id.set(Some(real_id));

        let cur_scope = self.stack.current_scope().unwrap();

        listeners.iter().for_each(|listener| {
            self.fix_listener(listener);
            listener.mounted_node.set(Some(real_id));
            self.mutations
                .new_event_listener(listener, cur_scope.clone());
        });

        for attr in *attributes {
            self.mutations.set_attribute(attr);
        }

        if children.len() > 0 {
            self.stack.create_children(children, MountType::Append);
        }
    }

    fn create_fragment_node(&mut self, frag: &'bump VFragment<'bump>) {
        self.stack.create_children(frag.children, MountType::Absorb);
    }

    fn create_component_node(&mut self, vcomponent: &'bump VComponent<'bump>) {
        let caller = vcomponent.caller.clone();

        let parent_idx = self.stack.current_scope().unwrap();

        // Insert a new scope into our component list
        let new_idx = self.vdom.insert_scope_with_key(|new_idx| {
            let parent_scope = self.vdom.get_scope(parent_idx).unwrap();
            let height = parent_scope.height + 1;
            Scope::new(
                caller,
                new_idx,
                Some(parent_idx),
                height,
                ScopeChildren(vcomponent.children),
                self.vdom.clone(),
                vcomponent.name,
            )
        });

        // Actually initialize the caller's slot with the right address
        vcomponent.ass_scope.set(Some(new_idx));

        if !vcomponent.can_memoize {
            let cur_scope = self.vdom.get_scope_mut(parent_idx).unwrap();
            let extended = vcomponent as *const VComponent;
            let extended: *const VComponent<'static> = unsafe { std::mem::transmute(extended) };
            cur_scope.borrowed_props.borrow_mut().push(extended);
        }

        // TODO:
        //  add noderefs to current noderef list Noderefs
        //  add effects to current effect list Effects

        let new_component = self.vdom.get_scope_mut(new_idx).unwrap();

        // Run the scope for one iteration to initialize it
        match new_component.run_scope() {
            Ok(_g) => {
                // all good, new nodes exist
            }
            Err(err) => {
                // failed to run. this is the first time the component ran, and it failed
                // we manually set its head node to an empty fragment
                panic!("failing components not yet implemented");
            }
        }

        // Take the node that was just generated from running the component
        let nextnode = new_component.frames.fin_head();

        self.stack.create_component(new_idx, nextnode);

        // Finally, insert this scope as a seen node.
        self.seen_scopes.insert(new_idx);
    }

    // =================================
    //  Tools for diffing nodes
    // =================================

    pub fn diff_node(&mut self, old_node: &'bump VNode<'bump>, new_node: &'bump VNode<'bump>) {
        use VNode::*;
        match (old_node, new_node) {
            // Check the most common cases first
            (Text(old), Text(new)) => self.diff_text_nodes(old, new),
            (Element(old), Element(new)) => self.diff_element_nodes(old, new),
            (Component(old), Component(new)) => self.diff_component_nodes(old, new),
            (Fragment(old), Fragment(new)) => self.diff_fragment_nodes(old, new),
            (Anchor(old), Anchor(new)) => new.dom_id.set(old.dom_id.get()),
            (Suspended(old), Suspended(new)) => new.node.set(old.node.get()),

            // Anything else is just a basic replace and create
            (
                Component(_) | Fragment(_) | Text(_) | Element(_) | Anchor(_) | Suspended(_),
                Component(_) | Fragment(_) | Text(_) | Element(_) | Anchor(_) | Suspended(_),
            ) => self.replace_and_create_one_with_one(old_node, new_node),
        }
    }

    fn diff_text_nodes(&mut self, old: &'bump VText<'bump>, new: &'bump VText<'bump>) {
        let root = old.dom_id.get().unwrap();

        if old.text != new.text {
            self.mutations.push_root(root);
            self.mutations.set_text(new.text);
            self.mutations.pop();
        }

        new.dom_id.set(Some(root));
    }

    fn diff_element_nodes(&mut self, old: &'bump VElement<'bump>, new: &'bump VElement<'bump>) {
        let root = old.dom_id.get().unwrap();

        // If the element type is completely different, the element needs to be re-rendered completely
        // This is an optimization React makes due to how users structure their code
        //
        // This case is rather rare (typically only in non-keyed lists)
        if new.tag_name != old.tag_name || new.namespace != old.namespace {
            todo!();
            // self.replace_node_with_node(root, old_node, new_node);
            return;
        }

        new.dom_id.set(Some(root));

        // Don't push the root if we don't have to
        let mut has_comitted = false;
        let mut please_commit = |edits: &mut Vec<DomEdit>| {
            if !has_comitted {
                has_comitted = true;
                edits.push(PushRoot { id: root.as_u64() });
            }
        };

        // Diff Attributes
        //
        // It's extraordinarily rare to have the number/order of attributes change
        // In these cases, we just completely erase the old set and make a new set
        //
        // TODO: take a more efficient path than this
        if old.attributes.len() == new.attributes.len() {
            for (old_attr, new_attr) in old.attributes.iter().zip(new.attributes.iter()) {
                if old_attr.value != new_attr.value {
                    please_commit(&mut self.mutations.edits);
                    self.mutations.set_attribute(new_attr);
                }
            }
        } else {
            // TODO: provide some sort of report on how "good" the diffing was
            please_commit(&mut self.mutations.edits);
            for attribute in old.attributes {
                self.mutations.remove_attribute(attribute);
            }
            for attribute in new.attributes {
                self.mutations.set_attribute(attribute)
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
        let cur_scope = self.stack.current_scope().unwrap();
        if old.listeners.len() == new.listeners.len() {
            for (old_l, new_l) in old.listeners.iter().zip(new.listeners.iter()) {
                if old_l.event != new_l.event {
                    please_commit(&mut self.mutations.edits);
                    self.mutations.remove_event_listener(old_l.event);
                    self.mutations.new_event_listener(new_l, cur_scope);
                }
                new_l.mounted_node.set(old_l.mounted_node.get());
                self.fix_listener(new_l);
            }
        } else {
            please_commit(&mut self.mutations.edits);
            for listener in old.listeners {
                self.mutations.remove_event_listener(listener.event);
            }
            for listener in new.listeners {
                listener.mounted_node.set(Some(root));
                self.mutations.new_event_listener(listener, cur_scope);

                // Make sure the listener gets attached to the scope list
                self.fix_listener(listener);
            }
        }

        if has_comitted {
            self.mutations.pop();
        }

        self.diff_children(old.children, new.children);
    }

    fn diff_component_nodes(
        &mut self,
        old: &'bump VComponent<'bump>,
        new: &'bump VComponent<'bump>,
    ) {
        let scope_addr = old.ass_scope.get().unwrap();

        // Make sure we're dealing with the same component (by function pointer)
        if old.user_fc == new.user_fc {
            //
            self.stack.scope_stack.push(scope_addr);

            // Make sure the new component vnode is referencing the right scope id
            new.ass_scope.set(Some(scope_addr));

            // make sure the component's caller function is up to date
            let scope = self.vdom.get_scope_mut(scope_addr).unwrap();

            scope.update_scope_dependencies(new.caller.clone(), ScopeChildren(new.children));

            // React doesn't automatically memoize, but we do.
            let compare = old.comparator.unwrap();

            match compare(new) {
                true => {
                    // the props are the same...
                }
                false => {
                    // the props are different...
                    scope.run_scope().unwrap();
                    self.diff_node(scope.frames.wip_head(), scope.frames.fin_head());
                }
            }

            self.stack.scope_stack.pop();

            self.seen_scopes.insert(scope_addr);
        } else {
            todo!();

            // let mut old_iter = RealChildIterator::new(old_node, &self.vdom);
            // let first = old_iter
            //     .next()
            //     .expect("Components should generate a placeholder root");

            // // remove any leftovers
            // for to_remove in old_iter {
            //     self.mutations.push_root(to_remove.direct_id());
            //     self.mutations.remove();
            // }

            // // seems like we could combine this into a single instruction....
            // self.replace_node_with_node(first.direct_id(), old_node, new_node);

            // // Wipe the old one and plant the new one
            // let old_scope = old.ass_scope.get().unwrap();
            // self.destroy_scopes(old_scope);
        }
    }

    fn diff_fragment_nodes(&mut self, old: &'bump VFragment<'bump>, new: &'bump VFragment<'bump>) {
        // This is the case where options or direct vnodes might be used.
        // In this case, it's faster to just skip ahead to their diff
        if old.children.len() == 1 && new.children.len() == 1 {
            self.diff_node(&old.children[0], &new.children[0]);
            return;
        }

        self.diff_children(old.children, new.children);
    }

    // =============================================
    //  Utilites for creating new diff instructions
    // =============================================

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
    // Frament nodes cannot generate empty children lists, so we can assume that when a list is empty, it belongs only
    // to an element, and appending makes sense.
    fn diff_children(&mut self, old: &'bump [VNode<'bump>], new: &'bump [VNode<'bump>]) {
        const IS_EMPTY: bool = true;
        const IS_NOT_EMPTY: bool = false;

        // Remember, fragments can never be empty (they always have a single child)
        match (old.is_empty(), new.is_empty()) {
            (IS_EMPTY, IS_EMPTY) => {}

            // Completely adding new nodes, removing any placeholder if it exists
            (IS_EMPTY, IS_NOT_EMPTY) => {
                self.stack.create_children(new, MountType::Append);
            }

            // Completely removing old nodes and putting an anchor in its place
            // no anchor (old has nodes) and the new is empty
            // remove all the old nodes
            (IS_NOT_EMPTY, IS_EMPTY) => {
                for node in old {
                    self.remove_nodes(Some(node));
                }
            }

            (IS_NOT_EMPTY, IS_NOT_EMPTY) => {
                let first_old = &old[0];
                let first_new = &new[0];

                match (&first_old, &first_new) {
                    // Anchors can only appear in empty fragments
                    (VNode::Anchor(old_anchor), VNode::Anchor(new_anchor)) => {
                        old_anchor.dom_id.set(new_anchor.dom_id.get());
                    }

                    // Replace the anchor with whatever new nodes are coming down the pipe
                    (VNode::Anchor(anchor), _) => {
                        self.stack
                            .create_children(new, MountType::Replace { old: first_old });
                    }

                    // Replace whatever nodes are sitting there with the anchor
                    (_, VNode::Anchor(anchor)) => {
                        self.replace_and_create_many_with_one(old, first_new);
                    }

                    // Use the complex diff algorithm to diff the nodes
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
    fn diff_keyed_children(&mut self, old: &'bump [VNode<'bump>], new: &'bump [VNode<'bump>]) {
        if cfg!(debug_assertions) {
            let mut keys = fxhash::FxHashSet::default();
            let mut assert_unique_keys = |children: &'bump [VNode<'bump>]| {
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
        //
        // TODO: just inline this
        let shared_prefix_count = match self.diff_keyed_prefix(old, new) {
            KeyedPrefixResult::Finished => return,
            KeyedPrefixResult::MoreWorkToDo(count) => count,
        };

        // Next, we find out how many of the nodes at the end of the children have
        // the same key. We do _not_ diff them yet, since we want to emit the change
        // list instructions such that they can be applied in a single pass over the
        // DOM. Instead, we just save this information for later.
        //
        // `shared_suffix_count` is the count of how many nodes at the end of `new`
        // and `old` share the same keys.
        let shared_suffix_count = old[shared_prefix_count..]
            .iter()
            .rev()
            .zip(new[shared_prefix_count..].iter().rev())
            .take_while(|&(old, new)| old.key() == new.key())
            .count();

        let old_shared_suffix_start = old.len() - shared_suffix_count;
        let new_shared_suffix_start = new.len() - shared_suffix_count;

        // Ok, we now hopefully have a smaller range of children in the middle
        // within which to re-order nodes with the same keys, remove old nodes with
        // now-unused keys, and create new nodes with fresh keys.
        self.diff_keyed_middle(
            &old[shared_prefix_count..old_shared_suffix_start],
            &new[shared_prefix_count..new_shared_suffix_start],
            shared_prefix_count,
            shared_suffix_count,
            old_shared_suffix_start,
        );

        // Finally, diff the nodes at the end of `old` and `new` that share keys.
        let old_suffix = &old[old_shared_suffix_start..];
        let new_suffix = &new[new_shared_suffix_start..];
        debug_assert_eq!(old_suffix.len(), new_suffix.len());
        if !old_suffix.is_empty() {
            self.diff_keyed_suffix(old_suffix, new_suffix, new_shared_suffix_start)
        }
    }

    // Diff the prefix of children in `new` and `old` that share the same keys in
    // the same order.
    //
    // The stack is empty upon entry.
    fn diff_keyed_prefix(
        &mut self,
        old: &'bump [VNode<'bump>],
        new: &'bump [VNode<'bump>],
    ) -> KeyedPrefixResult {
        let mut shared_prefix_count = 0;

        for (old, new) in old.iter().zip(new.iter()) {
            // abort early if we finally run into nodes with different keys
            if old.key() != new.key() {
                break;
            }
            self.diff_node(old, new);
            shared_prefix_count += 1;
        }

        // If that was all of the old children, then create and append the remaining
        // new children and we're finished.
        if shared_prefix_count == old.len() {
            // Load the last element
            let last_node = self.find_last_element(new.last().unwrap()).direct_id();
            self.mutations.push_root(last_node);

            // Create the new children and insert them after
            //
            todo!();
            // let meta = self.create_children(&new[shared_prefix_count..]);
            // self.mutations.insert_after(meta.added_to_stack);

            return KeyedPrefixResult::Finished;
        }

        // And if that was all of the new children, then remove all of the remaining
        // old children and we're finished.
        if shared_prefix_count == new.len() {
            self.remove_nodes(&old[shared_prefix_count..]);
            return KeyedPrefixResult::Finished;
        }

        KeyedPrefixResult::MoreWorkToDo(shared_prefix_count)
    }

    // Create the given children and append them to the parent node.
    //
    // The parent node must currently be on top of the change list stack:
    //
    //     [... parent]
    //
    // When this function returns, the change list stack is in the same state.
    fn create_and_append_children(&mut self, new: &'bump [VNode<'bump>]) {
        for child in new {
            todo!();
            // let meta = self.create_vnode(child);
            // self.mutations.append_children(meta.added_to_stack);
        }
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
    // Upon exit from this function, it will be restored to that same state.
    fn diff_keyed_middle(
        &mut self,
        old: &'bump [VNode<'bump>],
        mut new: &'bump [VNode<'bump>],
        shared_prefix_count: usize,
        shared_suffix_count: usize,
        old_shared_suffix_start: usize,
    ) {
        /*
        1. Map the old keys into a numerical ordering based on indicies.
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
        debug_assert_ne!(new.first().map(|n| n.key()), old.first().map(|o| o.key()));
        debug_assert_ne!(new.last().map(|n| n.key()), old.last().map(|o| o.key()));

        // 1. Map the old keys into a numerical ordering based on indicies.
        // 2. Create a map of old key to its index
        // IE if the keys were A B C, then we would have (A, 1) (B, 2) (C, 3).
        let old_key_to_old_index = old
            .iter()
            .enumerate()
            .map(|(i, o)| (o.key().unwrap(), i))
            .collect::<FxHashMap<_, _>>();

        let mut shared_keys = FxHashSet::default();
        let mut to_add = FxHashSet::default();

        // 3. Map each new key to the old key, carrying over the old index.
        let new_index_to_old_index = new
            .iter()
            .map(|n| {
                let key = n.key().unwrap();
                if let Some(&index) = old_key_to_old_index.get(&key) {
                    shared_keys.insert(key);
                    index
                } else {
                    to_add.insert(key);
                    u32::MAX as usize
                }
            })
            .collect::<Vec<_>>();

        // If none of the old keys are reused by the new children, then we
        // remove all the remaining old children and create the new children
        // afresh.
        if shared_suffix_count == 0 && shared_keys.is_empty() {
            self.replace_and_create_many_with_many(old, new);
            return;
        }

        // 4. Compute the LIS of this list

        // The longest increasing subsequence within `new_index_to_old_index`. This
        // is the longest sequence on DOM nodes in `old` that are relatively ordered
        // correctly within `new`. We will leave these nodes in place in the DOM,
        // and only move nodes that are not part of the LIS. This results in the
        // maximum number of DOM nodes left in place, AKA the minimum number of DOM
        // nodes moved.
        let mut new_index_is_in_lis = FxHashSet::default();
        new_index_is_in_lis.reserve(new_index_to_old_index.len());

        let mut predecessors = vec![0; new_index_to_old_index.len()];
        let mut starts = vec![0; new_index_to_old_index.len()];

        longest_increasing_subsequence::lis_with(
            &new_index_to_old_index,
            &mut new_index_is_in_lis,
            |a, b| a < b,
            &mut predecessors,
            &mut starts,
        );

        // use the old nodes to navigate the new nodes
        let mut lis_in_order = new_index_is_in_lis.into_iter().collect::<Vec<_>>();
        lis_in_order.sort_unstable();

        // we walk front to back, creating the head node
        // diff the shared, in-place nodes first
        // this makes sure we can rely on their first/last nodes being correct later on
        for id in &lis_in_order {
            let new_node = &new[*id];
            let key = new_node.key().unwrap();
            let old_index = old_key_to_old_index.get(&key).unwrap();
            let old_node = &old[*old_index];
            self.diff_node(old_node, new_node);
        }

        // return the old node from the key
        let load_old_node_from_lsi = |key| -> &VNode {
            let old_index = old_key_to_old_index.get(key).unwrap();
            let old_node = &old[*old_index];
            old_node
        };

        let mut root = None;
        let mut new_iter = new.iter().enumerate();
        for lis_id in &lis_in_order {
            eprintln!("tracking {:?}", lis_id);
            // this is the next milestone node we are working up to
            let new_anchor = &new[*lis_id];
            root = Some(new_anchor);

            // let anchor_el = self.find_first_element(new_anchor);
            // self.mutations.push_root(anchor_el.direct_id());
            // // let mut pushed = false;

            'inner: loop {
                let (next_id, next_new) = new_iter.next().unwrap();
                if next_id == *lis_id {
                    // we've reached the milestone, break this loop so we can step to the next milestone
                    // remember: we already diffed this node
                    eprintln!("breaking {:?}", next_id);
                    break 'inner;
                } else {
                    let key = next_new.key().unwrap();
                    eprintln!("found key {:?}", key);
                    if shared_keys.contains(&key) {
                        eprintln!("key is contained {:?}", key);
                        shared_keys.remove(key);
                        // diff the two nodes
                        let old_node = load_old_node_from_lsi(key);
                        self.diff_node(old_node, next_new);

                        // now move all the nodes into the right spot
                        for child in RealChildIterator::new(next_new, self.vdom) {
                            let el = child.direct_id();
                            self.mutations.push_root(el);
                            self.mutations.insert_before(1);
                        }
                    } else {
                        self.stack.push(DiffInstruction::Create {
                            node: next_new,
                            and: MountType::InsertBefore {
                                other_node: Some(new_anchor),
                            },
                        });
                    }
                }
            }

            self.mutations.pop();
        }

        let final_lis_node = root.unwrap();
        let final_el_node = self.find_last_element(final_lis_node);
        let final_el = final_el_node.direct_id();
        self.mutations.push_root(final_el);

        let mut last_iter = new.iter().rev().enumerate();
        let last_key = final_lis_node.key().unwrap();
        loop {
            let (last_id, last_node) = last_iter.next().unwrap();
            let key = last_node.key().unwrap();

            eprintln!("checking final nodes {:?}", key);

            if last_key == key {
                eprintln!("breaking final nodes");
                break;
            }

            if shared_keys.contains(&key) {
                eprintln!("key is contained {:?}", key);
                shared_keys.remove(key);
                // diff the two nodes
                let old_node = load_old_node_from_lsi(key);
                self.diff_node(old_node, last_node);

                // now move all the nodes into the right spot
                for child in RealChildIterator::new(last_node, self.vdom) {
                    let el = child.direct_id();
                    self.mutations.push_root(el);
                    self.mutations.insert_after(1);
                }
            } else {
                eprintln!("key is not contained {:?}", key);
                // new node needs to be created
                // insert it before the current milestone
                todo!();
                // let meta = self.create_vnode(last_node);
                // self.mutations.insert_after(meta.added_to_stack);
            }
        }
        self.mutations.pop();
    }

    // Diff the suffix of keyed children that share the same keys in the same order.
    //
    // The parent must be on the change list stack when we enter this function:
    //
    //     [... parent]
    //
    // When this function exits, the change list stack remains the same.
    fn diff_keyed_suffix(
        &mut self,
        old: &'bump [VNode<'bump>],
        new: &'bump [VNode<'bump>],
        new_shared_suffix_start: usize,
    ) {
        debug_assert_eq!(old.len(), new.len());
        debug_assert!(!old.is_empty());

        for (old_child, new_child) in old.iter().zip(new.iter()) {
            self.diff_node(old_child, new_child);
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
    fn diff_non_keyed_children(&mut self, old: &'bump [VNode<'bump>], new: &'bump [VNode<'bump>]) {
        // Handled these cases in `diff_children` before calling this function.
        //
        debug_assert!(!new.is_empty());
        debug_assert!(!old.is_empty());

        match old.len().cmp(&new.len()) {
            // old.len > new.len -> removing some nodes
            Ordering::Greater => {
                // Generate instructions to diff the existing elements
                for (new_child, old_child) in new.iter().zip(old.iter()).rev() {
                    self.stack.push(DiffInstruction::DiffNode {
                        new: new_child,
                        old: old_child,
                    });
                }

                self.stack.push(DiffInstruction::RemoveChildren {
                    children: &old[new.len()..],
                });
            }

            // old.len < new.len -> adding some nodes
            // this is wrong in the case where we're diffing fragments
            //
            // we need to save the last old element and then replace it with all the new ones
            Ordering::Less => {
                // Generate instructions to diff the existing elements
                for (new_child, old_child) in new.iter().zip(old.iter()).rev() {
                    self.stack.push(DiffInstruction::DiffNode {
                        new: new_child,
                        old: old_child,
                    });
                }

                // Generate instructions to add in the new elements
                self.stack.create_children(
                    &new[old.len()..],
                    MountType::InsertAfter {
                        other_node: old.last(),
                    },
                );
            }

            // old.len == new.len -> no nodes added/removed, but perhaps changed
            Ordering::Equal => {
                for (new_child, old_child) in new.iter().zip(old.iter()).rev() {
                    self.stack.push(DiffInstruction::DiffNode {
                        new: new_child,
                        old: old_child,
                    });
                }
            }
        }
    }

    fn find_last_element(&mut self, vnode: &'bump VNode<'bump>) -> &'bump VNode<'bump> {
        let mut search_node = Some(vnode);

        loop {
            let node = search_node.take().unwrap();
            match &node {
                // the ones that have a direct id
                VNode::Text(_) | VNode::Element(_) | VNode::Anchor(_) | VNode::Suspended(_) => {
                    break node
                }

                VNode::Fragment(frag) => {
                    search_node = frag.children.last();
                }
                VNode::Component(el) => {
                    let scope_id = el.ass_scope.get().unwrap();
                    let scope = self.vdom.get_scope(scope_id).unwrap();
                    search_node = Some(scope.root());
                }
            }
        }
    }

    fn find_first_element(&mut self, vnode: &'bump VNode<'bump>) -> &'bump VNode<'bump> {
        let mut search_node = Some(vnode);

        loop {
            let node = search_node.take().unwrap();
            match &node {
                // the ones that have a direct id
                VNode::Text(_) | VNode::Element(_) | VNode::Anchor(_) | VNode::Suspended(_) => {
                    break node
                }

                VNode::Fragment(frag) => {
                    search_node = Some(&frag.children[0]);
                }
                VNode::Component(el) => {
                    let scope_id = el.ass_scope.get().unwrap();
                    let scope = self.vdom.get_scope(scope_id).unwrap();
                    search_node = Some(scope.root());
                }
            }
        }
    }

    // fn remove_child(&mut self, node: &'bump VNode<'bump>) {
    //     self.replace_and_create_many_with_many(Some(node), None);
    // }

    fn replace_and_create_one_with_one(
        &mut self,
        old: &'bump VNode<'bump>,
        new: &'bump VNode<'bump>,
    ) {
        self.stack.create_node(new, MountType::Replace { old });
    }

    fn replace_many_with_many(&mut self, old: &'bump [VNode<'bump>], new: &'bump [VNode<'bump>]) {
        //
    }

    fn replace_one_with_many(&mut self, old: &'bump VNode<'bump>, new: &'bump [VNode<'bump>]) {
        self.stack.create_children(new, MountType::Replace { old });
    }

    fn replace_and_create_many_with_one(
        &mut self,
        old: &'bump [VNode<'bump>],
        new: &'bump VNode<'bump>,
    ) {
        if let Some(first_old) = old.get(0) {
            self.remove_nodes(&old[1..]);
            self.stack
                .create_node(new, MountType::Replace { old: first_old });
        } else {
            self.stack.create_node(new, MountType::Append {});
        }
    }

    /// schedules nodes for garbage collection and pushes "remove" to the mutation stack
    /// remove can happen whenever
    fn remove_nodes(&mut self, nodes: impl IntoIterator<Item = &'bump VNode<'bump>>) {
        // or cache the vec on the diff machine
        for node in nodes {
            match node {
                VNode::Text(t) => {
                    t.dom_id.get().map(|id| {
                        self.mutations.remove(id.as_u64());
                        self.vdom.collect_garbage(id);
                    });
                }
                VNode::Suspended(s) => {
                    s.node.get().map(|id| {
                        self.mutations.remove(id.as_u64());
                        self.vdom.collect_garbage(id);
                    });
                }
                VNode::Anchor(a) => {
                    a.dom_id.get().map(|id| {
                        self.mutations.remove(id.as_u64());
                        self.vdom.collect_garbage(id);
                    });
                }
                VNode::Element(e) => {
                    e.dom_id.get().map(|id| self.mutations.remove(id.as_u64()));
                }
                VNode::Fragment(f) => {
                    self.remove_nodes(f.children);
                }
                VNode::Component(c) => {
                    let scope_id = c.ass_scope.get().unwrap();
                    let scope = self.vdom.get_scope(scope_id).unwrap();
                    let root = scope.root();
                    self.remove_nodes(Some(root));
                }
            }
        }

        // let mut nodes_to_replace = Vec::new();
        // let mut nodes_to_search = vec![old_node];
        // let mut scopes_obliterated = Vec::new();
        // while let Some(node) = nodes_to_search.pop() {
        //     match &node {
        //         // the ones that have a direct id return immediately
        //         VNode::Text(el) => nodes_to_replace.push(el.dom_id.get().unwrap()),
        //         VNode::Element(el) => nodes_to_replace.push(el.dom_id.get().unwrap()),
        //         VNode::Anchor(el) => nodes_to_replace.push(el.dom_id.get().unwrap()),
        //         VNode::Suspended(el) => nodes_to_replace.push(el.node.get().unwrap()),

        //         // Fragments will either have a single anchor or a list of children
        //         VNode::Fragment(frag) => {
        //             for child in frag.children {
        //                 nodes_to_search.push(child);
        //             }
        //         }

        //         // Components can be any of the nodes above
        //         // However, we do need to track which components need to be removed
        //         VNode::Component(el) => {
        //             let scope_id = el.ass_scope.get().unwrap();
        //             let scope = self.vdom.get_scope(scope_id).unwrap();
        //             let root = scope.root();
        //             nodes_to_search.push(root);
        //             scopes_obliterated.push(scope_id);
        //         }
        //     }
        //     // TODO: enable internal garabge collection
        //     // self.create_garbage(node);
        // }

        // let n = nodes_to_replace.len();
        // for node in nodes_to_replace {
        //     self.mutations.push_root(node);
        // }

        // let mut nodes_created = 0;
        // for node in new_nodes {
        //     todo!();
        // let meta = self.create_vnode(node);
        // nodes_created += meta.added_to_stack;
        // }

        // if 0 nodes are created, then it gets interperted as a deletion
        // self.mutations.replace_with(n as u32, nodes_created);

        // self.instructions.push(DiffInstruction::CreateChildren {
        //     and: MountType::Replace { old: None },
        //     children:
        // });

        // obliterate!
        // for scope in scopes_obliterated {

        // todo: mark as garbage

        // self.destroy_scopes(scope);
        // }
    }

    /// Remove all the old nodes and replace them with newly created new nodes.
    ///
    /// The new nodes *will* be created - don't create them yourself!
    fn replace_and_create_many_with_many(
        &mut self,
        old: &'bump [VNode<'bump>],
        new: &'bump [VNode<'bump>],
    ) {
        if let Some(first_old) = old.get(0) {
            self.remove_nodes(&old[1..]);
            self.stack
                .create_children(new, MountType::Replace { old: first_old })
        } else {
            self.stack.create_children(new, MountType::Append {});
        }
    }

    fn create_garbage(&mut self, node: &'bump VNode<'bump>) {
        match self
            .stack
            .current_scope()
            .and_then(|id| self.vdom.get_scope(id))
        {
            Some(scope) => {
                let garbage: &'bump VNode<'static> = unsafe { std::mem::transmute(node) };
                scope.pending_garbage.borrow_mut().push(garbage);
            }
            None => {
                log::info!("No scope to collect garbage into")
            }
        }
    }

    fn replace_node_with_node(
        &mut self,
        old_node: &'bump VNode<'bump>,
        new_node: &'bump VNode<'bump>,
    ) {
        self.stack.instructions.push(DiffInstruction::Create {
            and: MountType::Replace { old: old_node },
            node: new_node,
        });
    }

    fn fix_listener<'a>(&mut self, listener: &'a Listener<'a>) {
        let scope_id = self.stack.current_scope();
        if let Some(scope_id) = scope_id {
            let scope = self.vdom.get_scope(scope_id).unwrap();
            let mut queue = scope.listeners.borrow_mut();
            let long_listener: &'a Listener<'static> = unsafe { std::mem::transmute(listener) };
            queue.push(long_listener as *const _)
        }
    }
}

enum KeyedPrefixResult {
    // Fast path: we finished diffing all the children just by looking at the
    // prefix of shared keys!
    Finished,
    // There is more diffing work to do. Here is a count of how many children at
    // the beginning of `new` and `old` we already processed.
    MoreWorkToDo(usize),
}
