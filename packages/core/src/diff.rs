//! This module contains the stateful DiffMachine and all methods to diff VNodes, their properties, and their children.
//! The DiffMachine calculates the diffs between the old and new frames, updates the new nodes, and modifies the real dom.
//!
//! ## Notice:
//! The inspiration and code for this module was originally taken from Dodrio (@fitzgen) and then modified to support
//! Components, Fragments, Suspense, SubTree memoization, and additional batching operations.
//!
//! ## Implementation Details:
//!
//! ### IDs for elements
//! --------------------
//! All nodes are addressed by their IDs. The RealDom provides an imperative interface for making changes to these nodes.
//! We don't necessarily require that DOM changes happen instnatly during the diffing process, so the implementor may choose
//! to batch nodes if it is more performant for their application. The expectation is that renderers use a Slotmap for nodes
//! whose keys can be converted to u64 on FFI boundaries.
//!
//! When new nodes are created through `render`, they won't know which real node they correspond to. During diffing, we
//! always make sure to copy over the ID. If we don't do this properly, the ElementId will be populated incorrectly and
//! brick the user's page.
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
//! track nodes created in a scope and clean up all relevant data. Support for this is currently WIP
//!
//! ## Bloom Filter and Heuristics
//! ------------------------------
//! For all components, we employ some basic heuristics to speed up allocations and pre-size bump arenas. The heuristics are
//! currently very rough, but will get better as time goes on. For FFI, we recommend using a bloom filter to cache strings.
//!
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
//! Further Reading and Thoughts
//! ----------------------------
//! There are more ways of increasing diff performance here that are currently not implemented.
//! More info on how to improve this diffing algorithm:
//!  - https://hacks.mozilla.org/2019/03/fast-bump-allocated-virtual-doms-with-rust-and-wasm/

use crate::{arena::SharedResources, innerlude::*};
use fxhash::{FxHashMap, FxHashSet};
use smallvec::{smallvec, SmallVec};

use std::{any::Any, cell::Cell, cmp::Ordering};
use DomEdit::*;

/// Instead of having handles directly over nodes, Dioxus uses simple u32 as node IDs.
/// The expectation is that the underlying renderer will mainain their Nodes in vec where the ids are the index. This allows
/// for a form of passive garbage collection where nodes aren't immedately cleaned up.
///
/// The "RealDom" abstracts over the... real dom. The RealDom trait assumes that the renderer maintains a stack of real
/// nodes as the diffing algorithm descenes through the tree. This means that whatever is on top of the stack will receive
/// any modifications that follow. This technique enables the diffing algorithm to avoid directly handling or storing any
/// target-specific Node type as well as easily serializing the edits to be sent over a network or IPC connection.
pub trait RealDom<'a> {
    fn raw_node_as_any(&self) -> &mut dyn Any;
}

pub struct DiffMachine<'real, 'bump> {
    pub real_dom: &'real dyn RealDom<'bump>,

    pub vdom: &'bump SharedResources,

    pub edits: &'real mut Vec<DomEdit<'bump>>,

    pub scheduled_garbage: Vec<&'bump VNode<'bump>>,

    pub cur_idxs: SmallVec<[ScopeId; 5]>,

    pub diffed: FxHashSet<ScopeId>,

    pub seen_nodes: FxHashSet<ScopeId>,
}

impl<'real, 'bump> DiffMachine<'real, 'bump> {
    pub fn new(
        edits: &'real mut Vec<DomEdit<'bump>>,
        dom: &'real dyn RealDom<'bump>,
        cur_idx: ScopeId,
        shared: &'bump SharedResources,
    ) -> Self {
        Self {
            real_dom: dom,
            edits,
            cur_idxs: smallvec![cur_idx],
            vdom: shared,
            scheduled_garbage: vec![],
            diffed: FxHashSet::default(),
            seen_nodes: FxHashSet::default(),
        }
    }

    // Diff the `old` node with the `new` node. Emits instructions to modify a
    // physical DOM node that reflects `old` into something that reflects `new`.
    //
    // the real stack should be what it is coming in and out of this function (ideally empty)
    //
    // each function call assumes the stack is fresh (empty).
    pub fn diff_node(&mut self, old_node: &'bump VNode<'bump>, new_node: &'bump VNode<'bump>) {
        let root = old_node.dom_id.get();

        match (&old_node.kind, &new_node.kind) {
            // Handle the "sane" cases first.
            // The rsx and html macros strongly discourage dynamic lists not encapsulated by a "Fragment".
            // So the sane (and fast!) cases are where the virtual structure stays the same and is easily diffable.
            (VNodeKind::Text(old), VNodeKind::Text(new)) => {
                let root = root.unwrap();

                if old.text != new.text {
                    self.push_root(root);
                    self.set_text(new.text);
                    self.pop();
                }

                new_node.dom_id.set(Some(root));
            }

            (VNodeKind::Element(old), VNodeKind::Element(new)) => {
                let root = root.unwrap();

                // If the element type is completely different, the element needs to be re-rendered completely
                // This is an optimization React makes due to how users structure their code
                //
                // This case is rather rare (typically only in non-keyed lists)
                if new.tag_name != old.tag_name || new.namespace != old.namespace {
                    self.push_root(root);
                    let meta = self.create(new_node);
                    self.replace_with(meta.added_to_stack);
                    self.scheduled_garbage.push(old_node);
                    self.pop();
                    return;
                }

                new_node.dom_id.set(Some(root));

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
                            please_commit(&mut self.edits);
                            self.set_attribute(new_attr);
                        }
                    }
                } else {
                    // TODO: provide some sort of report on how "good" the diffing was
                    please_commit(&mut self.edits);
                    for attribute in old.attributes {
                        self.remove_attribute(attribute);
                    }
                    for attribute in new.attributes {
                        self.set_attribute(attribute)
                    }
                }

                // Diff listeners
                //
                // It's extraordinarily rare to have the number/order of listeners change
                // In the cases where the listeners change, we completely wipe the data attributes and add new ones
                //
                // TODO: take a more efficient path than this
                if old.listeners.len() == new.listeners.len() {
                    for (old_l, new_l) in old.listeners.iter().zip(new.listeners.iter()) {
                        if old_l.event != new_l.event {
                            please_commit(&mut self.edits);
                            self.remove_event_listener(old_l.event);
                            self.new_event_listener(new_l);
                        }
                        new_l.mounted_node.set(old_l.mounted_node.get());
                    }
                } else {
                    please_commit(&mut self.edits);
                    for listener in old.listeners {
                        self.remove_event_listener(listener.event);
                    }
                    for listener in new.listeners {
                        listener.mounted_node.set(Some(root));
                        self.new_event_listener(listener);
                    }
                }

                if has_comitted {
                    self.pop();
                }

                // Each child pushes its own root, so it doesn't need our current root

                self.diff_children(old.children, new.children, None, &mut None);
            }

            (VNodeKind::Component(old), VNodeKind::Component(new)) => {
                let scope_addr = old.ass_scope.get().unwrap();

                // Make sure we're dealing with the same component (by function pointer)
                if old.user_fc == new.user_fc {
                    //
                    self.cur_idxs.push(scope_addr);

                    // Make sure the new component vnode is referencing the right scope id
                    new.ass_scope.set(Some(scope_addr));

                    // make sure the component's caller function is up to date
                    let scope = self.get_scope_mut(&scope_addr).unwrap();

                    scope
                        .update_scope_dependencies(new.caller.clone(), ScopeChildren(new.children));

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

                    self.cur_idxs.pop();

                    self.seen_nodes.insert(scope_addr);
                } else {
                    let mut old_iter = RealChildIterator::new(old_node, &self.vdom);
                    let first = old_iter
                        .next()
                        .expect("Components should generate a placeholder root");

                    // remove any leftovers
                    for to_remove in old_iter {
                        self.push_root(to_remove.element_id().unwrap());
                        self.remove();
                    }

                    // seems like we could combine this into a single instruction....
                    self.push_root(first.element_id().unwrap());
                    let meta = self.create(new_node);
                    self.replace_with(meta.added_to_stack);
                    self.scheduled_garbage.push(old_node);
                    self.pop();

                    // Wipe the old one and plant the new one
                    let old_scope = old.ass_scope.get().unwrap();
                    self.destroy_scopes(old_scope);
                }
            }

            (VNodeKind::Fragment(old), VNodeKind::Fragment(new)) => {
                // This is the case where options or direct vnodes might be used.
                // In this case, it's faster to just skip ahead to their diff
                if old.children.len() == 1 && new.children.len() == 1 {
                    self.diff_node(&old.children[0], &new.children[0]);
                    return;
                }

                let mut new_anchor = None;
                self.diff_children(
                    old.children,
                    new.children,
                    old_node.dom_id.get(),
                    &mut new_anchor,
                );
                if let Some(anchor) = new_anchor {
                    new_node.dom_id.set(Some(anchor));
                }
            }

            // The strategy here is to pick the first possible node from the previous set and use that as our replace with root
            //
            // We also walk the "real node" list to make sure all latent roots are claened up
            // This covers the case any time a fragment or component shows up with pretty much anything else
            //
            // This likely isn't the fastest way to go about replacing one node with a virtual node, but the "insane" cases
            // are pretty rare.  IE replacing a list (component or fragment) with a single node.
            (
                VNodeKind::Component(_)
                | VNodeKind::Fragment(_)
                | VNodeKind::Text(_)
                | VNodeKind::Element(_),
                VNodeKind::Component(_)
                | VNodeKind::Fragment(_)
                | VNodeKind::Text(_)
                | VNodeKind::Element(_),
            ) => {
                log::info!(
                    "taking the awkard diffing path {:#?}, {:#?}",
                    old_node,
                    new_node
                );
                // currently busted for components - need to fid
                let root = old_node.dom_id.get().expect(&format!(
                    "Should not be diffing old nodes that were never assigned, {:#?}",
                    old_node
                ));
                // Choose the node to use as the placeholder for replacewith
                let back_node_id = match old_node.kind {
                    // We special case these two types to avoid allocating the small-vecs
                    VNodeKind::Element(_) | VNodeKind::Text(_) => root,

                    _ => {
                        let mut old_iter = RealChildIterator::new(old_node, &self.vdom);

                        let back_node = old_iter
                            .next()
                            .expect("Empty fragments should generate a placeholder.");

                        // remove any leftovers
                        for to_remove in old_iter {
                            self.push_root(to_remove.element_id().unwrap());
                            self.remove();
                        }

                        back_node.element_id().unwrap()
                    }
                };

                // replace the placeholder or first node with the nodes generated from the "new"
                self.push_root(back_node_id);
                let meta = self.create(new_node);
                self.replace_with(meta.added_to_stack);

                // todo use the is_static metadata to update this subtree
            }

            // TODO
            (VNodeKind::Suspended { node }, new) => todo!(),
            (old, VNodeKind::Suspended { .. }) => {
                // a node that was once real is now suspended
                //
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
    pub fn create(&mut self, node: &'bump VNode<'bump>) -> CreateMeta {
        log::warn!("Creating node! ... {:#?}", node);
        match &node.kind {
            VNodeKind::Text(text) => {
                let real_id = self.vdom.reserve_node();
                self.create_text_node(text.text, real_id);
                node.dom_id.set(Some(real_id));

                CreateMeta::new(text.is_static, 1)
            }
            VNodeKind::Element(el) => {
                // we have the potential to completely eliminate working on this node in the future(!)
                //
                // This can only be done if all of the elements properties (attrs, children, listeners, etc) are static
                // While creating these things, keep track if we can memoize this element.
                // At the end, we'll set this flag on the element to skip it
                let mut is_static: bool = true;

                let VElement {
                    tag_name,
                    listeners,
                    attributes,
                    children,
                    namespace,
                    static_attrs: _,
                    static_children: _,
                    static_listeners: _,
                } = el;

                let real_id = self.vdom.reserve_node();
                if let Some(namespace) = namespace {
                    self.create_element(tag_name, Some(namespace), real_id)
                } else {
                    self.create_element(tag_name, None, real_id)
                };
                node.dom_id.set(Some(real_id));

                listeners.iter().for_each(|listener| {
                    log::info!("setting listener id to {:#?}", real_id);
                    listener.mounted_node.set(Some(real_id));
                    self.new_event_listener(listener);

                    // if the node has an event listener, then it must be visited ?
                    is_static = false;
                });

                for attr in *attributes {
                    is_static = is_static && attr.is_static;
                    self.set_attribute(attr);
                }

                // Fast path: if there is a single text child, it is faster to
                // create-and-append the text node all at once via setting the
                // parent's `textContent` in a single change list instruction than
                // to emit three instructions to (1) create a text node, (2) set its
                // text content, and finally (3) append the text node to this
                // parent.
                //
                // Notice: this is a web-specific optimization and may be changed in the future
                //
                // TODO move over
                // if children.len() == 1 {
                //     if let VNodeKind::Text(text) = &children[0].kind {
                //         self.set_text(text.text);
                //         return CreateMeta::new(is_static, 1);
                //     }
                // }

                for child in *children {
                    let child_meta = self.create(child);
                    is_static = is_static && child_meta.is_static;

                    // append whatever children were generated by this call
                    self.append_children(child_meta.added_to_stack);
                }

                // if is_static {
                //     log::debug!("created a static node {:#?}", node);
                // } else {
                //     log::debug!("created a dynamic node {:#?}", node);
                // }

                // el_is_static.set(is_static);
                CreateMeta::new(is_static, 1)
            }

            VNodeKind::Component(vcomponent) => {
                log::debug!("Mounting a new component");
                let caller = vcomponent.caller.clone();

                let parent_idx = self.cur_idxs.last().unwrap().clone();

                // Insert a new scope into our component list
                let new_idx = self.vdom.insert_scope_with_key(|new_idx| {
                    let parent_scope = self.get_scope(&parent_idx).unwrap();
                    let height = parent_scope.height + 1;
                    Scope::new(
                        caller,
                        new_idx,
                        Some(parent_idx),
                        height,
                        ScopeChildren(vcomponent.children),
                        self.vdom.clone(),
                    )
                });

                // This code is supposed to insert the new idx into the parent's descendent list, but it doesn't really work.
                // This is mostly used for cleanup - to remove old scopes when components are destroyed.
                // TODO
                //
                // self.components
                //     .try_get_mut(idx)
                //     .unwrap()
                //     .descendents
                //     .borrow_mut()
                //     .insert(idx);

                // TODO: abstract this unsafe into the arena abstraction
                let new_component = self.get_scope_mut(&new_idx).unwrap();

                // Actually initialize the caller's slot with the right address
                vcomponent.ass_scope.set(Some(new_idx));

                // Run the scope for one iteration to initialize it
                new_component.run_scope().unwrap();

                // TODO: we need to delete (IE relcaim this node, otherwise the arena will grow infinitely)
                let nextnode = new_component.frames.fin_head();
                self.cur_idxs.push(new_idx);
                let meta = self.create(nextnode);
                self.cur_idxs.pop();
                node.dom_id.set(nextnode.dom_id.get());

                // Finally, insert this node as a seen node.
                self.seen_nodes.insert(new_idx);

                CreateMeta::new(vcomponent.is_static, meta.added_to_stack)
            }

            // Fragments are the only nodes that can contain dynamic content (IE through curlies or iterators).
            // We can never ignore their contents, so the prescence of a fragment indicates that we need always diff them.
            // Fragments will just put all their nodes onto the stack after creation
            VNodeKind::Fragment(frag) => {
                let mut nodes_added = 0;
                for child in frag.children.iter().rev() {
                    // different types of nodes will generate different amounts on the stack
                    // nested fragments will spew a ton of nodes onto the stack
                    // TODO: make sure that our order (.rev) makes sense in a nested situation
                    let new_meta = self.create(child);
                    nodes_added += new_meta.added_to_stack;
                }
                log::info!("This fragment added {} nodes to the stack", nodes_added);

                // Never ignore
                CreateMeta::new(false, nodes_added)
            }

            VNodeKind::Suspended { node: real_node } => {
                let id = self.vdom.reserve_node();
                self.create_placeholder(id);
                node.dom_id.set(Some(id));
                real_node.set(Some(id));
                CreateMeta::new(false, 1)
            }
        }
    }

    fn create_children(&mut self, children: &'bump [VNode<'bump>]) -> CreateMeta {
        let mut is_static = true;
        let mut added_to_stack = 0;

        for child in children {
            let child_meta = self.create(child);
            is_static = is_static && child_meta.is_static;
            added_to_stack += child_meta.added_to_stack;
        }

        CreateMeta {
            is_static,
            added_to_stack,
        }
    }

    pub fn replace_vnode(&mut self, old_node: &'bump VNode<'bump>, new_node: &'bump VNode<'bump>) {}

    /// Destroy a scope and all of its descendents.
    ///
    /// Calling this will run the destuctors on all hooks in the tree.
    /// It will also add the destroyed nodes to the `seen_nodes` cache to prevent them from being renderered.
    fn destroy_scopes(&mut self, old_scope: ScopeId) {
        let mut nodes_to_delete = vec![old_scope];
        let mut scopes_to_explore = vec![old_scope];

        // explore the scope tree breadth first
        while let Some(scope_id) = scopes_to_explore.pop() {
            // If we're planning on deleting this node, then we don't need to both rendering it
            self.seen_nodes.insert(scope_id);
            let scope = self.get_scope(&scope_id).unwrap();
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
            let _scope = self.vdom.try_remove(node).unwrap();
            // do anything we need to do to delete the scope
            // I think we need to run the destructors on the hooks
            // TODO
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
    //
    // If old no anchors are provided, then it's assumed that we can freely append to the parent.
    //
    // Remember, non-empty lists does not mean that there are real elements, just that there are virtual elements.
    fn diff_children(
        &mut self,
        old: &'bump [VNode<'bump>],
        new: &'bump [VNode<'bump>],
        old_anchor: Option<ElementId>,
        new_anchor: &mut Option<ElementId>,
    ) {
        const IS_EMPTY: bool = true;
        const IS_NOT_EMPTY: bool = false;

        match (old_anchor, new.is_empty()) {
            // Both are empty, dealing only with potential anchors
            (Some(anchor), IS_EMPTY) => {
                // new_anchor.set(Some(anchor));
                if old.len() > 0 {
                    // clean up these virtual nodes (components, fragments, etc)
                }
            }

            // Completely adding new nodes, removing any placeholder if it exists
            (Some(anchor), IS_NOT_EMPTY) => {
                //
                // match old_anchor {
                // If there's anchor to work from, then we replace it with the new children
                // Some(anchor) => {
                self.push_root(anchor);
                let meta = self.create_children(new);
                if meta.added_to_stack > 0 {
                    self.replace_with(meta.added_to_stack)
                } else {
                    // no items added to the stack... hmmmm....
                    *new_anchor = old_anchor;
                }
                // }

                // If there's no anchor to work with, we just straight up append them
                // None => {
                let meta = self.create_children(new);
                self.append_children(meta.added_to_stack);
                // }
                // }
            }

            // Completely removing old nodes and putting an anchor in its place
            // no anchor (old has nodes) and the new is empty
            // remove all the old nodes
            (None, IS_EMPTY) => {
                // load the first real
                if let Some(to_replace) = find_first_real_node(old, self.vdom) {
                    //
                    self.push_root(to_replace.dom_id.get().unwrap());

                    // Create the anchor
                    let anchor_id = self.vdom.reserve_node();
                    self.create_placeholder(anchor_id);
                    // *new_anchor = Some(anchor_id);

                    // Replace that node
                    self.replace_with(1);
                } else {
                    // no real nodes -
                    // new_anchor.set(old_anchor);
                }

                // remove the rest
                for child in &old[1..] {
                    self.push_root(child.element_id().unwrap());
                    self.remove();
                }
            }

            (None, IS_NOT_EMPTY) => {
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
                    self.diff_keyed_children(old, new);
                } else {
                    self.diff_non_keyed_children(old, new);
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
    // When entering this function, the parent must be on top of the change list
    // stack:
    //
    //     [... parent]
    //
    // Upon exiting, the change list stack is in the same state.
    fn diff_keyed_children(&mut self, old: &'bump [VNode<'bump>], new: &'bump [VNode<'bump>]) {
        if cfg!(debug_assertions) {
            let mut keys = fxhash::FxHashSet::default();
            let mut assert_unique_keys = |children: &'bump [VNode<'bump>]| {
                keys.clear();
                for child in children {
                    let key = child.key;
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
            .take_while(|&(old, new)| old.key == new.key)
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
    fn diff_keyed_prefix(
        &mut self,
        old: &'bump [VNode<'bump>],
        new: &'bump [VNode<'bump>],
    ) -> KeyedPrefixResult {
        // self.go_down();

        let mut shared_prefix_count = 0;

        for (i, (old, new)) in old.iter().zip(new.iter()).enumerate() {
            // abort early if we finally run into nodes with different keys
            if old.key() != new.key() {
                break;
            }

            // self.go_to_sibling(i);

            self.diff_node(old, new);

            shared_prefix_count += 1;
        }

        // If that was all of the old children, then create and append the remaining
        // new children and we're finished.
        if shared_prefix_count == old.len() {
            // self.go_up();
            // self.commit_traversal();
            self.create_and_append_children(&new[shared_prefix_count..]);
            return KeyedPrefixResult::Finished;
        }

        // And if that was all of the new children, then remove all of the remaining
        // old children and we're finished.
        if shared_prefix_count == new.len() {
            // self.go_to_sibling(shared_prefix_count);
            // self.commit_traversal();
            self.remove_self_and_next_siblings(&old[shared_prefix_count..]);
            return KeyedPrefixResult::Finished;
        }
        //
        // self.go_up();
        KeyedPrefixResult::MoreWorkToDo(shared_prefix_count)
    }

    // Remove all of a node's children.
    //
    // The change list stack must have this shape upon entry to this function:
    //
    //     [... parent]
    //
    // When this function returns, the change list stack is in the same state.
    fn remove_all_children(&mut self, old: &'bump [VNode<'bump>]) {
        // debug_assert!(self.traversal_is_committed());
        log::debug!("REMOVING CHILDREN");
        for _child in old {
            // registry.remove_subtree(child);
        }
        // Fast way to remove all children: set the node's textContent to an empty
        // string.
        todo!()
        // self.set_inner_text("");
    }

    // Create the given children and append them to the parent node.
    //
    // The parent node must currently be on top of the change list stack:
    //
    //     [... parent]
    //
    // When this function returns, the change list stack is in the same state.
    pub fn create_and_append_children(&mut self, new: &'bump [VNode<'bump>]) {
        for child in new {
            let meta = self.create(child);
            self.append_children(meta.added_to_stack);
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
    // Upon entry to this function, the change list stack must be:
    //
    //     [... parent]
    //
    // Upon exit from this function, it will be restored to that same state.
    fn diff_keyed_middle(
        &self,
        old: &[VNode<'bump>],
        new: &[VNode<'bump>],
        shared_prefix_count: usize,
        shared_suffix_count: usize,
        old_shared_suffix_start: usize,
    ) {
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
        // // IE if the keys were A B C, then we would have (A, 1) (B, 2) (C, 3).
        // let mut old_key_to_old_index = old
        //     .iter()
        //     .enumerate()
        //     .map(|(i, o)| (o.key(), i))
        //     .collect::<FxHashMap<_, _>>();

        // // The set of shared keys between `new` and `old`.
        // let mut shared_keys = FxHashSet::default();

        // // Map from each index in `new` to the index of the node in `old` that
        // // has the same key.
        // let mut new_index_to_old_index = new
        //     .iter()
        //     .map(|n| {
        //         let key = n.key();
        //         if let Some(&i) = old_key_to_old_index.get(&key) {
        //             shared_keys.insert(key);
        //             i
        //         } else {
        //             u32::MAX as usize
        //         }
        //     })
        //     .collect::<Vec<_>>();

        // // If none of the old keys are reused by the new children, then we
        // // remove all the remaining old children and create the new children
        // // afresh.
        // if shared_suffix_count == 0 && shared_keys.is_empty() {
        //     if shared_prefix_count == 0 {
        //         // self.commit_traversal();
        //         self.remove_all_children(old);
        //     } else {
        //         // self.go_down_to_child(shared_prefix_count);
        //         // self.commit_traversal();
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
        //         // self.commit_traversal();
        //         let mut t = self.save_children_to_temporaries(
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
        //         // self.commit_traversal();
        //         self.remove(old_child.dom_id.get());
        //         self.remove_child(i + shared_prefix_count);
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
        //     self.edits
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
        //         self.go_down_to_temp_child(temp);
        //         // [... parent last]
        //         self.diff_node(&old[old_index], last);

        //         if new_index_is_in_lis.contains(&last_index) {
        //             // Don't move it, since it is already where it needs to be.
        //         } else {
        //             // self.commit_traversal();
        //             // [... parent last]
        //             self.append_child();
        //             // [... parent]
        //             self.go_down_to_temp_child(temp);
        //             // [... parent last]
        //         }
        //     } else {
        //         // self.commit_traversal();
        //         // [... parent]
        //         self.create(last);

        //         // [... parent last]
        //         self.append_child();
        //         // [... parent]
        //         self.go_down_to_reverse_child(0);
        //         // [... parent last]
        //     }
        // }

        // for (new_index, new_child) in new.iter().enumerate().rev() {
        //     let old_index = new_index_to_old_index[new_index];
        //     if old_index == u32::MAX as usize {
        //         debug_assert!(!shared_keys.contains(&new_child.key()));
        //         // self.commit_traversal();
        //         // [... parent successor]
        //         self.create(new_child);
        //         // [... parent successor new_child]
        //         self.insert_before();
        //     // [... parent new_child]
        //     } else {
        //         debug_assert!(shared_keys.contains(&new_child.key()));
        //         let temp = old_index_to_temp[old_index];
        //         debug_assert_ne!(temp, u32::MAX);

        //         if new_index_is_in_lis.contains(&new_index) {
        //             // [... parent successor]
        //             self.go_to_temp_sibling(temp);
        //         // [... parent new_child]
        //         } else {
        //             // self.commit_traversal();
        //             // [... parent successor]
        //             self.push_temporary(temp);
        //             // [... parent successor new_child]
        //             self.insert_before();
        //             // [... parent new_child]
        //         }

        //         self.diff_node(&old[old_index], new_child);
        //     }
        // }
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

        for (i, (old_child, new_child)) in old.iter().zip(new.iter()).enumerate() {
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
        // debug_assert!(!new.is_empty());
        // debug_assert!(!old.is_empty());

        for (_i, (new_child, old_child)) in new.iter().zip(old.iter()).enumerate() {
            self.diff_node(old_child, new_child);
        }

        match old.len().cmp(&new.len()) {
            // old.len > new.len -> removing some nodes
            Ordering::Greater => {
                for item in &old[new.len()..] {
                    for i in RealChildIterator::new(item, self.vdom) {
                        self.push_root(i.element_id().unwrap());
                        self.remove();
                    }
                }
            }
            // old.len < new.len -> adding some nodes
            Ordering::Less => {
                // [... parent last_child]
                // self.go_up();
                // [... parent]
                // self.commit_traversal();
                self.create_and_append_children(&new[old.len()..]);
            }
            // old.len == new.len -> no nodes added/removed, but πerhaps changed
            Ordering::Equal => {}
        }
    }

    // ======================
    // Support methods
    // ======================

    // Remove the current child and all of its following siblings.
    //
    // The change list stack must have this shape upon entry to this function:
    //
    //     [... parent child]
    //
    // After the function returns, the child is no longer on the change list stack:
    //
    //     [... parent]
    fn remove_self_and_next_siblings(&self, old: &[VNode<'bump>]) {
        // debug_assert!(self.traversal_is_committed());
        for child in old {
            if let VNodeKind::Component(_vcomp) = child.kind {
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
                // self.save_known_root(id);
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
        // self.remove_self_and_next_siblings();
    }

    pub fn get_scope_mut(&mut self, id: &ScopeId) -> Option<&'bump mut Scope> {
        // ensure we haven't seen this scope before
        // if we have, then we're trying to alias it, which is not allowed
        debug_assert!(!self.seen_nodes.contains(id));

        unsafe { self.vdom.get_scope_mut(*id) }
    }
    pub fn get_scope(&mut self, id: &ScopeId) -> Option<&'bump Scope> {
        // ensure we haven't seen this scope before
        // if we have, then we're trying to alias it, which is not allowed
        unsafe { self.vdom.get_scope(*id) }
    }

    // Navigation
    pub(crate) fn push_root(&mut self, root: ElementId) {
        let id = root.as_u64();
        self.edits.push(PushRoot { id });
    }

    pub(crate) fn pop(&mut self) {
        self.edits.push(PopRoot {});
    }

    // Add Nodes to the dom
    // add m nodes from the stack
    pub(crate) fn append_children(&mut self, many: u32) {
        self.edits.push(AppendChildren { many });
    }

    // replace the n-m node on the stack with the m nodes
    // ends with the last element of the chain on the top of the stack
    pub(crate) fn replace_with(&mut self, many: u32) {
        self.edits.push(ReplaceWith { many });
    }

    // Remove Nodesfrom the dom
    pub(crate) fn remove(&mut self) {
        self.edits.push(Remove);
    }

    // Create
    pub(crate) fn create_text_node(&mut self, text: &'bump str, id: ElementId) {
        let id = id.as_u64();
        self.edits.push(CreateTextNode { text, id });
    }

    pub(crate) fn create_element(
        &mut self,
        tag: &'static str,
        ns: Option<&'static str>,
        id: ElementId,
    ) {
        let id = id.as_u64();
        match ns {
            Some(ns) => self.edits.push(CreateElementNs { id, ns, tag }),
            None => self.edits.push(CreateElement { id, tag }),
        }
    }

    // placeholders are nodes that don't get rendered but still exist as an "anchor" in the real dom
    pub(crate) fn create_placeholder(&mut self, id: ElementId) {
        let id = id.as_u64();
        self.edits.push(CreatePlaceholder { id });
    }

    // events
    pub(crate) fn new_event_listener(&mut self, listener: &Listener) {
        let Listener {
            event,
            scope,
            mounted_node,
            ..
        } = listener;

        let element_id = mounted_node.get().unwrap().as_u64();

        self.edits.push(NewEventListener {
            scope: scope.clone(),
            event_name: event,
            mounted_node_id: element_id,
        });
    }

    pub(crate) fn remove_event_listener(&mut self, event: &'static str) {
        self.edits.push(RemoveEventListener { event });
    }

    // modify
    pub(crate) fn set_text(&mut self, text: &'bump str) {
        self.edits.push(SetText { text });
    }

    pub(crate) fn set_attribute(&mut self, attribute: &'bump Attribute) {
        let Attribute {
            name,
            value,
            is_static,
            is_volatile,
            namespace,
        } = attribute;
        // field: &'static str,
        // value: &'bump str,
        // ns: Option<&'static str>,
        self.edits.push(SetAttribute {
            field: name,
            value,
            ns: *namespace,
        });
    }

    pub(crate) fn remove_attribute(&mut self, attribute: &Attribute) {
        let name = attribute.name;
        self.edits.push(RemoveAttribute { name });
    }
}

// When we create new nodes, we need to propagate some information back up the call chain.
// This gives the caller some information on how to handle things like insertins, appending, and subtree discarding.
pub struct CreateMeta {
    pub is_static: bool,
    pub added_to_stack: u32,
}

impl CreateMeta {
    fn new(is_static: bool, added_to_tack: u32) -> Self {
        Self {
            is_static,
            added_to_stack: added_to_tack,
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

fn find_first_real_node<'a>(
    nodes: impl IntoIterator<Item = &'a VNode<'a>>,
    scopes: &'a SharedResources,
) -> Option<&'a VNode<'a>> {
    for node in nodes {
        let mut iter = RealChildIterator::new(node, scopes);
        if let Some(node) = iter.next() {
            return Some(node);
        }
    }

    None
}

/// This iterator iterates through a list of virtual children and only returns real children (Elements or Text).
///
/// This iterator is useful when it's important to load the next real root onto the top of the stack for operations like
/// "InsertBefore".
struct RealChildIterator<'a> {
    scopes: &'a SharedResources,

    // Heuristcally we should never bleed into 4 completely nested fragments/components
    // Smallvec lets us stack allocate our little stack machine so the vast majority of cases are sane
    // TODO: use const generics instead of the 4 estimation
    stack: smallvec::SmallVec<[(u16, &'a VNode<'a>); 4]>,
}

impl<'a> RealChildIterator<'a> {
    fn new(starter: &'a VNode<'a>, scopes: &'a SharedResources) -> Self {
        Self {
            scopes,
            stack: smallvec::smallvec![(0, starter)],
        }
    }
}

impl<'a> Iterator for RealChildIterator<'a> {
    type Item = &'a VNode<'a>;

    fn next(&mut self) -> Option<&'a VNode<'a>> {
        let mut should_pop = false;
        let mut returned_node: Option<&'a VNode<'a>> = None;
        let mut should_push = None;

        while returned_node.is_none() {
            if let Some((count, node)) = self.stack.last_mut() {
                match &node.kind {
                    // We can only exit our looping when we get "real" nodes
                    // This includes fragments and components when they're empty (have a single root)
                    VNodeKind::Element(_) | VNodeKind::Text(_) => {
                        // We've recursed INTO an element/text
                        // We need to recurse *out* of it and move forward to the next
                        should_pop = true;
                        returned_node = Some(&*node);
                    }

                    // If we get a fragment we push the next child
                    VNodeKind::Fragment(frag) => {
                        let subcount = *count as usize;

                        if frag.children.len() == 0 {
                            should_pop = true;
                            returned_node = Some(&*node);
                        }

                        if subcount >= frag.children.len() {
                            should_pop = true;
                        } else {
                            should_push = Some(&frag.children[subcount]);
                        }
                    }

                    // Immediately abort suspended nodes - can't do anything with them yet
                    // VNodeKind::Suspended => should_pop = true,
                    VNodeKind::Suspended { node, .. } => {
                        todo!()
                        // *node = node.as_ref().borrow().get().expect("msg");
                    }

                    // For components, we load their root and push them onto the stack
                    VNodeKind::Component(sc) => {
                        let scope =
                            unsafe { self.scopes.get_scope(sc.ass_scope.get().unwrap()) }.unwrap();
                        // let scope = self.scopes.get(sc.ass_scope.get().unwrap()).unwrap();

                        // Simply swap the current node on the stack with the root of the component
                        *node = scope.frames.fin_head();
                    }
                }
            } else {
                // If there's no more items on the stack, we're done!
                return None;
            }

            if should_pop {
                self.stack.pop();
                if let Some((id, _)) = self.stack.last_mut() {
                    *id += 1;
                }
                should_pop = false;
            }

            if let Some(push) = should_push {
                self.stack.push((0, push));
                should_push = None;
            }
        }

        returned_node
    }
}

fn compare_strs(a: &str, b: &str) -> bool {
    // Check by pointer, optimizing for static strs
    if !std::ptr::eq(a, b) {
        // If the pointers are different then check by value
        a == b
    } else {
        true
    }
}
