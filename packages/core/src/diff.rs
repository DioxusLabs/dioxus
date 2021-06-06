//! Diff the `old` node with the `new` node. Emits instructions to modify a
//! physical DOM node that reflects `old` into something that reflects `new`.
//!
//! Upon entry to this function, the physical DOM node must be on the top of the
//! change list stack:
//!
//!     [... node]
//!
//! The change list stack is in the same state when this function exits.
//!
//! ----
//!
//! There are more ways of increasing diff performance here that are currently not implemented.
//! Additionally, the caching mechanism has also been tweaked.
//!
//! Instead of having "cached" nodes, each component is, by default, a cached node. This leads to increased
//! memory overhead for large numbers of small components, but we can optimize this by tracking alloc size over time
//! and shrinking bumps down if possible.
//!
//! Additionally, clean up of these components is not done at diff time (though it should), but rather, the diffing
//! proprogates removal lifecycle events for affected components into the event queue. It's not imperative that these
//! are ran immediately, but it should be noted that cleanup of components might be able to emit changes.
//!
//! This diffing only ever occurs on a component-by-component basis (not entire trees at once).
//!
//! Currently, the listener situation is a bit broken.
//! We aren't removing listeners (choosing to leak them instead) :(
//! Eventually, we'll set things up so add/remove listener is an instruction again
//!
//! A major assumption of this diff algorithm when combined with the ChangeList is that the Changelist will be
//! fresh and the event queue is clean. This lets us continue to batch edits together under the same ChangeList
//!
//! More info on how to improve this diffing algorithm:
//!  - https://hacks.mozilla.org/2019/03/fast-bump-allocated-virtual-doms-with-rust-and-wasm/
use crate::{arena::ScopeArena, innerlude::*};
use bumpalo::Bump;
use fxhash::{FxHashMap, FxHashSet};
use generational_arena::Arena;

use std::{
    cell::{RefCell, RefMut},
    cmp::Ordering,
    collections::VecDeque,
    rc::{Rc, Weak},
    sync::atomic::AtomicU32,
};

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
pub struct DiffMachine<'a> {
    pub create_diffs: bool,
    pub change_list: EditMachine<'a>,
    pub diffed: FxHashSet<ScopeIdx>,
    pub lifecycle_events: VecDeque<LifeCycleEvent<'a>>,
    pub vdom: ScopeArena,
}
pub enum LifeCycleEvent<'a> {
    Mount {
        caller: Weak<dyn Fn(&Scope) -> VNode + 'a>,
        stable_scope_addr: Weak<VCompAssociatedScope>,
        root_id: u32,
    },
    PropsChanged {
        caller: Weak<dyn Fn(&Scope) -> VNode + 'a>,
        stable_scope_addr: Weak<VCompAssociatedScope>,
        root_id: u32,
    },
    SameProps {
        caller: Weak<dyn Fn(&Scope) -> VNode + 'a>,
        stable_scope_addr: Weak<VCompAssociatedScope>,
        root_id: u32,
    },
    Replace {
        caller: Weak<dyn Fn(&Scope) -> VNode + 'a>,
        old_scope: Weak<VCompAssociatedScope>,
        new_scope: Weak<VCompAssociatedScope>,
        root_id: u32,
    },
    Remove {
        stable_scope_addr: Weak<VCompAssociatedScope>,
        root_id: u32,
    },
}

static COUNTER: AtomicU32 = AtomicU32::new(1);
fn get_id() -> u32 {
    COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

impl<'a> DiffMachine<'a> {
    pub fn new(vdom: ScopeArena) -> Self {
        Self {
            vdom,
            create_diffs: true,
            lifecycle_events: VecDeque::new(),
            change_list: EditMachine::new(),
            diffed: FxHashSet::default(),
        }
    }

    pub fn consume(self) -> EditList<'a> {
        self.change_list.emitter
    }

    pub fn diff_node(&mut self, old: &VNode<'a>, new: &VNode<'a>) {
        // pub fn diff_node(&mut self, old: &VNode<'a>, new: &VNode<'a>) {
        /*
        For each valid case, we "commit traversal", meaning we save this current position in the tree.
        Then, we diff and queue an edit event (via chagelist). s single trees - when components show up, we save that traversal and then re-enter later.
        When re-entering, we reuse the EditList in DiffState
        */
        match (old, new) {
            (VNode::Text(VText { text: old_text }), VNode::Text(VText { text: new_text })) => {
                if old_text != new_text {
                    self.change_list.commit_traversal();
                    self.change_list.set_text(new_text);
                }
            }

            (VNode::Text(_), VNode::Element(_)) => {
                self.change_list.commit_traversal();
                self.create(new);
                self.change_list.replace_with();
            }

            (VNode::Element(_), VNode::Text(_)) => {
                self.change_list.commit_traversal();
                self.create(new);
                self.change_list.replace_with();
            }

            (VNode::Element(eold), VNode::Element(enew)) => {
                // If the element type is completely different, the element needs to be re-rendered completely
                if enew.tag_name != eold.tag_name || enew.namespace != eold.namespace {
                    self.change_list.commit_traversal();
                    self.change_list.replace_with();
                    return;
                }

                self.diff_listeners(eold.listeners, enew.listeners);
                self.diff_attr(eold.attributes, enew.attributes, enew.namespace.is_some());
                self.diff_children(eold.children, enew.children);
            }

            (VNode::Component(cold), VNode::Component(cnew)) => {
                // todo!("should not happen")
                // self.change_list.commit_traversal();
                if cold.user_fc == cnew.user_fc {
                    // todo: create a stable addr
                    let caller = Rc::downgrade(&cnew.caller);
                    let id = cold.stable_addr.borrow().unwrap();
                    *cnew.stable_addr.borrow_mut() = Some(id);
                    *cnew.ass_scope.borrow_mut() = *cold.ass_scope.borrow();

                    let scope = Rc::downgrade(&cold.ass_scope);
                    self.lifecycle_events
                        .push_back(LifeCycleEvent::PropsChanged {
                            caller,
                            root_id: id,
                            stable_scope_addr: scope,
                        });
                } else {
                    let caller = Rc::downgrade(&cnew.caller);
                    let id = cold.stable_addr.borrow().unwrap();
                    let old_scope = Rc::downgrade(&cold.ass_scope);
                    let new_scope = Rc::downgrade(&cnew.ass_scope);

                    self.lifecycle_events.push_back(LifeCycleEvent::Replace {
                        caller,
                        root_id: id,
                        old_scope,
                        new_scope,
                    });
                }
            }

            // todo: knock out any listeners
            (_, VNode::Component(_new)) => {
                self.change_list.commit_traversal();
            }

            (VNode::Component(_old), _) => {
                todo!("Usage of component VNode not currently supported");
            }

            (VNode::Suspended, _) | (_, VNode::Suspended) => {
                todo!("Suspended components not currently available")
            }

            (VNode::Fragment(_), VNode::Fragment(_)) => {
                todo!("Fragments not currently supported in diffing")
            }
            (_, VNode::Fragment(_)) => todo!("Fragments not currently supported in diffing"),
            (VNode::Fragment(_), _) => todo!("Fragments not currently supported in diffing"),
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
        debug_assert!(self.change_list.traversal_is_committed());
        match node {
            VNode::Text(VText { text }) => {
                self.change_list.create_text_node(text);
            }
            VNode::Element(&VElement {
                key: _,
                tag_name,
                listeners,
                attributes,
                children,
                namespace,
            }) => {
                // log::info!("Creating {:#?}", node);
                if let Some(namespace) = namespace {
                    self.change_list.create_element_ns(tag_name, namespace);
                } else {
                    self.change_list.create_element(tag_name);
                }

                listeners.iter().enumerate().for_each(|(_id, listener)| {
                    self.change_list
                        .new_event_listener(listener.event, listener.scope, listener.id)
                });

                for attr in attributes {
                    self.change_list
                        .set_attribute(&attr.name, &attr.value, namespace.is_some());
                }

                // Fast path: if there is a single text child, it is faster to
                // create-and-append the text node all at once via setting the
                // parent's `textContent` in a single change list instruction than
                // to emit three instructions to (1) create a text node, (2) set its
                // text content, and finally (3) append the text node to this
                // parent.
                if children.len() == 1 {
                    if let VNode::Text(VText { text }) = children[0] {
                        self.change_list.set_text(text);
                        return;
                    }
                }

                for child in children {
                    self.create(child);
                    self.change_list.append_child();
                }
            }

            /*
            todo: integrate re-entrace
            */
            VNode::Component(component) => {
                self.change_list
                    .create_text_node("placeholder for vcomponent");

                let id = get_id();
                *component.stable_addr.as_ref().borrow_mut() = Some(id);
                self.change_list.save_known_root(id);
                let scope = Rc::downgrade(&component.ass_scope);
                self.lifecycle_events.push_back(LifeCycleEvent::Mount {
                    caller: Rc::downgrade(&component.caller),
                    root_id: id,
                    stable_scope_addr: scope,
                });
            }
            VNode::Suspended => {
                todo!("Creation of VNode::Suspended not yet supported")
            }
            VNode::Fragment(frag) => {
                //
                todo!("Cannot current create fragments")
            }
        }
    }

    // Diff event listeners between `old` and `new`.
    //
    // The listeners' node must be on top of the change list stack:
    //
    //     [... node]
    //
    // The change list stack is left unchanged.
    fn diff_listeners(&mut self, old: &[Listener<'_>], new: &[Listener<'_>]) {
        // fn diff_listeners(&mut self, old: &[Listener<'a>], new: &[Listener<'a>]) {
        if !old.is_empty() || !new.is_empty() {
            self.change_list.commit_traversal();
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
                        self.change_list.remove_event_listener(event_type);
                        self.change_list
                            .update_event_listener(event_type, new_l.scope, new_l.id)
                    }

                    // if let Some(scope) = self.current_idx {
                    //     let cb = CbIdx::from_gi_index(scope, l_idx);
                    // self.change_list
                    //     .update_event_listener(event_type, new_l.scope, new_l.id);
                    // }

                    continue 'outer1;
                }
            }

            // if let Some(scope) = self.current_idx {
            // let cb = CbIdx::from_gi_index(scope, l_idx);
            self.change_list
                .new_event_listener(event_type, new_l.scope, new_l.id);
            // }
        }

        'outer2: for old_l in old {
            // registry.remove(old_l);

            for new_l in new {
                if new_l.event == old_l.event {
                    continue 'outer2;
                }
            }
            self.change_list.remove_event_listener(old_l.event);
        }
    }

    // Diff a node's attributes.
    //
    // The attributes' node must be on top of the change list stack:
    //
    //     [... node]
    //
    // The change list stack is left unchanged.
    fn diff_attr(
        &mut self,
        old: &'a [Attribute<'a>],
        new: &'a [Attribute<'a>],
        is_namespaced: bool,
    ) {
        // Do O(n^2) passes to add/update and remove attributes, since
        // there are almost always very few attributes.
        'outer: for new_attr in new {
            if new_attr.is_volatile() {
                self.change_list.commit_traversal();
                self.change_list
                    .set_attribute(new_attr.name, new_attr.value, is_namespaced);
            } else {
                for old_attr in old {
                    if old_attr.name == new_attr.name {
                        if old_attr.value != new_attr.value {
                            self.change_list.commit_traversal();
                            self.change_list.set_attribute(
                                new_attr.name,
                                new_attr.value,
                                is_namespaced,
                            );
                        }
                        continue 'outer;
                    }
                }

                self.change_list.commit_traversal();
                self.change_list
                    .set_attribute(new_attr.name, new_attr.value, is_namespaced);
            }
        }

        'outer2: for old_attr in old {
            for new_attr in new {
                if old_attr.name == new_attr.name {
                    continue 'outer2;
                }
            }

            self.change_list.commit_traversal();
            self.change_list.remove_attribute(old_attr.name);
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
                self.change_list.commit_traversal();
                self.remove_all_children(old);
            }
            return;
        }

        if new.len() == 1 {
            match (old.first(), &new[0]) {
                (
                    Some(&VNode::Text(VText { text: old_text })),
                    &VNode::Text(VText { text: new_text }),
                ) if old_text == new_text => {
                    // Don't take this fast path...
                }

                (_, &VNode::Text(VText { text })) => {
                    self.change_list.commit_traversal();
                    self.change_list.set_text(text);
                    // for o in old {
                    //     registry.remove_subtree(o);
                    // }
                    return;
                }

                (_, _) => {}
            }
        }

        if old.is_empty() {
            if !new.is_empty() {
                self.change_list.commit_traversal();
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
            let t = self.change_list.next_temporary();
            self.change_list.set_next_temporary(t);
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
    fn diff_keyed_children(&mut self, old: &[VNode<'a>], new: &[VNode<'a>]) {
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
    fn diff_keyed_prefix(&mut self, old: &[VNode<'a>], new: &[VNode<'a>]) -> KeyedPrefixResult {
        self.change_list.go_down();
        let mut shared_prefix_count = 0;

        for (i, (old, new)) in old.iter().zip(new.iter()).enumerate() {
            if old.key() != new.key() {
                break;
            }

            self.change_list.go_to_sibling(i);

            self.diff_node(old, new);

            shared_prefix_count += 1;
        }

        // If that was all of the old children, then create and append the remaining
        // new children and we're finished.
        if shared_prefix_count == old.len() {
            self.change_list.go_up();
            self.change_list.commit_traversal();
            self.create_and_append_children(&new[shared_prefix_count..]);
            return KeyedPrefixResult::Finished;
        }

        // And if that was all of the new children, then remove all of the remaining
        // old children and we're finished.
        if shared_prefix_count == new.len() {
            self.change_list.go_to_sibling(shared_prefix_count);
            self.change_list.commit_traversal();
            self.remove_self_and_next_siblings(&old[shared_prefix_count..]);
            return KeyedPrefixResult::Finished;
        }

        self.change_list.go_up();
        KeyedPrefixResult::MoreWorkToDo(shared_prefix_count)
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
        &mut self,
        old: &[VNode<'a>],
        mut new: &[VNode<'a>],
        shared_prefix_count: usize,
        shared_suffix_count: usize,
        old_shared_suffix_start: usize,
    ) {
        // Should have already diffed the shared-key prefixes and suffixes.
        debug_assert_ne!(new.first().map(|n| n.key()), old.first().map(|o| o.key()));
        debug_assert_ne!(new.last().map(|n| n.key()), old.last().map(|o| o.key()));

        // The algorithm below relies upon using `u32::MAX` as a sentinel
        // value, so if we have that many new nodes, it won't work. This
        // check is a bit academic (hence only enabled in debug), since
        // wasm32 doesn't have enough address space to hold that many nodes
        // in memory.
        debug_assert!(new.len() < u32::MAX as usize);

        // Map from each `old` node's key to its index within `old`.
        let mut old_key_to_old_index = FxHashMap::default();
        old_key_to_old_index.reserve(old.len());
        old_key_to_old_index.extend(old.iter().enumerate().map(|(i, o)| (o.key(), i)));

        // The set of shared keys between `new` and `old`.
        let mut shared_keys = FxHashSet::default();
        // Map from each index in `new` to the index of the node in `old` that
        // has the same key.
        let mut new_index_to_old_index = Vec::with_capacity(new.len());
        new_index_to_old_index.extend(new.iter().map(|n| {
            let key = n.key();
            if let Some(&i) = old_key_to_old_index.get(&key) {
                shared_keys.insert(key);
                i
            } else {
                u32::MAX as usize
            }
        }));

        // If none of the old keys are reused by the new children, then we
        // remove all the remaining old children and create the new children
        // afresh.
        if shared_suffix_count == 0 && shared_keys.is_empty() {
            if shared_prefix_count == 0 {
                self.change_list.commit_traversal();
                self.remove_all_children(old);
            } else {
                self.change_list.go_down_to_child(shared_prefix_count);
                self.change_list.commit_traversal();
                self.remove_self_and_next_siblings(&old[shared_prefix_count..]);
            }

            self.create_and_append_children(new);

            return;
        }

        // Save each of the old children whose keys are reused in the new
        // children.
        let mut old_index_to_temp = vec![u32::MAX; old.len()];
        let mut start = 0;
        loop {
            let end = (start..old.len())
                .find(|&i| {
                    let key = old[i].key();
                    !shared_keys.contains(&key)
                })
                .unwrap_or(old.len());

            if end - start > 0 {
                self.change_list.commit_traversal();
                let mut t = self.change_list.save_children_to_temporaries(
                    shared_prefix_count + start,
                    shared_prefix_count + end,
                );
                for i in start..end {
                    old_index_to_temp[i] = t;
                    t += 1;
                }
            }

            debug_assert!(end <= old.len());
            if end == old.len() {
                break;
            } else {
                start = end + 1;
            }
        }

        // Remove any old children whose keys were not reused in the new
        // children. Remove from the end first so that we don't mess up indices.
        let mut removed_count = 0;
        for (i, old_child) in old.iter().enumerate().rev() {
            if !shared_keys.contains(&old_child.key()) {
                // registry.remove_subtree(old_child);
                // todo
                self.change_list.commit_traversal();
                self.change_list.remove_child(i + shared_prefix_count);
                removed_count += 1;
            }
        }

        // If there aren't any more new children, then we are done!
        if new.is_empty() {
            return;
        }

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

        // Now we will iterate from the end of the new children back to the
        // beginning, diffing old children we are reusing and if they aren't in the
        // LIS moving them to their new destination, or creating new children. Note
        // that iterating in reverse order lets us use `Node.prototype.insertBefore`
        // to move/insert children.
        //
        // But first, we ensure that we have a child on the change list stack that
        // we can `insertBefore`. We handle this once before looping over `new`
        // children, so that we don't have to keep checking on every loop iteration.
        if shared_suffix_count > 0 {
            // There is a shared suffix after these middle children. We will be
            // inserting before that shared suffix, so add the first child of that
            // shared suffix to the change list stack.
            //
            // [... parent]
            self.change_list
                .go_down_to_child(old_shared_suffix_start - removed_count);
        // [... parent first_child_of_shared_suffix]
        } else {
            // There is no shared suffix coming after these middle children.
            // Therefore we have to process the last child in `new` and move it to
            // the end of the parent's children if it isn't already there.
            let last_index = new.len() - 1;
            // uhhhh why an unwrap?
            let last = new.last().unwrap();
            // let last = new.last().unwrap_throw();
            new = &new[..new.len() - 1];
            if shared_keys.contains(&last.key()) {
                let old_index = new_index_to_old_index[last_index];
                let temp = old_index_to_temp[old_index];
                // [... parent]
                self.change_list.go_down_to_temp_child(temp);
                // [... parent last]
                self.diff_node(&old[old_index], last);

                if new_index_is_in_lis.contains(&last_index) {
                    // Don't move it, since it is already where it needs to be.
                } else {
                    self.change_list.commit_traversal();
                    // [... parent last]
                    self.change_list.append_child();
                    // [... parent]
                    self.change_list.go_down_to_temp_child(temp);
                    // [... parent last]
                }
            } else {
                self.change_list.commit_traversal();
                // [... parent]
                self.create(last);

                // [... parent last]
                self.change_list.append_child();
                // [... parent]
                self.change_list.go_down_to_reverse_child(0);
                // [... parent last]
            }
        }

        for (new_index, new_child) in new.iter().enumerate().rev() {
            let old_index = new_index_to_old_index[new_index];
            if old_index == u32::MAX as usize {
                debug_assert!(!shared_keys.contains(&new_child.key()));
                self.change_list.commit_traversal();
                // [... parent successor]
                self.create(new_child);
                // [... parent successor new_child]
                self.change_list.insert_before();
            // [... parent new_child]
            } else {
                debug_assert!(shared_keys.contains(&new_child.key()));
                let temp = old_index_to_temp[old_index];
                debug_assert_ne!(temp, u32::MAX);

                if new_index_is_in_lis.contains(&new_index) {
                    // [... parent successor]
                    self.change_list.go_to_temp_sibling(temp);
                // [... parent new_child]
                } else {
                    self.change_list.commit_traversal();
                    // [... parent successor]
                    self.change_list.push_temporary(temp);
                    // [... parent successor new_child]
                    self.change_list.insert_before();
                    // [... parent new_child]
                }

                self.diff_node(&old[old_index], new_child);
            }
        }

        // [... parent child]
        self.change_list.go_up();
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
        &mut self,
        old: &[VNode<'a>],
        new: &[VNode<'a>],
        new_shared_suffix_start: usize,
    ) {
        debug_assert_eq!(old.len(), new.len());
        debug_assert!(!old.is_empty());

        // [... parent]
        self.change_list.go_down();
        // [... parent new_child]

        for (i, (old_child, new_child)) in old.iter().zip(new.iter()).enumerate() {
            self.change_list.go_to_sibling(new_shared_suffix_start + i);
            self.diff_node(old_child, new_child);
        }

        // [... parent]
        self.change_list.go_up();
    }

    // Diff children that are not keyed.
    //
    // The parent must be on the top of the change list stack when entering this
    // function:
    //
    //     [... parent]
    //
    // the change list stack is in the same state when this function returns.
    fn diff_non_keyed_children(&mut self, old: &'a [VNode<'a>], new: &'a [VNode<'a>]) {
        // Handled these cases in `diff_children` before calling this function.
        debug_assert!(!new.is_empty());
        debug_assert!(!old.is_empty());

        //     [... parent]
        self.change_list.go_down();
        //     [... parent child]

        for (i, (new_child, old_child)) in new.iter().zip(old.iter()).enumerate() {
            // [... parent prev_child]
            self.change_list.go_to_sibling(i);
            // [... parent this_child]
            self.diff_node(old_child, new_child);
        }

        match old.len().cmp(&new.len()) {
            // old.len > new.len -> removing some nodes
            Ordering::Greater => {
                // [... parent prev_child]
                self.change_list.go_to_sibling(new.len());
                // [... parent first_child_to_remove]
                self.change_list.commit_traversal();
                // support::remove_self_and_next_siblings(state, &old[new.len()..]);
                self.remove_self_and_next_siblings(&old[new.len()..]);
                // [... parent]
            }
            // old.len < new.len -> adding some nodes
            Ordering::Less => {
                // [... parent last_child]
                self.change_list.go_up();
                // [... parent]
                self.change_list.commit_traversal();
                self.create_and_append_children(&new[old.len()..]);
            }
            // old.len == new.len -> no nodes added/removed, but πerhaps changed
            Ordering::Equal => {
                // [... parent child]
                self.change_list.go_up();
                // [... parent]
            }
        }
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
        debug_assert!(self.change_list.traversal_is_committed());
        log::debug!("REMOVING CHILDREN");
        for _child in old {
            // registry.remove_subtree(child);
        }
        // Fast way to remove all children: set the node's textContent to an empty
        // string.
        self.change_list.set_text("");
    }

    // Create the given children and append them to the parent node.
    //
    // The parent node must currently be on top of the change list stack:
    //
    //     [... parent]
    //
    // When this function returns, the change list stack is in the same state.
    pub fn create_and_append_children(&mut self, new: &[VNode<'a>]) {
        debug_assert!(self.change_list.traversal_is_committed());
        for child in new {
            self.create(child);
            self.change_list.append_child();
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
    pub fn remove_self_and_next_siblings(&mut self, old: &[VNode<'a>]) {
        debug_assert!(self.change_list.traversal_is_committed());
        for child in old {
            if let VNode::Component(vcomp) = child {
                // self.change_list
                //     .create_text_node("placeholder for vcomponent");

                let root_id = vcomp.stable_addr.as_ref().borrow().unwrap();
                self.lifecycle_events.push_back(LifeCycleEvent::Remove {
                    root_id,
                    stable_scope_addr: Rc::downgrade(&vcomp.ass_scope),
                })
                // let id = get_id();
                // *component.stable_addr.as_ref().borrow_mut() = Some(id);
                // self.change_list.save_known_root(id);
                // let scope = Rc::downgrade(&component.ass_scope);
                // self.lifecycle_events.push_back(LifeCycleEvent::Mount {
                //     caller: Rc::downgrade(&component.caller),
                //     root_id: id,
                //     stable_scope_addr: scope,
                // });
            }

            // registry.remove_subtree(child);
        }
        self.change_list.remove_self_and_next_siblings();
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
