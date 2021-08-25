//! This module contains the stateful DiffMachine and all methods to diff VNodes, their properties, and their children.
//!
//! The [`DiffMachine`] calculates the diffs between the old and new frames, updates the new nodes, and generates a set
//! of mutations for the RealDom to apply.
//!
//! ## Notice:
//!
//! The inspiration and code for this module was originally taken from Dodrio (@fitzgen) and then modified to support
//! Components, Fragments, Suspense, SubTree memoization, incremental diffing, cancelation, NodeRefs, pausing, priority
//! scheduling, and additional batching operations.
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
//! Due to the frequent calls to "yield_now" we can get the pure "fetch-as-you-render" behavior of React fiber.
//!
//! We're able to use this approach because we use placeholder nodes - futures that aren't ready still get submitted to
//! DOM, but as a placeholder.
//!
//! Right now, the "suspense" queue is intertwined the hooks. In the future, we should allow any future to drive attributes
//! and contents, without the need for the "use_suspense" hook. In the interim, this is the quickest way to get suspense working.
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
//! - Strong memoization of subtrees.
//! - Guided diffing.
//! - Certain web-dom-specific optimizations.
//!
//! More info on how to improve this diffing algorithm:
//!  - https://hacks.mozilla.org/2019/03/fast-bump-allocated-virtual-doms-with-rust-and-wasm/

use crate::{innerlude::*, scheduler::Scheduler};
use fxhash::{FxHashMap, FxHashSet};
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
    vdom: &'bump Scheduler,

    pub mutations: &'bump mut Mutations<'bump>,

    pub stack: DiffStack<'bump>,
    pub diffed: FxHashSet<ScopeId>,
    pub seen_scopes: FxHashSet<ScopeId>,
}

impl<'bump> DiffMachine<'bump> {
    pub(crate) fn new(
        edits: &'bump mut Mutations<'bump>,
        cur_scope: ScopeId,
        shared: &'bump Scheduler,
    ) -> Self {
        Self {
            stack: DiffStack::new(cur_scope),
            mutations: edits,
            vdom: shared,
            diffed: FxHashSet::default(),
            seen_scopes: FxHashSet::default(),
        }
    }

    // pub fn new_headless(shared: &'bump SharedResources) -> Self {
    //     let edits = Mutations::new();
    //     let cur_scope = ScopeId(0);
    //     Self::new(edits, cur_scope, shared)
    // }

    //
    pub async fn diff_scope(&mut self, id: ScopeId) {
        if let Some(component) = self.vdom.get_scope_mut(id) {
            let (old, new) = (component.frames.wip_head(), component.frames.fin_head());
            self.stack.push(DiffInstruction::DiffNode { new, old });
            self.work().await;
        }
    }

    /// Progress the diffing for this "fiber"
    ///
    /// This method implements a depth-first iterative tree traversal.
    ///
    /// We do depth-first to maintain high cache locality (nodes were originally generated recursively).
    pub async fn work(&mut self) {
        // defer to individual functions so the compiler produces better code
        // large functions tend to be difficult for the compiler to work with
        while let Some(instruction) = self.stack.pop() {
            // todo: call this less frequently, there is a bit of overhead involved
            yield_now().await;

            match instruction {
                DiffInstruction::PopScope => {
                    self.stack.pop_scope();
                }

                DiffInstruction::DiffNode { old, new, .. } => {
                    self.diff_node(old, new);
                }

                DiffInstruction::DiffChildren { old, new } => {
                    self.diff_children(old, new);
                }

                DiffInstruction::Create { node } => {
                    self.create_node(node);
                }

                DiffInstruction::Mount { and } => {
                    self.mount(and);
                }

                DiffInstruction::PrepareMoveNode { node } => {
                    log::debug!("Preparing to move node: {:?}", node);
                    for el in RealChildIterator::new(node, self.vdom) {
                        self.mutations.push_root(el.direct_id());
                        self.stack.add_child_count(1);
                    }
                }
            };
        }
    }

    fn mount(&mut self, and: MountType<'bump>) {
        let nodes_created = self.stack.pop_nodes_created();
        match and {
            // add the nodes from this virtual list to the parent
            // used by fragments and components
            MountType::Absorb => {
                self.stack.add_child_count(nodes_created);
            }
            MountType::Append => {
                self.mutations.edits.push(AppendChildren {
                    many: nodes_created as u32,
                });
            }

            MountType::Replace { old } => {
                let mut iter = RealChildIterator::new(old, self.vdom);
                let first = iter.next().unwrap();
                self.mutations
                    .replace_with(first.direct_id(), nodes_created as u32);
                self.remove_nodes(iter);
            }

            MountType::ReplaceByElementId { el: old } => {
                self.mutations.replace_with(old, nodes_created as u32);
            }

            MountType::InsertAfter { other_node } => {
                let root = self.find_last_element(other_node).unwrap();
                self.mutations.insert_after(root, nodes_created as u32);
            }

            MountType::InsertBefore { other_node } => {
                let root = self.find_first_element_id(other_node).unwrap();
                self.mutations.insert_before(root, nodes_created as u32);
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
        suspended.dom_id.set(Some(real_id));
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
        dom_id.set(Some(real_id));

        self.mutations.create_element(tag_name, *namespace, real_id);

        self.stack.add_child_count(1);

        let cur_scope_id = self.stack.current_scope().unwrap();
        let scope = self.vdom.get_scope(cur_scope_id).unwrap();

        listeners.iter().for_each(|listener| {
            self.attach_listener_to_scope(listener, scope);
            listener.mounted_node.set(Some(real_id));
            self.mutations
                .new_event_listener(listener, cur_scope_id.clone());
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

        let shared = self.vdom.channel.clone();
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
                shared,
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
            (Component(old), Component(new)) => self.diff_component_nodes(old, new),
            (Fragment(old), Fragment(new)) => self.diff_fragment_nodes(old, new),
            (Anchor(old), Anchor(new)) => new.dom_id.set(old.dom_id.get()),
            (Suspended(old), Suspended(new)) => new.dom_id.set(old.dom_id.get()),
            (Element(old), Element(new)) => self.diff_element_nodes(old, new),

            // Anything else is just a basic replace and create
            (
                Component(_) | Fragment(_) | Text(_) | Element(_) | Anchor(_) | Suspended(_),
                Component(_) | Fragment(_) | Text(_) | Element(_) | Anchor(_) | Suspended(_),
            ) => self
                .stack
                .create_node(new_node, MountType::Replace { old: old_node }),
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
            // maybe make this an instruction?
            // issue is that we need the "vnode" but this method only has the velement
            self.stack.push_nodes_created(0);
            self.stack.push(DiffInstruction::Mount {
                and: MountType::ReplaceByElementId {
                    el: old.dom_id.get().unwrap(),
                },
            });
            self.create_element_node(new);
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

        let cur_scope_id = self.stack.current_scope().unwrap();
        let scope = self.vdom.get_scope(cur_scope_id).unwrap();

        if old.listeners.len() == new.listeners.len() {
            for (old_l, new_l) in old.listeners.iter().zip(new.listeners.iter()) {
                if old_l.event != new_l.event {
                    please_commit(&mut self.mutations.edits);
                    self.mutations.remove_event_listener(old_l.event);
                    self.mutations.new_event_listener(new_l, cur_scope_id);
                }
                new_l.mounted_node.set(old_l.mounted_node.get());
                self.attach_listener_to_scope(new_l, scope);
            }
        } else {
            please_commit(&mut self.mutations.edits);
            for listener in old.listeners {
                self.mutations.remove_event_listener(listener.event);
            }
            for listener in new.listeners {
                listener.mounted_node.set(Some(root));
                self.mutations.new_event_listener(listener, cur_scope_id);
                self.attach_listener_to_scope(listener, scope);
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
        // Remember, fragments can never be empty (they always have a single child)
        match (old, new) {
            ([], []) => {}
            ([], _) => {
                self.stack.create_children(new, MountType::Append);
            }
            (_, []) => {
                for node in old {
                    self.remove_nodes(Some(node));
                }
            }
            ([VNode::Anchor(old_anchor)], [VNode::Anchor(new_anchor)]) => {
                old_anchor.dom_id.set(new_anchor.dom_id.get());
            }
            ([VNode::Anchor(anchor)], _) => {
                let el = anchor.dom_id.get().unwrap();
                self.stack
                    .create_children(new, MountType::ReplaceByElementId { el });
            }
            (_, [VNode::Anchor(_)]) => {
                self.replace_and_create_many_with_one(old, &new[0]);
            }
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
    fn diff_non_keyed_children(&mut self, old: &'bump [VNode<'bump>], new: &'bump [VNode<'bump>]) {
        // Handled these cases in `diff_children` before calling this function.
        log::debug!("diffing non-keyed case");
        debug_assert!(!new.is_empty());
        debug_assert!(!old.is_empty());

        for (new, old) in new.iter().zip(old.iter()).rev() {
            self.stack.push(DiffInstruction::DiffNode { new, old });
        }

        if old.len() > new.len() {
            self.remove_nodes(&old[new.len()..]);
        } else if new.len() > old.len() {
            log::debug!("Calling create children on array differences");
            self.stack.create_children(
                &new[old.len()..],
                MountType::InsertAfter {
                    other_node: old.last().unwrap(),
                },
            );
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
        let (left_offset, right_offset) = match self.diff_keyed_ends(old, new) {
            Some(count) => count,
            None => return,
        };
        log::debug!(
            "Left offset, right offset, {}, {}",
            left_offset,
            right_offset,
        );

        // Ok, we now hopefully have a smaller range of children in the middle
        // within which to re-order nodes with the same keys, remove old nodes with
        // now-unused keys, and create new nodes with fresh keys.
        self.diff_keyed_middle(
            &old[left_offset..(old.len() - right_offset)],
            &new[left_offset..(new.len() - right_offset)],
        );
    }

    /// Diff both ends of the children that share keys.
    ///
    /// Returns a left offset and right offset of that indicates a smaller section to pass onto the middle diffing.
    ///
    /// If there is no offset, then this function returns None and the diffing is complete.
    fn diff_keyed_ends(
        &mut self,
        old: &'bump [VNode<'bump>],
        new: &'bump [VNode<'bump>],
    ) -> Option<(usize, usize)> {
        let mut left_offset = 0;

        for (old, new) in old.iter().zip(new.iter()) {
            // abort early if we finally run into nodes with different keys
            if old.key() != new.key() {
                break;
            }
            self.stack.push(DiffInstruction::DiffNode { old, new });
            left_offset += 1;
        }

        // If that was all of the old children, then create and append the remaining
        // new children and we're finished.
        if left_offset == old.len() {
            self.stack.create_children(
                &new[left_offset..],
                MountType::InsertAfter {
                    other_node: old.last().unwrap(),
                },
            );
            return None;
        }

        // And if that was all of the new children, then remove all of the remaining
        // old children and we're finished.
        if left_offset == new.len() {
            self.remove_nodes(&old[left_offset..]);
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
    // Upon exit from this function, it will be restored to that same state.
    fn diff_keyed_middle(&mut self, old: &'bump [VNode<'bump>], new: &'bump [VNode<'bump>]) {
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
            self.replace_and_create_many_with_many(old, new);
            return;
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

        let apply = |new_idx, new_node: &'bump VNode<'bump>, stack: &mut DiffStack<'bump>| {
            let old_index = new_index_to_old_index[new_idx];
            if old_index == u32::MAX as usize {
                stack.create_node(new_node, MountType::Absorb);
            } else {
                // this funciton should never take LIS indicies
                stack.push(DiffInstruction::PrepareMoveNode { node: new_node });
                stack.push(DiffInstruction::DiffNode {
                    new: new_node,
                    old: &old[old_index],
                });
            }
        };

        // add mount instruction for the last items not covered by the lis
        let first_lis = *lis_sequence.first().unwrap();
        if first_lis > 0 {
            self.stack.push_nodes_created(0);
            self.stack.push(DiffInstruction::Mount {
                and: MountType::InsertBefore {
                    other_node: &new[first_lis],
                },
            });

            for (idx, new_node) in new[..first_lis].iter().enumerate().rev() {
                apply(idx, new_node, &mut self.stack);
            }
        }

        // for each spacing, generate a mount instruction
        let mut lis_iter = lis_sequence.iter().rev();
        let mut last = *lis_iter.next().unwrap();
        while let Some(&next) = lis_iter.next() {
            if last - next > 1 {
                self.stack.push_nodes_created(0);
                self.stack.push(DiffInstruction::Mount {
                    and: MountType::InsertBefore {
                        other_node: &new[last],
                    },
                });
                for (idx, new_node) in new[(next + 1)..last].iter().enumerate().rev() {
                    apply(idx + next + 1, new_node, &mut self.stack);
                }
            }
            last = next;
        }

        // add mount instruction for the first items not covered by the lis
        let last = *lis_sequence.last().unwrap();
        if last < (new.len() - 1) {
            self.stack.push_nodes_created(0);
            self.stack.push(DiffInstruction::Mount {
                and: MountType::InsertAfter {
                    other_node: &new[last],
                },
            });
            for (idx, new_node) in new[(last + 1)..].iter().enumerate().rev() {
                apply(idx + last + 1, new_node, &mut self.stack);
            }
        }

        for idx in lis_sequence.iter().rev() {
            self.stack.push(DiffInstruction::DiffNode {
                new: &new[*idx],
                old: &old[new_index_to_old_index[*idx]],
            });
        }
    }

    // =====================
    //  Utilities
    // =====================

    fn find_last_element(&mut self, vnode: &'bump VNode<'bump>) -> Option<ElementId> {
        let mut search_node = Some(vnode);

        loop {
            match &search_node.take().unwrap() {
                VNode::Text(t) => break t.dom_id.get(),
                VNode::Element(t) => break t.dom_id.get(),
                VNode::Suspended(t) => break t.dom_id.get(),
                VNode::Anchor(t) => break t.dom_id.get(),

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

    fn find_first_element_id(&mut self, vnode: &'bump VNode<'bump>) -> Option<ElementId> {
        let mut search_node = Some(vnode);

        loop {
            match &search_node.take().unwrap() {
                // the ones that have a direct id
                VNode::Fragment(frag) => {
                    search_node = Some(&frag.children[0]);
                }
                VNode::Component(el) => {
                    let scope_id = el.ass_scope.get().unwrap();
                    let scope = self.vdom.get_scope(scope_id).unwrap();
                    search_node = Some(scope.root());
                }
                VNode::Text(t) => break t.dom_id.get(),
                VNode::Element(t) => break t.dom_id.get(),
                VNode::Suspended(t) => break t.dom_id.get(),
                VNode::Anchor(t) => break t.dom_id.get(),
            }
        }
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
                    s.dom_id.get().map(|id| {
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

    /// Adds a listener closure to a scope during diff.
    fn attach_listener_to_scope<'a>(&mut self, listener: &'a Listener<'a>, scope: &Scope) {
        let mut queue = scope.listeners.borrow_mut();
        let long_listener: &'a Listener<'static> = unsafe { std::mem::transmute(listener) };
        queue.push(long_listener as *const _)
    }
}
