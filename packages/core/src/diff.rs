//! This module contains the stateful DiffMachine and all methods to diff VNodes, their properties, and their children.
//! The DiffMachine calculates the diffs between the old and new frames, updates the new nodes, and modifies the real dom.
//!
//! Notice:
//! ------
//!
//! The inspiration and code for this module was originally taken from Dodrio (@fitzgen) and modified to support Components,
//! Fragments, Suspense, and additional batching operations.
//!
//! Implementation Details:
//! -----------------------
//!
//! All nodes are addressed by their IDs. The RealDom provides an imperative interface for making changes to these nodes.
//! We don't necessarily intend for changes to happen exactly during the diffing process, so the implementor may choose
//! to batch nodes if it is more performant for their application. The u32 should be a no-op to hash,
//!
//!
//! Further Reading and Thoughts
//! ----------------------------
//!
//! There are more ways of increasing diff performance here that are currently not implemented.
//! More info on how to improve this diffing algorithm:
//!  - https://hacks.mozilla.org/2019/03/fast-bump-allocated-virtual-doms-with-rust-and-wasm/

use crate::{arena::ScopeArena, innerlude::*};
use fxhash::{FxHashMap, FxHashSet};

use std::{
    any::Any,
    cell::Cell,
    cmp::Ordering,
    rc::{Rc, Weak},
};

/// The accompanying "real dom" exposes an imperative API for controlling the UI layout
///
/// Instead of having handles directly over nodes, Dioxus uses simple u32s as node IDs.
/// This allows layouts with up to 4,294,967,295 nodes. If we use nohasher, then retrieving is very fast.

/// The "RealDom" abstracts over the... real dom. Elements are mapped by ID. The RealDom is inteded to maintain a stack
/// of real nodes as the diffing algorithm descenes through the tree. This means that whatever is on top of the stack
/// will receive modifications. However, instead of using child-based methods for descending through the tree, we instead
/// ask the RealDom to either push or pop real nodes onto the stack. This saves us the indexing cost while working on a
/// single node
pub trait RealDom {
    // Navigation
    fn push_root(&self, root: RealDomNode);
    fn pop(&self);

    // Add Nodes to the dom
    fn append_child(&self);
    fn replace_with(&self);

    // Remove Nodesfrom the dom
    fn remove(&self);
    fn remove_all_children(&self);

    // Create
    fn create_text_node(&self, text: &str) -> RealDomNode;
    fn create_element(&self, tag: &str) -> RealDomNode;
    fn create_element_ns(&self, tag: &str, namespace: &str) -> RealDomNode;

    // events
    fn new_event_listener(&self, event: &str, scope: ScopeIdx, id: usize);
    // fn new_event_listener(&self, event: &str);
    fn remove_event_listener(&self, event: &str);

    // modify
    fn set_text(&self, text: &str);
    fn set_attribute(&self, name: &str, value: &str, is_namespaced: bool);
    fn remove_attribute(&self, name: &str);

    // node ref
    fn raw_node_as_any_mut(&self) -> &mut dyn Any;
}

/// The DiffState is a cursor internal to the VirtualDOM's diffing algorithm that allows persistence of state while
/// diffing trees of components. This means we can "re-enter" a subtree of a component by queuing a "NeedToDiff" event.
///
/// By re-entering via NodeDiff, we can connect disparate edits together into a single EditList. This batching of edits
/// leads to very fast re-renders (all done in a single animation frame).
///
/// It also means diffing two trees is only ever complex as diffing a single smaller tree, and then re-entering at a
/// different cursor position.
///
/// The order of these re-entrances is stored in the DiffState itself. The DiffState comes pre-loaded with a set of components
/// that were modified by the eventtrigger. This prevents doubly evaluating components if they were both updated via
/// subscriptions and props changes.
pub struct DiffMachine<'a, Dom: RealDom> {
    pub dom: &'a mut Dom,
    pub cur_idx: ScopeIdx,
    pub diffed: FxHashSet<ScopeIdx>,
    pub components: ScopeArena,
    pub event_queue: EventQueue,
    pub seen_nodes: FxHashSet<ScopeIdx>,
}

impl<'a, Dom: RealDom> DiffMachine<'a, Dom> {
    pub fn new(
        dom: &'a mut Dom,
        components: ScopeArena,
        cur_idx: ScopeIdx,
        event_queue: EventQueue,
    ) -> Self {
        Self {
            components,
            dom,
            cur_idx,
            event_queue,
            diffed: FxHashSet::default(),
            seen_nodes: FxHashSet::default(),
        }
    }
    // Diff the `old` node with the `new` node. Emits instructions to modify a
    // physical DOM node that reflects `old` into something that reflects `new`.
    //
    // Upon entry to this function, the physical DOM node must be on the top of the
    // change list stack:
    //
    //     [... node]
    //
    // The change list stack is in the same state when this function exits.
    // In the case of Fragments, the parent node is on the stack
    pub fn diff_node(&mut self, old_node: &VNode<'a>, new_node: &VNode<'a>) {
        // pub fn diff_node(&self, old: &VNode<'a>, new: &VNode<'a>) {
        /*
        For each valid case, we "commit traversal", meaning we save this current position in the tree.
        Then, we diff and queue an edit event (via chagelist). s single trees - when components show up, we save that traversal and then re-enter later.
        When re-entering, we reuse the EditList in DiffState
        */
        match old_node {
            VNode::Element(old) => match new_node {
                // New node is an element, old node was en element, need to investiage more deeply
                VNode::Element(new) => {
                    // If the element type is completely different, the element needs to be re-rendered completely
                    // This is an optimization React makes due to how users structure their code
                    if new.tag_name != old.tag_name || new.namespace != old.namespace {
                        self.create(new_node);
                        self.dom.replace_with();
                        return;
                    }

                    self.diff_listeners(old.listeners, new.listeners);
                    self.diff_attr(old.attributes, new.attributes, new.namespace.is_some());
                    self.diff_children(old.children, new.children);
                }
                // New node is a text element, need to replace the element with a simple text node
                VNode::Text(_) => {
                    self.create(new_node);
                    self.dom.replace_with();
                }

                // New node is a component
                // Make the component and replace our element on the stack with it
                VNode::Component(_) => {
                    self.create(new_node);
                    self.dom.replace_with();
                }

                // New node is actually a sequence of nodes.
                // We need to replace this one node with a sequence of nodes
                // Not yet implement because it's kinda hairy
                VNode::Fragment(_) => todo!(),

                // New Node is actually suspended. Todo
                VNode::Suspended => todo!(),
            },

            // Old element was text
            VNode::Text(old) => match new_node {
                VNode::Text(new) => {
                    if old.text != new.text {
                        self.dom.set_text(new.text);
                    }
                }
                VNode::Element(_) | VNode::Component(_) => {
                    self.create(new_node);
                    self.dom.replace_with();
                }

                // TODO on handling these types
                VNode::Fragment(_) => todo!(),
                VNode::Suspended => todo!(),
            },

            // Old element was a component
            VNode::Component(old) => {
                match new_node {
                    // It's something entirely different
                    VNode::Element(_) | VNode::Text(_) => {
                        self.create(new_node);
                        self.dom.replace_with();
                    }

                    // It's also a component
                    VNode::Component(new) => {
                        match old.user_fc == new.user_fc {
                            // Make sure we're dealing with the same component (by function pointer)
                            true => {
                                // Make sure the new component vnode is referencing the right scope id
                                let scope_id = old.ass_scope.borrow().clone();
                                *new.ass_scope.borrow_mut() = scope_id;

                                // make sure the component's caller function is up to date
                                self.components
                                    .with_scope(scope_id.unwrap(), |scope| {
                                        scope.caller = Rc::downgrade(&new.caller)
                                    })
                                    .unwrap();

                                // React doesn't automatically memoize, but we do.
                                // The cost is low enough to make it worth checking
                                let should_render = match old.comparator {
                                    Some(comparator) => comparator(new),
                                    None => true,
                                };

                                if should_render {
                                    // // self.dom.commit_traversal();
                                    self.components
                                        .with_scope(scope_id.unwrap(), |f| {
                                            f.run_scope().unwrap();
                                        })
                                        .unwrap();
                                    // diff_machine.change_list.load_known_root(root_id);
                                    // run the scope
                                    //
                                } else {
                                    // Component has memoized itself and doesn't need to be re-rendered.
                                    // We still need to make sure the child's props are up-to-date.
                                    // Don't commit traversal
                                }
                            }
                            // It's an entirely different component
                            false => {
                                // A new component has shown up! We need to destroy the old node

                                // Wipe the old one and plant the new one
                                // self.dom.commit_traversal();
                                // self.dom.replace_node_with(old.dom_id, new.dom_id);
                                // self.create(new_node);
                                // self.dom.replace_with();
                                self.create(new_node);
                                // self.create_and_repalce(new_node, old.mounted_root.get());

                                // Now we need to remove the old scope and all of its descendents
                                let old_scope = old.ass_scope.borrow().as_ref().unwrap().clone();
                                self.destroy_scopes(old_scope);
                            }
                        }
                    }
                    VNode::Fragment(_) => todo!(),
                    VNode::Suspended => todo!(),
                }
            }

            VNode::Fragment(old) => {
                //
                match new_node {
                    VNode::Fragment(_) => todo!(),

                    // going from fragment to element means we're going from many (or potentially none) to one
                    VNode::Element(new) => {}
                    VNode::Text(_) => todo!(),
                    VNode::Suspended => todo!(),
                    VNode::Component(_) => todo!(),
                }
            }

            // a suspended node will perform a mem-copy of the previous elements until it is ready
            // this means that event listeners will need to be disabled and removed
            // it also means that props will need to disabled - IE if the node "came out of hibernation" any props should be considered outdated
            VNode::Suspended => {
                //
                match new_node {
                    VNode::Suspended => todo!(),
                    VNode::Element(_) => todo!(),
                    VNode::Text(_) => todo!(),
                    VNode::Fragment(_) => todo!(),
                    VNode::Component(_) => todo!(),
                }
            }
        }
    }

    // Emit instructions to create the given virtual node.
    //
    // The change list stack may have any shape upon entering this function:
    //
    //     [...]
    //
    // When this function returns, the new node is on top of the change list stack:
    //
    //     [... node]
    fn create(&mut self, node: &VNode<'a>) {
        // debug_assert!(self.dom.traversal_is_committed());
        match node {
            VNode::Text(text) => {
                let real_id = self.dom.create_text_node(text.text);
                text.dom_id.set(real_id);
            }
            VNode::Element(el) => {
                let VElement {
                    key,
                    tag_name,
                    listeners,
                    attributes,
                    children,
                    namespace,
                    dom_id,
                } = el;
                // log::info!("Creating {:#?}", node);
                let real_id = if let Some(namespace) = namespace {
                    self.dom.create_element_ns(tag_name, namespace)
                } else {
                    self.dom.create_element(tag_name)
                };
                el.dom_id.set(real_id);

                listeners.iter().enumerate().for_each(|(_id, listener)| {
                    todo!()
                    // dom
                    //     .new_event_listener(listener.event, listener.scope, listener.id)
                });

                for attr in *attributes {
                    self.dom
                        .set_attribute(&attr.name, &attr.value, namespace.is_some());
                }

                // Fast path: if there is a single text child, it is faster to
                // create-and-append the text node all at once via setting the
                // parent's `textContent` in a single change list instruction than
                // to emit three instructions to (1) create a text node, (2) set its
                // text content, and finally (3) append the text node to this
                // parent.
                if children.len() == 1 {
                    if let VNode::Text(text) = &children[0] {
                        self.dom.set_text(text.text);
                        return;
                    }
                }

                for child in *children {
                    self.create(child);
                    if let VNode::Fragment(_) = child {
                        // do nothing
                        // fragments append themselves
                    } else {
                        self.dom.append_child();
                    }
                }
            }

            VNode::Component(component) => {
                self.dom.create_text_node("placeholder for vcomponent");

                // let root_id = next_id();
                // self.dom.save_known_root(root_id);

                log::debug!("Mounting a new component");
                let caller: Weak<OpaqueComponent> = Rc::downgrade(&component.caller);

                // We're modifying the component arena while holding onto references into the assoiated bump arenas of its children
                // those references are stable, even if the component arena moves around in memory, thanks to the bump arenas.
                // However, there is no way to convey this to rust, so we need to use unsafe to pierce through the lifetime.

                let parent_idx = self.cur_idx;

                // Insert a new scope into our component list
                let idx = self
                    .components
                    .with(|components| {
                        components.insert_with(|new_idx| {
                            let parent_scope = self.components.try_get(parent_idx).unwrap();
                            let height = parent_scope.height + 1;
                            Scope::new(
                                caller,
                                new_idx,
                                Some(parent_idx),
                                height,
                                self.event_queue.new_channel(height, new_idx),
                                self.components.clone(),
                                component.children,
                            )
                        })
                    })
                    .unwrap();

                {
                    let cur_component = self.components.try_get_mut(idx).unwrap();
                    let mut ch = cur_component.descendents.borrow_mut();
                    ch.insert(idx);
                    std::mem::drop(ch);
                }

                // yaaaaay lifetimes out of thin air
                // really tho, we're merging the frame lifetimes together
                let inner: &'a mut _ = unsafe { &mut *self.components.0.borrow().arena.get() };
                let new_component = inner.get_mut(idx).unwrap();

                // Actually initialize the caller's slot with the right address
                *component.ass_scope.borrow_mut() = Some(idx);

                // Run the scope for one iteration to initialize it
                new_component.run_scope().unwrap();

                // And then run the diff algorithm
                todo!();
                // self.diff_node(new_component.old_frame(), new_component.next_frame());

                // Finally, insert this node as a seen node.
                self.seen_nodes.insert(idx);
            }

            // we go the the "known root" but only operate on a sibling basis
            VNode::Fragment(frag) => {
                // create the children directly in the space
                for child in frag.children {
                    todo!()
                    // self.create(child);
                    // self.dom.append_child();
                }
            }

            VNode::Suspended => {
                todo!("Creation of VNode::Suspended not yet supported")
            }
        }
    }
}

impl<'a, Dom: RealDom> DiffMachine<'a, Dom> {
    /// Destroy a scope and all of its descendents.
    ///
    /// Calling this will run the destuctors on all hooks in the tree.
    /// It will also add the destroyed nodes to the `seen_nodes` cache to prevent them from being renderered.
    fn destroy_scopes(&mut self, old_scope: ScopeIdx) {
        let mut nodes_to_delete = vec![old_scope];
        let mut scopes_to_explore = vec![old_scope];

        // explore the scope tree breadth first
        while let Some(scope_id) = scopes_to_explore.pop() {
            // If we're planning on deleting this node, then we don't need to both rendering it
            self.seen_nodes.insert(scope_id);
            let scope = self.components.try_get(scope_id).unwrap();
            for child in scope.descendents.borrow().iter() {
                // Add this node to be explored
                scopes_to_explore.push(child.clone());

                // Also add it for deletion
                nodes_to_delete.push(child.clone());
            }
        }

        // Delete all scopes that we found as part of this subtree
        for node in nodes_to_delete {
            log::debug!("Removing scope {:#?}", node);
            let _scope = self.components.try_remove(node).unwrap();
            // do anything we need to do to delete the scope
            // I think we need to run the destructors on the hooks
            // TODO
        }
    }

    // Diff event listeners between `old` and `new`.
    //
    // The listeners' node must be on top of the change list stack:
    //
    //     [... node]
    //
    // The change list stack is left unchanged.
    fn diff_listeners(&self, old: &[Listener<'_>], new: &[Listener<'_>]) {
        if !old.is_empty() || !new.is_empty() {
            // self.dom.commit_traversal();
        }

        'outer1: for (_l_idx, new_l) in new.iter().enumerate() {
            // go through each new listener
            // find its corresponding partner in the old list
            // if any characteristics changed, remove and then re-add

            // if nothing changed, then just move on

            let event_type = new_l.event;

            for old_l in old {
                if new_l.event == old_l.event {
                    if new_l.id != old_l.id {
                        self.dom.remove_event_listener(event_type);
                        // TODO! we need to mess with events and assign them by RealDomNode
                        // self.dom
                        //     .update_event_listener(event_type, new_l.scope, new_l.id)
                    }

                    continue 'outer1;
                }
            }

            self.dom
                .new_event_listener(event_type, new_l.scope, new_l.id);
        }

        'outer2: for old_l in old {
            for new_l in new {
                if new_l.event == old_l.event {
                    continue 'outer2;
                }
            }
            self.dom.remove_event_listener(old_l.event);
        }
    }

    // Diff a node's attributes.
    //
    // The attributes' node must be on top of the change list stack:
    //
    //     [... node]
    //
    // The change list stack is left unchanged.
    fn diff_attr(&self, old: &'a [Attribute<'a>], new: &'a [Attribute<'a>], is_namespaced: bool) {
        // Do O(n^2) passes to add/update and remove attributes, since
        // there are almost always very few attributes.
        //
        // The "fast" path is when the list of attributes name is identical and in the same order
        // With the Rsx and Html macros, this will almost always be the case
        'outer: for new_attr in new {
            if new_attr.is_volatile() {
                // self.dom.commit_traversal();
                self.dom
                    .set_attribute(new_attr.name, new_attr.value, is_namespaced);
            } else {
                for old_attr in old {
                    if old_attr.name == new_attr.name {
                        if old_attr.value != new_attr.value {
                            // self.dom.commit_traversal();
                            self.dom
                                .set_attribute(new_attr.name, new_attr.value, is_namespaced);
                        }
                        continue 'outer;
                    } else {
                        // names are different, a varying order of attributes has arrived
                    }
                }

                // self.dom.commit_traversal();
                self.dom
                    .set_attribute(new_attr.name, new_attr.value, is_namespaced);
            }
        }

        'outer2: for old_attr in old {
            for new_attr in new {
                if old_attr.name == new_attr.name {
                    continue 'outer2;
                }
            }

            // self.dom.commit_traversal();
            self.dom.remove_attribute(old_attr.name);
        }
    }

    // Diff the given set of old and new children.
    //
    // The parent must be on top of the change list stack when this function is
    // entered:
    //
    //     [... parent]
    //
    // the change list stack is in the same state when this function returns.
    fn diff_children(&mut self, old: &'a [VNode<'a>], new: &'a [VNode<'a>]) {
        if new.is_empty() {
            if !old.is_empty() {
                // self.dom.commit_traversal();
                self.remove_all_children(old);
            }
            return;
        }

        if new.len() == 1 {
            match (&old.first(), &new[0]) {
                (Some(VNode::Text(old_vtext)), VNode::Text(new_vtext))
                    if old_vtext.text == new_vtext.text =>
                {
                    // Don't take this fast path...
                }

                (_, VNode::Text(text)) => {
                    // self.dom.commit_traversal();
                    self.dom.set_text(text.text);
                    return;
                }

                // todo: any more optimizations
                (_, _) => {}
            }
        }

        if old.is_empty() {
            if !new.is_empty() {
                // self.dom.commit_traversal();
                self.create_and_append_children(new);
            }
            return;
        }

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
            todo!("Not yet implemented a migration away from temporaries");
            // let t = self.dom.next_temporary();
            // self.diff_keyed_children(old, new);
            // self.dom.set_next_temporary(t);
        } else {
            self.diff_non_keyed_children(old, new);
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
    // When entering this function, the parent must be on top of the change list
    // stack:
    //
    //     [... parent]
    //
    // Upon exiting, the change list stack is in the same state.
    fn diff_keyed_children(&self, old: &[VNode<'a>], new: &[VNode<'a>]) {
        todo!();
        // if cfg!(debug_assertions) {
        //     let mut keys = fxhash::FxHashSet::default();
        //     let mut assert_unique_keys = |children: &[VNode]| {
        //         keys.clear();
        //         for child in children {
        //             let key = child.key();
        //             debug_assert!(
        //                 key.is_some(),
        //                 "if any sibling is keyed, all siblings must be keyed"
        //             );
        //             keys.insert(key);
        //         }
        //         debug_assert_eq!(
        //             children.len(),
        //             keys.len(),
        //             "keyed siblings must each have a unique key"
        //         );
        //     };
        //     assert_unique_keys(old);
        //     assert_unique_keys(new);
        // }

        // First up, we diff all the nodes with the same key at the beginning of the
        // children.
        //
        // `shared_prefix_count` is the count of how many nodes at the start of
        // `new` and `old` share the same keys.
        let shared_prefix_count = match self.diff_keyed_prefix(old, new) {
            KeyedPrefixResult::Finished => return,
            KeyedPrefixResult::MoreWorkToDo(count) => count,
        };

        match self.diff_keyed_prefix(old, new) {
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
    // Upon entry of this function, the change list stack must be:
    //
    //     [... parent]
    //
    // Upon exit, the change list stack is the same.
    fn diff_keyed_prefix(&self, old: &[VNode<'a>], new: &[VNode<'a>]) -> KeyedPrefixResult {
        todo!()
        // self.dom.go_down();
        // let mut shared_prefix_count = 0;

        // for (i, (old, new)) in old.iter().zip(new.iter()).enumerate() {
        //     if old.key() != new.key() {
        //         break;
        //     }

        //     self.dom.go_to_sibling(i);

        //     self.diff_node(old, new);

        //     shared_prefix_count += 1;
        // }

        // // If that was all of the old children, then create and append the remaining
        // // new children and we're finished.
        // if shared_prefix_count == old.len() {
        //     self.dom.go_up();
        //     // self.dom.commit_traversal();
        //     self.create_and_append_children(&new[shared_prefix_count..]);
        //     return KeyedPrefixResult::Finished;
        // }

        // // And if that was all of the new children, then remove all of the remaining
        // // old children and we're finished.
        // if shared_prefix_count == new.len() {
        //     self.dom.go_to_sibling(shared_prefix_count);
        //     // self.dom.commit_traversal();
        //     self.remove_self_and_next_siblings(&old[shared_prefix_count..]);
        //     return KeyedPrefixResult::Finished;
        // }

        // self.dom.go_up();
        // KeyedPrefixResult::MoreWorkToDo(shared_prefix_count)
    }

    // The most-general, expensive code path for keyed children diffing.
    //
    // We find the longest subsequence within `old` of children that are relatively
    // ordered the same way in `new` (via finding a longest-increasing-subsequence
    // of the old child's index within `new`). The children that are elements of
    // this subsequence will remain in place, minimizing the number of DOM moves we
    // will have to do.
    //
    // Upon entry to this function, the change list stack must be:
    //
    //     [... parent]
    //
    // Upon exit from this function, it will be restored to that same state.
    fn diff_keyed_middle(
        &self,
        old: &[VNode<'a>],
        mut new: &[VNode<'a>],
        shared_prefix_count: usize,
        shared_suffix_count: usize,
        old_shared_suffix_start: usize,
    ) {
        todo!()
        // // Should have already diffed the shared-key prefixes and suffixes.
        // debug_assert_ne!(new.first().map(|n| n.key()), old.first().map(|o| o.key()));
        // debug_assert_ne!(new.last().map(|n| n.key()), old.last().map(|o| o.key()));

        // // The algorithm below relies upon using `u32::MAX` as a sentinel
        // // value, so if we have that many new nodes, it won't work. This
        // // check is a bit academic (hence only enabled in debug), since
        // // wasm32 doesn't have enough address space to hold that many nodes
        // // in memory.
        // debug_assert!(new.len() < u32::MAX as usize);

        // // Map from each `old` node's key to its index within `old`.
        // let mut old_key_to_old_index = FxHashMap::default();
        // old_key_to_old_index.reserve(old.len());
        // old_key_to_old_index.extend(old.iter().enumerate().map(|(i, o)| (o.key(), i)));

        // // The set of shared keys between `new` and `old`.
        // let mut shared_keys = FxHashSet::default();
        // // Map from each index in `new` to the index of the node in `old` that
        // // has the same key.
        // let mut new_index_to_old_index = Vec::with_capacity(new.len());
        // new_index_to_old_index.extend(new.iter().map(|n| {
        //     let key = n.key();
        //     if let Some(&i) = old_key_to_old_index.get(&key) {
        //         shared_keys.insert(key);
        //         i
        //     } else {
        //         u32::MAX as usize
        //     }
        // }));

        // // If none of the old keys are reused by the new children, then we
        // // remove all the remaining old children and create the new children
        // // afresh.
        // if shared_suffix_count == 0 && shared_keys.is_empty() {
        //     if shared_prefix_count == 0 {
        //         // self.dom.commit_traversal();
        //         self.remove_all_children(old);
        //     } else {
        //         self.dom.go_down_to_child(shared_prefix_count);
        //         // self.dom.commit_traversal();
        //         self.remove_self_and_next_siblings(&old[shared_prefix_count..]);
        //     }

        //     self.create_and_append_children(new);

        //     return;
        // }

        // // Save each of the old children whose keys are reused in the new
        // // children.
        // let mut old_index_to_temp = vec![u32::MAX; old.len()];
        // let mut start = 0;
        // loop {
        //     let end = (start..old.len())
        //         .find(|&i| {
        //             let key = old[i].key();
        //             !shared_keys.contains(&key)
        //         })
        //         .unwrap_or(old.len());

        //     if end - start > 0 {
        //         // self.dom.commit_traversal();
        //         let mut t = self.dom.save_children_to_temporaries(
        //             shared_prefix_count + start,
        //             shared_prefix_count + end,
        //         );
        //         for i in start..end {
        //             old_index_to_temp[i] = t;
        //             t += 1;
        //         }
        //     }

        //     debug_assert!(end <= old.len());
        //     if end == old.len() {
        //         break;
        //     } else {
        //         start = end + 1;
        //     }
        // }

        // // Remove any old children whose keys were not reused in the new
        // // children. Remove from the end first so that we don't mess up indices.
        // let mut removed_count = 0;
        // for (i, old_child) in old.iter().enumerate().rev() {
        //     if !shared_keys.contains(&old_child.key()) {
        //         // registry.remove_subtree(old_child);
        //         // todo
        //         // self.dom.commit_traversal();
        //         self.dom.remove_child(i + shared_prefix_count);
        //         removed_count += 1;
        //     }
        // }

        // // If there aren't any more new children, then we are done!
        // if new.is_empty() {
        //     return;
        // }

        // // The longest increasing subsequence within `new_index_to_old_index`. This
        // // is the longest sequence on DOM nodes in `old` that are relatively ordered
        // // correctly within `new`. We will leave these nodes in place in the DOM,
        // // and only move nodes that are not part of the LIS. This results in the
        // // maximum number of DOM nodes left in place, AKA the minimum number of DOM
        // // nodes moved.
        // let mut new_index_is_in_lis = FxHashSet::default();
        // new_index_is_in_lis.reserve(new_index_to_old_index.len());
        // let mut predecessors = vec![0; new_index_to_old_index.len()];
        // let mut starts = vec![0; new_index_to_old_index.len()];
        // longest_increasing_subsequence::lis_with(
        //     &new_index_to_old_index,
        //     &mut new_index_is_in_lis,
        //     |a, b| a < b,
        //     &mut predecessors,
        //     &mut starts,
        // );

        // // Now we will iterate from the end of the new children back to the
        // // beginning, diffing old children we are reusing and if they aren't in the
        // // LIS moving them to their new destination, or creating new children. Note
        // // that iterating in reverse order lets us use `Node.prototype.insertBefore`
        // // to move/insert children.
        // //
        // // But first, we ensure that we have a child on the change list stack that
        // // we can `insertBefore`. We handle this once before looping over `new`
        // // children, so that we don't have to keep checking on every loop iteration.
        // if shared_suffix_count > 0 {
        //     // There is a shared suffix after these middle children. We will be
        //     // inserting before that shared suffix, so add the first child of that
        //     // shared suffix to the change list stack.
        //     //
        //     // [... parent]
        //     self.dom
        //         .go_down_to_child(old_shared_suffix_start - removed_count);
        // // [... parent first_child_of_shared_suffix]
        // } else {
        //     // There is no shared suffix coming after these middle children.
        //     // Therefore we have to process the last child in `new` and move it to
        //     // the end of the parent's children if it isn't already there.
        //     let last_index = new.len() - 1;
        //     // uhhhh why an unwrap?
        //     let last = new.last().unwrap();
        //     // let last = new.last().unwrap_throw();
        //     new = &new[..new.len() - 1];
        //     if shared_keys.contains(&last.key()) {
        //         let old_index = new_index_to_old_index[last_index];
        //         let temp = old_index_to_temp[old_index];
        //         // [... parent]
        //         self.dom.go_down_to_temp_child(temp);
        //         // [... parent last]
        //         self.diff_node(&old[old_index], last);

        //         if new_index_is_in_lis.contains(&last_index) {
        //             // Don't move it, since it is already where it needs to be.
        //         } else {
        //             // self.dom.commit_traversal();
        //             // [... parent last]
        //             self.dom.append_child();
        //             // [... parent]
        //             self.dom.go_down_to_temp_child(temp);
        //             // [... parent last]
        //         }
        //     } else {
        //         // self.dom.commit_traversal();
        //         // [... parent]
        //         self.create(last);

        //         // [... parent last]
        //         self.dom.append_child();
        //         // [... parent]
        //         self.dom.go_down_to_reverse_child(0);
        //         // [... parent last]
        //     }
        // }

        // for (new_index, new_child) in new.iter().enumerate().rev() {
        //     let old_index = new_index_to_old_index[new_index];
        //     if old_index == u32::MAX as usize {
        //         debug_assert!(!shared_keys.contains(&new_child.key()));
        //         // self.dom.commit_traversal();
        //         // [... parent successor]
        //         self.create(new_child);
        //         // [... parent successor new_child]
        //         self.dom.insert_before();
        //     // [... parent new_child]
        //     } else {
        //         debug_assert!(shared_keys.contains(&new_child.key()));
        //         let temp = old_index_to_temp[old_index];
        //         debug_assert_ne!(temp, u32::MAX);

        //         if new_index_is_in_lis.contains(&new_index) {
        //             // [... parent successor]
        //             self.dom.go_to_temp_sibling(temp);
        //         // [... parent new_child]
        //         } else {
        //             // self.dom.commit_traversal();
        //             // [... parent successor]
        //             self.dom.push_temporary(temp);
        //             // [... parent successor new_child]
        //             self.dom.insert_before();
        //             // [... parent new_child]
        //         }

        //         self.diff_node(&old[old_index], new_child);
        //     }
        // }

        // // [... parent child]
        // self.dom.go_up();
        // [... parent]
    }

    // Diff the suffix of keyed children that share the same keys in the same order.
    //
    // The parent must be on the change list stack when we enter this function:
    //
    //     [... parent]
    //
    // When this function exits, the change list stack remains the same.
    fn diff_keyed_suffix(
        &self,
        old: &[VNode<'a>],
        new: &[VNode<'a>],
        new_shared_suffix_start: usize,
    ) {
        todo!()
        //     debug_assert_eq!(old.len(), new.len());
        //     debug_assert!(!old.is_empty());

        //     // [... parent]
        //     self.dom.go_down();
        //     // [... parent new_child]

        //     for (i, (old_child, new_child)) in old.iter().zip(new.iter()).enumerate() {
        //         self.dom.go_to_sibling(new_shared_suffix_start + i);
        //         self.diff_node(old_child, new_child);
        //     }

        //     // [... parent]
        //     self.dom.go_up();
    }

    // Diff children that are not keyed.
    //
    // The parent must be on the top of the change list stack when entering this
    // function:
    //
    //     [... parent]
    //
    // the change list stack is in the same state when this function returns.
    fn diff_non_keyed_children(&self, old: &'a [VNode<'a>], new: &'a [VNode<'a>]) {
        // Handled these cases in `diff_children` before calling this function.
        debug_assert!(!new.is_empty());
        debug_assert!(!old.is_empty());

        //     [... parent]
        // self.dom.go_down();
        // self.dom.push_root()
        //     [... parent child]

        todo!()
        // for (i, (new_child, old_child)) in new.iter().zip(old.iter()).enumerate() {
        //     // [... parent prev_child]
        //     self.dom.go_to_sibling(i);
        //     // [... parent this_child]
        //     self.diff_node(old_child, new_child);
        // }

        // match old.len().cmp(&new.len()) {
        //     // old.len > new.len -> removing some nodes
        //     Ordering::Greater => {
        //         // [... parent prev_child]
        //         self.dom.go_to_sibling(new.len());
        //         // [... parent first_child_to_remove]
        //         // self.dom.commit_traversal();
        //         // support::remove_self_and_next_siblings(state, &old[new.len()..]);
        //         self.remove_self_and_next_siblings(&old[new.len()..]);
        //         // [... parent]
        //     }
        //     // old.len < new.len -> adding some nodes
        //     Ordering::Less => {
        //         // [... parent last_child]
        //         self.dom.go_up();
        //         // [... parent]
        //         // self.dom.commit_traversal();
        //         self.create_and_append_children(&new[old.len()..]);
        //     }
        //     // old.len == new.len -> no nodes added/removed, but πerhaps changed
        //     Ordering::Equal => {
        //         // [... parent child]
        //         self.dom.go_up();
        //         // [... parent]
        //     }
        // }
    }

    // ======================
    // Support methods
    // ======================

    // Remove all of a node's children.
    //
    // The change list stack must have this shape upon entry to this function:
    //
    //     [... parent]
    //
    // When this function returns, the change list stack is in the same state.
    pub fn remove_all_children(&mut self, old: &[VNode<'a>]) {
        // debug_assert!(self.dom.traversal_is_committed());
        log::debug!("REMOVING CHILDREN");
        for _child in old {
            // registry.remove_subtree(child);
        }
        // Fast way to remove all children: set the node's textContent to an empty
        // string.
        todo!()
        // self.dom.set_inner_text("");
    }

    // Create the given children and append them to the parent node.
    //
    // The parent node must currently be on top of the change list stack:
    //
    //     [... parent]
    //
    // When this function returns, the change list stack is in the same state.
    pub fn create_and_append_children(&mut self, new: &[VNode<'a>]) {
        // debug_assert!(self.dom.traversal_is_committed());
        for child in new {
            // self.create_and_append(node, parent)
            self.create(child);
            self.dom.append_child();
        }
    }

    // Remove the current child and all of its following siblings.
    //
    // The change list stack must have this shape upon entry to this function:
    //
    //     [... parent child]
    //
    // After the function returns, the child is no longer on the change list stack:
    //
    //     [... parent]
    pub fn remove_self_and_next_siblings(&self, old: &[VNode<'a>]) {
        // debug_assert!(self.dom.traversal_is_committed());
        for child in old {
            if let VNode::Component(vcomp) = child {
                // dom
                //     .create_text_node("placeholder for vcomponent");

                todo!()
                // let root_id = vcomp.stable_addr.as_ref().borrow().unwrap();
                // self.lifecycle_events.push_back(LifeCycleEvent::Remove {
                //     root_id,
                //     stable_scope_addr: Rc::downgrade(&vcomp.ass_scope),
                // })
                // let id = get_id();
                // *component.stable_addr.as_ref().borrow_mut() = Some(id);
                // self.dom.save_known_root(id);
                // let scope = Rc::downgrade(&component.ass_scope);
                // self.lifecycle_events.push_back(LifeCycleEvent::Mount {
                //     caller: Rc::downgrade(&component.caller),
                //     root_id: id,
                //     stable_scope_addr: scope,
                // });
            }

            // registry.remove_subtree(child);
        }
        todo!()
        // self.dom.remove_self_and_next_siblings();
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
