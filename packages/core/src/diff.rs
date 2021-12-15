//! This module contains the stateful DiffMachine and all methods to diff VNodes, their properties, and their children.
//!
//! The [`DiffMachine`] calculates the diffs between the old and new frames, updates the new nodes, and generates a set
//! of mutations for the RealDom to apply.
//!
//! ## Notice:
//!
//! The inspiration and code for this module was originally taken from Dodrio (@fitzgen) and then modified to support
//! Components, Fragments, Suspense, SubTree memoization, incremental diffing, cancellation, NodeRefs, pausing, priority
//! scheduling, and additional batching operations.
//!
//! ## Implementation Details:
//!
//! ### IDs for elements
//! --------------------
//! All nodes are addressed by their IDs. The RealDom provides an imperative interface for making changes to these nodes.
//! We don't necessarily require that DOM changes happen instantly during the diffing process, so the implementor may choose
//! to batch nodes if it is more performant for their application. The element IDs are indices into the internal element
//! array. The expectation is that implementors will use the ID as an index into a Vec of real nodes, allowing for passive
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
//! Due to the frequent calls to "yield_now" we can get the pure "fetch-as-you-render" behavior of React Fiber.
//!
//! We're able to use this approach because we use placeholder nodes - futures that aren't ready still get submitted to
//! DOM, but as a placeholder.
//!
//! Right now, the "suspense" queue is intertwined with hooks. In the future, we should allow any future to drive attributes
//! and contents, without the need for the "use_suspense" hook. In the interim, this is the quickest way to get Suspense working.
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
//!  - https://hacks.mozilla.org/2019/03/fast-bump-allocated-virtual-doms-with-rust-and-wasm/

use crate::innerlude::*;
use fxhash::{FxHashMap, FxHashSet};
use smallvec::{smallvec, SmallVec};
use DomEdit::*;

/// Our DiffMachine is an iterative tree differ.
///
/// It uses techniques of a stack machine to allow pausing and restarting of the diff algorithm. This
/// was originally implemented using recursive techniques, but Rust lacks the ability to call async functions recursively,
/// meaning we could not "pause" the original diffing algorithm.
///
/// Instead, we use a traditional stack machine approach to diff and create new nodes. The diff algorithm periodically
/// calls "yield_now" which allows the machine to pause and return control to the caller. The caller can then wait for
/// the next period of idle time, preventing our diff algorithm from blocking the main thread.
///
/// Funnily enough, this stack machine's entire job is to create instructions for another stack machine to execute. It's
/// stack machines all the way down!
pub(crate) struct DiffState<'bump> {
    pub(crate) scopes: &'bump ScopeArena,
    pub(crate) mutations: Mutations<'bump>,
    pub(crate) stack: DiffStack<'bump>,
    pub(crate) force_diff: bool,
}

impl<'bump> DiffState<'bump> {
    pub(crate) fn new(scopes: &'bump ScopeArena) -> Self {
        Self {
            scopes,
            mutations: Mutations::new(),
            stack: DiffStack::new(),
            force_diff: false,
        }
    }
}

/// The stack instructions we use to diff and create new nodes.
#[derive(Debug)]
pub(crate) enum DiffInstruction<'a> {
    Diff {
        old: &'a VNode<'a>,
        new: &'a VNode<'a>,
    },

    Create {
        node: &'a VNode<'a>,
    },

    /// pushes the node elements onto the stack for use in mount
    PrepareMove {
        node: &'a VNode<'a>,
    },

    Mount {
        and: MountType<'a>,
    },

    PopScope,
    PopElement,
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum MountType<'a> {
    Absorb,
    Append,
    Replace { old: &'a VNode<'a> },
    InsertAfter { other_node: &'a VNode<'a> },
    InsertBefore { other_node: &'a VNode<'a> },
}

pub(crate) struct DiffStack<'bump> {
    pub(crate) instructions: Vec<DiffInstruction<'bump>>,
    pub(crate) nodes_created_stack: SmallVec<[usize; 10]>,
    pub(crate) scope_stack: SmallVec<[ScopeId; 5]>,
    pub(crate) element_stack: SmallVec<[ElementId; 10]>,
}

impl<'bump> DiffStack<'bump> {
    fn new() -> Self {
        Self {
            instructions: Vec::with_capacity(1000),
            nodes_created_stack: smallvec![],
            scope_stack: smallvec![],
            element_stack: smallvec![],
        }
    }

    fn pop(&mut self) -> Option<DiffInstruction<'bump>> {
        self.instructions.pop()
    }

    fn pop_off_scope(&mut self) {
        self.scope_stack.pop();
    }

    pub(crate) fn push(&mut self, instruction: DiffInstruction<'bump>) {
        self.instructions.push(instruction)
    }

    fn create_children(&mut self, children: &'bump [VNode<'bump>], and: MountType<'bump>) {
        self.nodes_created_stack.push(0);
        self.instructions.push(DiffInstruction::Mount { and });

        for child in children.iter().rev() {
            self.instructions
                .push(DiffInstruction::Create { node: child });
        }
    }

    // todo: subtrees
    // fn push_subtree(&mut self) {
    //     self.nodes_created_stack.push(0);
    //     self.instructions.push(DiffInstruction::Mount {
    //         and: MountType::Append,
    //     });
    // }

    fn push_nodes_created(&mut self, count: usize) {
        self.nodes_created_stack.push(count);
    }

    pub(crate) fn create_node(&mut self, node: &'bump VNode<'bump>, and: MountType<'bump>) {
        self.nodes_created_stack.push(0);
        self.instructions.push(DiffInstruction::Mount { and });
        self.instructions.push(DiffInstruction::Create { node });
    }

    fn add_child_count(&mut self, count: usize) {
        *self.nodes_created_stack.last_mut().unwrap() += count;
    }

    fn pop_nodes_created(&mut self) -> usize {
        self.nodes_created_stack.pop().unwrap()
    }

    fn current_scope(&self) -> Option<ScopeId> {
        self.scope_stack.last().copied()
    }

    fn create_component(&mut self, idx: ScopeId, node: &'bump VNode<'bump>) {
        // Push the new scope onto the stack
        self.scope_stack.push(idx);

        self.instructions.push(DiffInstruction::PopScope);

        // Run the creation algorithm with this scope on the stack
        // ?? I think we treat components as fragments??
        self.instructions.push(DiffInstruction::Create { node });
    }
}

impl<'bump> DiffState<'bump> {
    pub fn diff_scope(&mut self, id: ScopeId) {
        let (old, new) = (self.scopes.wip_head(id), self.scopes.fin_head(id));
        self.stack.push(DiffInstruction::Diff { old, new });
        self.stack.scope_stack.push(id);
        let scope = self.scopes.get_scope(id).unwrap();
        self.stack.element_stack.push(scope.container);
        self.work(|| false);
    }

    /// Progress the diffing for this "fiber"
    ///
    /// This method implements a depth-first iterative tree traversal.
    ///
    /// We do depth-first to maintain high cache locality (nodes were originally generated recursively).
    ///
    /// Returns a `bool` indicating that the work completed properly.
    pub fn work(&mut self, mut deadline_expired: impl FnMut() -> bool) -> bool {
        while let Some(instruction) = self.stack.pop() {
            match instruction {
                DiffInstruction::Diff { old, new } => self.diff_node(old, new),
                DiffInstruction::Create { node } => self.create_node(node),
                DiffInstruction::Mount { and } => self.mount(and),
                DiffInstruction::PrepareMove { node } => {
                    let num_on_stack = self.push_all_nodes(node);
                    self.stack.add_child_count(num_on_stack);
                }
                DiffInstruction::PopScope => self.stack.pop_off_scope(),
                DiffInstruction::PopElement => {
                    self.stack.element_stack.pop();
                }
            };

            if deadline_expired() {
                log::debug!("Deadline expired before we could finish!");
                return false;
            }
        }

        true
    }

    // recursively push all the nodes of a tree onto the stack and return how many are there
    fn push_all_nodes(&mut self, node: &'bump VNode<'bump>) -> usize {
        match node {
            VNode::Text(_) | VNode::Placeholder(_) => {
                self.mutations.push_root(node.mounted_id());
                1
            }

            VNode::Fragment(_) | VNode::Component(_) => {
                //
                let mut added = 0;
                for child in node.children() {
                    added += self.push_all_nodes(child);
                }
                added
            }

            VNode::Element(el) => {
                let mut num_on_stack = 0;
                for child in el.children.iter() {
                    num_on_stack += self.push_all_nodes(child);
                }
                self.mutations.push_root(el.dom_id.get().unwrap());

                num_on_stack + 1
            }
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

            MountType::Replace { old } => {
                self.replace_node(old, nodes_created);
            }

            MountType::Append => {
                self.mutations.edits.push(AppendChildren {
                    many: nodes_created as u32,
                });
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
            VNode::Text(vtext) => self.create_text_node(vtext, node),
            VNode::Placeholder(anchor) => self.create_anchor_node(anchor, node),
            VNode::Element(element) => self.create_element_node(element, node),
            VNode::Fragment(frag) => self.create_fragment_node(frag),
            VNode::Component(component) => self.create_component_node(*component),
        }
    }

    fn create_text_node(&mut self, vtext: &'bump VText<'bump>, node: &'bump VNode<'bump>) {
        let real_id = self.scopes.reserve_node(node);

        self.mutations.create_text_node(vtext.text, real_id);
        vtext.dom_id.set(Some(real_id));
        self.stack.add_child_count(1);
    }

    fn create_anchor_node(&mut self, anchor: &'bump VPlaceholder, node: &'bump VNode<'bump>) {
        let real_id = self.scopes.reserve_node(node);

        self.mutations.create_placeholder(real_id);
        anchor.dom_id.set(Some(real_id));

        self.stack.add_child_count(1);
    }

    fn create_element_node(&mut self, element: &'bump VElement<'bump>, node: &'bump VNode<'bump>) {
        let VElement {
            tag_name,
            listeners,
            attributes,
            children,
            namespace,
            dom_id,
            parent_id,
            ..
        } = element;

        // set the parent ID for event bubbling
        self.stack.instructions.push(DiffInstruction::PopElement);

        let parent = self.stack.element_stack.last().unwrap();
        parent_id.set(Some(*parent));

        // set the id of the element
        let real_id = self.scopes.reserve_node(node);
        self.stack.element_stack.push(real_id);
        dom_id.set(Some(real_id));

        self.mutations.create_element(tag_name, *namespace, real_id);

        self.stack.add_child_count(1);

        if let Some(cur_scope_id) = self.stack.current_scope() {
            let scope = self.scopes.get_scope(cur_scope_id).unwrap();

            for listener in *listeners {
                self.attach_listener_to_scope(listener, scope);
                listener.mounted_node.set(Some(real_id));
                self.mutations.new_event_listener(listener, cur_scope_id);
            }
        } else {
            log::warn!("create element called with no scope on the stack - this is an error for a live dom");
        }

        for attr in *attributes {
            self.mutations.set_attribute(attr, real_id.as_u64());
        }

        // todo: the settext optimization

        // if children.len() == 1 {
        //     if let VNode::Text(vtext) = children[0] {
        //         self.mutations.set_text(vtext.text, real_id.as_u64());
        //         return;
        //     }
        // }

        if !children.is_empty() {
            self.stack.create_children(children, MountType::Append);
        }
    }

    fn create_fragment_node(&mut self, frag: &'bump VFragment<'bump>) {
        self.stack.create_children(frag.children, MountType::Absorb);
    }

    fn create_component_node(&mut self, vcomponent: &'bump VComponent<'bump>) {
        let parent_idx = self.stack.current_scope().unwrap();

        // Insert a new scope into our component list
        let parent_scope = self.scopes.get_scope(parent_idx).unwrap();
        let height = parent_scope.height + 1;
        let subtree = parent_scope.subtree.get();

        let parent_scope = self.scopes.get_scope_raw(parent_idx);
        let caller = unsafe { std::mem::transmute(vcomponent.caller as *const _) };
        let fc_ptr = vcomponent.user_fc;

        let container = *self.stack.element_stack.last().unwrap();

        let new_idx =
            self.scopes
                .new_with_key(fc_ptr, caller, parent_scope, container, height, subtree);

        // Actually initialize the caller's slot with the right address
        vcomponent.associated_scope.set(Some(new_idx));

        if !vcomponent.can_memoize {
            let cur_scope = self.scopes.get_scope(parent_idx).unwrap();
            let extended = unsafe { std::mem::transmute(vcomponent) };
            cur_scope.items.borrow_mut().borrowed_props.push(extended);
        } else {
            // the props are currently bump allocated but we need to move them to the heap
        }

        // TODO: add noderefs to current noderef list Noderefs
        let _new_component = self.scopes.get_scope(new_idx).unwrap();

        log::debug!(
            "initializing component {:?} with height {:?}",
            new_idx,
            height + 1
        );

        // Run the scope for one iteration to initialize it
        if self.scopes.run_scope(new_idx) {
            // Take the node that was just generated from running the component
            let nextnode = self.scopes.fin_head(new_idx);
            self.stack.create_component(new_idx, nextnode);

            // todo: subtrees
            // if new_component.is_subtree_root.get() {
            //     self.stack.push_subtree();
            // }
        }

        // Finally, insert this scope as a seen node.
        self.mutations.dirty_scopes.insert(new_idx);
    }

    // =================================
    //  Tools for diffing nodes
    // =================================

    pub fn diff_node(&mut self, old_node: &'bump VNode<'bump>, new_node: &'bump VNode<'bump>) {
        use VNode::*;
        match (old_node, new_node) {
            // Check the most common cases first
            (Text(old), Text(new)) => {
                self.diff_text_nodes(old, new, old_node, new_node);
            }
            (Component(old), Component(new)) => {
                self.diff_component_nodes(old_node, new_node, *old, *new)
            }
            (Fragment(old), Fragment(new)) => self.diff_fragment_nodes(old, new),
            (Placeholder(old), Placeholder(new)) => new.dom_id.set(old.dom_id.get()),
            (Element(old), Element(new)) => self.diff_element_nodes(old, new, old_node, new_node),

            // Anything else is just a basic replace and create
            (
                Component(_) | Fragment(_) | Text(_) | Element(_) | Placeholder(_),
                Component(_) | Fragment(_) | Text(_) | Element(_) | Placeholder(_),
            ) => self
                .stack
                .create_node(new_node, MountType::Replace { old: old_node }),
        }
    }

    fn diff_text_nodes(
        &mut self,
        old: &'bump VText<'bump>,
        new: &'bump VText<'bump>,
        _old_node: &'bump VNode<'bump>,
        new_node: &'bump VNode<'bump>,
    ) {
        if let Some(root) = old.dom_id.get() {
            if old.text != new.text {
                self.mutations.set_text(new.text, root.as_u64());
            }
            self.scopes.update_node(new_node, root);

            new.dom_id.set(Some(root));
        }
    }

    fn diff_element_nodes(
        &mut self,
        old: &'bump VElement<'bump>,
        new: &'bump VElement<'bump>,
        old_node: &'bump VNode<'bump>,
        new_node: &'bump VNode<'bump>,
    ) {
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
                and: MountType::Replace { old: old_node },
            });
            self.create_element_node(new, new_node);
            return;
        }

        self.scopes.update_node(new_node, root);

        new.dom_id.set(Some(root));
        new.parent_id.set(old.parent_id.get());

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
                self.mutations.set_attribute(attribute, root.as_u64())
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
        if let Some(cur_scope_id) = self.stack.current_scope() {
            let scope = self.scopes.get_scope(cur_scope_id).unwrap();

            if old.listeners.len() == new.listeners.len() {
                for (old_l, new_l) in old.listeners.iter().zip(new.listeners.iter()) {
                    if old_l.event != new_l.event {
                        self.mutations
                            .remove_event_listener(old_l.event, root.as_u64());
                        self.mutations.new_event_listener(new_l, cur_scope_id);
                    }
                    new_l.mounted_node.set(old_l.mounted_node.get());
                    self.attach_listener_to_scope(new_l, scope);
                }
            } else {
                for listener in old.listeners {
                    self.mutations
                        .remove_event_listener(listener.event, root.as_u64());
                }
                for listener in new.listeners {
                    listener.mounted_node.set(Some(root));
                    self.mutations.new_event_listener(listener, cur_scope_id);
                    self.attach_listener_to_scope(listener, scope);
                }
            }
        }

        if old.children.is_empty() && !new.children.is_empty() {
            self.mutations.edits.push(PushRoot {
                root: root.as_u64(),
            });
            self.stack.element_stack.push(root);
            self.stack.instructions.push(DiffInstruction::PopElement);
            self.stack.create_children(new.children, MountType::Append);
        } else {
            self.stack.element_stack.push(root);
            self.stack.instructions.push(DiffInstruction::PopElement);
            self.diff_children(old.children, new.children);
        }

        // todo: this is for the "settext" optimization
        // it works, but i'm not sure if it's the direction we want to take

        // match (old.children.len(), new.children.len()) {
        //     (0, 0) => {}
        //     (1, 1) => {
        //         let old1 = &old.children[0];
        //         let new1 = &new.children[0];

        //         match (old1, new1) {
        //             (VNode::Text(old_text), VNode::Text(new_text)) => {
        //                 if old_text.text != new_text.text {
        //                     self.mutations.set_text(new_text.text, root.as_u64());
        //                 }
        //             }
        //             (VNode::Text(_old_text), _) => {
        //                 self.stack.element_stack.push(root);
        //                 self.stack.instructions.push(DiffInstruction::PopElement);
        //                 self.stack.create_node(new1, MountType::Append);
        //             }
        //             (_, VNode::Text(new_text)) => {
        //                 self.remove_nodes([old1], false);
        //                 self.mutations.set_text(new_text.text, root.as_u64());
        //             }
        //             _ => {
        //                 self.stack.element_stack.push(root);
        //                 self.stack.instructions.push(DiffInstruction::PopElement);
        //                 self.diff_children(old.children, new.children);
        //             }
        //         }
        //     }
        //     (0, 1) => {
        //         if let VNode::Text(text) = &new.children[0] {
        //             self.mutations.set_text(text.text, root.as_u64());
        //         } else {
        //             self.stack.element_stack.push(root);
        //             self.stack.instructions.push(DiffInstruction::PopElement);
        //         }
        //     }
        //     (0, _) => {
        //         self.mutations.edits.push(PushRoot {
        //             root: root.as_u64(),
        //         });
        //         self.stack.element_stack.push(root);
        //         self.stack.instructions.push(DiffInstruction::PopElement);
        //         self.stack.create_children(new.children, MountType::Append);
        //     }
        //     (_, 0) => {
        //         self.remove_nodes(old.children, false);
        //         self.mutations.set_text("", root.as_u64());
        //     }
        //     (_, _) => {
        //         self.stack.element_stack.push(root);
        //         self.stack.instructions.push(DiffInstruction::PopElement);
        //         self.diff_children(old.children, new.children);
        //     }
        // }
    }

    fn diff_component_nodes(
        &mut self,
        old_node: &'bump VNode<'bump>,
        new_node: &'bump VNode<'bump>,
        old: &'bump VComponent<'bump>,
        new: &'bump VComponent<'bump>,
    ) {
        let scope_addr = old.associated_scope.get().unwrap();

        log::debug!(
            "Diffing components. old_scope: {:?}, old_addr: {:?}, new_addr: {:?}",
            scope_addr,
            old.user_fc,
            new.user_fc
        );

        // Make sure we're dealing with the same component (by function pointer)
        if old.user_fc == new.user_fc {
            self.stack.scope_stack.push(scope_addr);

            // Make sure the new component vnode is referencing the right scope id
            new.associated_scope.set(Some(scope_addr));

            // make sure the component's caller function is up to date
            let scope = self
                .scopes
                .get_scope(scope_addr)
                .unwrap_or_else(|| panic!("could not find {:?}", scope_addr));

            scope.caller.set(unsafe { std::mem::transmute(new.caller) });

            // React doesn't automatically memoize, but we do.
            let props_are_the_same = old.comparator.unwrap();

            if (self.force_diff || !props_are_the_same(new)) && self.scopes.run_scope(scope_addr) {
                self.diff_node(
                    self.scopes.wip_head(scope_addr),
                    self.scopes.fin_head(scope_addr),
                );
            }

            self.stack.scope_stack.pop();
        } else {
            self.stack
                .create_node(new_node, MountType::Replace { old: old_node });
        }
    }

    fn diff_fragment_nodes(&mut self, old: &'bump VFragment<'bump>, new: &'bump VFragment<'bump>) {
        // This is the case where options or direct vnodes might be used.
        // In this case, it's faster to just skip ahead to their diff
        if old.children.len() == 1 && new.children.len() == 1 {
            self.diff_node(&old.children[0], &new.children[0]);
            return;
        }

        debug_assert!(!old.children.is_empty());
        debug_assert!(!new.children.is_empty());

        self.diff_children(old.children, new.children);
    }

    // =============================================
    //  Utilities for creating new diff instructions
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
    // Fragment nodes cannot generate empty children lists, so we can assume that when a list is empty, it belongs only
    // to an element, and appending makes sense.
    fn diff_children(&mut self, old: &'bump [VNode<'bump>], new: &'bump [VNode<'bump>]) {
        // Remember, fragments can never be empty (they always have a single child)
        match (old, new) {
            ([], []) => {}
            ([], _) => {
                // we need to push the
                self.stack.create_children(new, MountType::Append);
            }
            (_, []) => {
                self.remove_nodes(old, true);
            }
            ([VNode::Placeholder(old_anchor)], [VNode::Placeholder(new_anchor)]) => {
                old_anchor.dom_id.set(new_anchor.dom_id.get());
            }
            ([VNode::Placeholder(_)], _) => {
                self.stack
                    .create_children(new, MountType::Replace { old: &old[0] });
            }
            (_, [VNode::Placeholder(_)]) => {
                let new: &'bump VNode<'bump> = &new[0];
                if let Some(first_old) = old.get(0) {
                    self.remove_nodes(&old[1..], true);
                    self.stack
                        .create_node(new, MountType::Replace { old: first_old });
                } else {
                    self.stack.create_node(new, MountType::Append {});
                }
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
        debug_assert!(!new.is_empty());
        debug_assert!(!old.is_empty());

        for (new, old) in new.iter().zip(old.iter()).rev() {
            self.stack.push(DiffInstruction::Diff { new, old });
        }

        use std::cmp::Ordering;
        match old.len().cmp(&new.len()) {
            Ordering::Greater => self.remove_nodes(&old[new.len()..], true),
            Ordering::Less => {
                self.stack.create_children(
                    &new[old.len()..],
                    MountType::InsertAfter {
                        other_node: old.last().unwrap(),
                    },
                );
            }
            Ordering::Equal => {
                // nothing - they're the same size
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
                self.stack.create_children(
                    new_middle,
                    MountType::InsertBefore {
                        other_node: foothold,
                    },
                );
            } else if right_offset == 0 {
                // insert at the end  the old list
                let foothold = old.last().unwrap();
                self.stack.create_children(
                    new_middle,
                    MountType::InsertAfter {
                        other_node: foothold,
                    },
                );
            } else {
                // inserting in the middle
                let foothold = &old[left_offset - 1];
                self.stack.create_children(
                    new_middle,
                    MountType::InsertAfter {
                        other_node: foothold,
                    },
                );
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

        old: &'bump [VNode<'bump>],
        new: &'bump [VNode<'bump>],
    ) -> Option<(usize, usize)> {
        let mut left_offset = 0;

        for (old, new) in old.iter().zip(new.iter()) {
            // abort early if we finally run into nodes with different keys
            if old.key() != new.key() {
                break;
            }
            self.stack.push(DiffInstruction::Diff { old, new });
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
    fn diff_keyed_middle(&mut self, old: &'bump [VNode<'bump>], new: &'bump [VNode<'bump>]) {
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
        debug_assert_ne!(new.first().map(|n| n.key()), old.first().map(|o| o.key()));
        debug_assert_ne!(new.last().map(|n| n.key()), old.last().map(|o| o.key()));

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
                self.stack
                    .create_children(new, MountType::Replace { old: first_old })
            } else {
                self.stack.create_children(new, MountType::Append {});
            }
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
                // this function should never take LIS indices
                stack.push(DiffInstruction::PrepareMove { node: new_node });
                stack.push(DiffInstruction::Diff {
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
        for next in lis_iter {
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
            last = *next;
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
            self.stack.push(DiffInstruction::Diff {
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
                VNode::Placeholder(t) => break t.dom_id.get(),
                VNode::Fragment(frag) => {
                    search_node = frag.children.last();
                }
                VNode::Component(el) => {
                    let scope_id = el.associated_scope.get().unwrap();
                    search_node = Some(self.scopes.root_node(scope_id));
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
                    let scope_id = el.associated_scope.get().unwrap();
                    search_node = Some(self.scopes.root_node(scope_id));
                }
                VNode::Text(t) => break t.dom_id.get(),
                VNode::Element(t) => break t.dom_id.get(),
                VNode::Placeholder(t) => break t.dom_id.get(),
            }
        }
    }

    fn replace_node(&mut self, old: &'bump VNode<'bump>, nodes_created: usize) {
        match old {
            VNode::Element(el) => {
                let id = old
                    .try_mounted_id()
                    .unwrap_or_else(|| panic!("broke on {:?}", old));

                log::debug!("element parent is {:?}", el.parent_id.get());

                self.mutations.replace_with(id, nodes_created as u32);
                self.remove_nodes(el.children, false);
            }

            VNode::Text(_) | VNode::Placeholder(_) => {
                let id = old
                    .try_mounted_id()
                    .unwrap_or_else(|| panic!("broke on {:?}", old));

                self.mutations.replace_with(id, nodes_created as u32);
            }

            VNode::Fragment(f) => {
                self.replace_node(&f.children[0], nodes_created);
                self.remove_nodes(f.children.iter().skip(1), true);
            }

            VNode::Component(c) => {
                let node = self.scopes.fin_head(c.associated_scope.get().unwrap());
                self.replace_node(node, nodes_created);

                let scope_id = c.associated_scope.get().unwrap();
                log::debug!("Destroying scope {:?}", scope_id);
                self.scopes.try_remove(scope_id).unwrap();
            }
        }
    }

    /// schedules nodes for garbage collection and pushes "remove" to the mutation stack
    /// remove can happen whenever
    fn remove_nodes(
        &mut self,
        nodes: impl IntoIterator<Item = &'bump VNode<'bump>>,
        gen_muts: bool,
    ) {
        // or cache the vec on the diff machine
        for node in nodes {
            match node {
                VNode::Text(t) => {
                    // this check exists because our null node will be removed but does not have an ID
                    if let Some(id) = t.dom_id.get() {
                        self.scopes.collect_garbage(id);

                        if gen_muts {
                            self.mutations.remove(id.as_u64());
                        }
                    }
                }
                VNode::Placeholder(a) => {
                    let id = a.dom_id.get().unwrap();
                    self.scopes.collect_garbage(id);

                    if gen_muts {
                        self.mutations.remove(id.as_u64());
                    }
                }
                VNode::Element(e) => {
                    let id = e.dom_id.get().unwrap();

                    if gen_muts {
                        self.mutations.remove(id.as_u64());
                    }

                    self.remove_nodes(e.children, false);
                }

                VNode::Fragment(f) => {
                    self.remove_nodes(f.children, gen_muts);
                }

                VNode::Component(c) => {
                    self.destroy_vomponent(c, gen_muts);
                }
            }
        }
    }

    fn destroy_vomponent(&mut self, vc: &VComponent, gen_muts: bool) {
        let scope_id = vc.associated_scope.get().unwrap();
        let root = self.scopes.root_node(scope_id);
        self.remove_nodes(Some(root), gen_muts);
        log::debug!("Destroying scope {:?}", scope_id);
        self.scopes.try_remove(scope_id).unwrap();
    }

    /// Adds a listener closure to a scope during diff.
    fn attach_listener_to_scope(&mut self, listener: &'bump Listener<'bump>, scope: &ScopeState) {
        let long_listener = unsafe { std::mem::transmute(listener) };
        scope.items.borrow_mut().listeners.push(long_listener)
    }
}
