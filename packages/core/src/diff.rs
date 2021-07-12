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
//! always make sure to copy over the ID. If we don't do this properly, the realdomnode will be populated incorrectly and
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
//! ## Garbage Collection
//! ---------------------
//! We roughly place the role of garbage collection onto the reconciler. Dioxus needs to manage the lifecycle of components
//! but will not spend any time cleaning up old elements. It's the Reconciler's duty to understand which elements need to
//! be cleaned up *after* the diffing is completed. The reconciler should schedule this garbage collection as the absolute
//! lowest priority task, after all edits have been applied.
//!
//!
//! Further Reading and Thoughts
//! ----------------------------
//! There are more ways of increasing diff performance here that are currently not implemented.
//! More info on how to improve this diffing algorithm:
//!  - https://hacks.mozilla.org/2019/03/fast-bump-allocated-virtual-doms-with-rust-and-wasm/

use crate::{arena::SharedArena, innerlude::*, tasks::TaskQueue};
use fxhash::{FxHashMap, FxHashSet};

use std::any::Any;

/// The accompanying "real dom" exposes an imperative API for controlling the UI layout
///
/// Instead of having handles directly over nodes, Dioxus uses simple u64s as node IDs.
/// The expectation is that the underlying renderer will mainain their Nodes in something like slotmap or an ECS memory
/// where indexing is very fast. For reference, the slotmap in the WebSys renderer takes about 3ns to randomly access any
/// node.
///
/// The "RealDom" abstracts over the... real dom. The RealDom trait assumes that the renderer maintains a stack of real
/// nodes as the diffing algorithm descenes through the tree. This means that whatever is on top of the stack will receive
/// any modifications that follow. This technique enables the diffing algorithm to avoid directly handling or storing any
/// target-specific Node type as well as easily serializing the edits to be sent over a network or IPC connection.
pub trait RealDom<'a> {
    // Navigation
    fn push(&mut self, root: RealDomNode);
    fn pop(&mut self);

    // Add Nodes to the dom
    // add m nodes from the stack
    fn append_children(&mut self, many: u32);

    // replace the n-m node on the stack with the m nodes
    // ends with the last element of the chain on the top of the stack
    fn replace_with(&mut self, many: u32);

    // Remove Nodesfrom the dom
    fn remove(&mut self);
    fn remove_all_children(&mut self);

    // Create
    fn create_text_node(&mut self, text: &'a str) -> RealDomNode;
    fn create_element(&mut self, tag: &'static str, ns: Option<&'static str>) -> RealDomNode;

    // placeholders are nodes that don't get rendered but still exist as an "anchor" in the real dom
    fn create_placeholder(&mut self) -> RealDomNode;

    // events
    fn new_event_listener(
        &mut self,
        event: &'static str,
        scope: ScopeIdx,
        element_id: usize,
        realnode: RealDomNode,
    );
    fn remove_event_listener(&mut self, event: &'static str);

    // modify
    fn set_text(&mut self, text: &'a str);
    fn set_attribute(&mut self, name: &'static str, value: &'a str, ns: Option<&'a str>);
    fn remove_attribute(&mut self, name: &'static str);

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
pub struct DiffMachine<'real, 'bump, Dom: RealDom<'bump>> {
    pub dom: &'real mut Dom,
    pub components: &'bump SharedArena,
    pub task_queue: &'bump TaskQueue,
    pub cur_idx: ScopeIdx,
    pub diffed: FxHashSet<ScopeIdx>,
    pub event_queue: EventQueue,
    pub seen_nodes: FxHashSet<ScopeIdx>,
}

impl<'real, 'bump, Dom> DiffMachine<'real, 'bump, Dom>
where
    Dom: RealDom<'bump>,
{
    pub fn new(
        dom: &'real mut Dom,
        components: &'bump SharedArena,
        cur_idx: ScopeIdx,
        event_queue: EventQueue,
        task_queue: &'bump TaskQueue,
    ) -> Self {
        Self {
            components,
            dom,
            cur_idx,
            event_queue,
            task_queue,
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
        match (&old_node.kind, &new_node.kind) {
            // Handle the "sane" cases first.
            // The rsx and html macros strongly discourage dynamic lists not encapsulated by a "Fragment".
            // So the sane (and fast!) cases are where the virtual structure stays the same and is easily diffable.
            (VNodeKind::Text(old), VNodeKind::Text(new)) => {
                let root = old_node.dom_id.get();
                if old.text != new.text {
                    self.dom.push(root);
                    log::debug!("Text has changed {}, {}", old.text, new.text);
                    self.dom.set_text(new.text);
                    self.dom.pop();
                }
                new_node.dom_id.set(root);
            }

            (VNodeKind::Element(old), VNodeKind::Element(new)) => {
                // If the element type is completely different, the element needs to be re-rendered completely
                // This is an optimization React makes due to how users structure their code
                //
                // In Dioxus, this is less likely to occur unless through a fragment
                let root = old_node.dom_id.get();
                if new.tag_name != old.tag_name || new.namespace != old.namespace {
                    self.dom.push(root);
                    let meta = self.create(new_node);
                    self.dom.replace_with(meta.added_to_stack);
                    self.dom.pop();
                    return;
                }

                new_node.dom_id.set(root);

                // push it just in case
                self.dom.push(root);
                self.diff_listeners(old.listeners, new.listeners);
                self.diff_attr(old.attributes, new.attributes, new.namespace);
                self.diff_children(old.children, new.children);
                self.dom.pop();
            }

            (VNodeKind::Component(old), VNodeKind::Component(new)) => {
                log::warn!("diffing components? {:#?}", new.user_fc);
                if old.user_fc == new.user_fc {
                    // Make sure we're dealing with the same component (by function pointer)

                    // Make sure the new component vnode is referencing the right scope id
                    let scope_id = old.ass_scope.get();
                    new.ass_scope.set(scope_id);

                    // make sure the component's caller function is up to date
                    let scope = self.components.try_get_mut(scope_id.unwrap()).unwrap();

                    scope.caller = new.caller.clone();

                    // ack - this doesn't work on its own!
                    scope.update_children(new.children);

                    // React doesn't automatically memoize, but we do.
                    let should_render = match old.comparator {
                        Some(comparator) => comparator(new),
                        None => true,
                    };

                    if should_render {
                        scope.run_scope().unwrap();
                        self.diff_node(scope.old_frame(), scope.next_frame());
                    } else {
                        //
                    }

                    self.seen_nodes.insert(scope_id.unwrap());
                } else {
                    // this seems to be a fairy common code path that we could
                    let mut old_iter = RealChildIterator::new(old_node, &self.components);
                    let first = old_iter
                        .next()
                        .expect("Components should generate a placeholder root");

                    // remove any leftovers
                    for to_remove in old_iter {
                        self.dom.push(to_remove);
                        self.dom.remove();
                    }

                    // seems like we could combine this into a single instruction....
                    self.dom.push(first);
                    let meta = self.create(new_node);
                    self.dom.replace_with(meta.added_to_stack);
                    self.dom.pop();

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

                // Diff using the approach where we're looking for added or removed nodes.
                if old.children.len() != new.children.len() {}

                // Diff where we think the elements are the same
                if old.children.len() == new.children.len() {}

                self.diff_children(old.children, new.children);
            }

            // The strategy here is to pick the first possible node from the previous set and use that as our replace with root
            // We also walk the "real node" list to make sure all latent roots are claened up
            // This covers the case any time a fragment or component shows up with pretty much anything else
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
                // Choose the node to use as the placeholder for replacewith
                let back_node = match old_node.kind {
                    // We special case these two types to avoid allocating the small-vecs
                    VNodeKind::Element(_) | VNodeKind::Text(_) => old_node.dom_id.get(),

                    _ => {
                        let mut old_iter = RealChildIterator::new(old_node, &self.components);

                        let back_node = old_iter
                            .next()
                            .expect("Empty fragments should generate a placeholder.");

                        // remove any leftovers
                        for to_remove in old_iter {
                            self.dom.push(to_remove);
                            self.dom.remove();
                        }

                        back_node
                    }
                };

                // replace the placeholder or first node with the nodes generated from the "new"
                self.dom.push(back_node);
                let meta = self.create(new_node);
                self.dom.replace_with(meta.added_to_stack);

                // todo use the is_static metadata to update this subtree
            }

            // TODO
            (VNodeKind::Suspended { .. }, _) => todo!(),
            (_, VNodeKind::Suspended { .. }) => todo!(),
        }
    }
}

// When we create new nodes, we need to propagate some information back up the call chain.
// This gives the caller some information on how to handle things like insertins, appending, and subtree discarding.
struct CreateMeta {
    is_static: bool,
    added_to_stack: u32,
}

impl CreateMeta {
    fn new(is_static: bool, added_to_tack: u32) -> Self {
        Self {
            is_static,
            added_to_stack: added_to_tack,
        }
    }
}

impl<'real, 'bump, Dom> DiffMachine<'real, 'bump, Dom>
where
    Dom: RealDom<'bump>,
{
    // Emit instructions to create the given virtual node.
    //
    // The change list stack may have any shape upon entering this function:
    //
    //     [...]
    //
    // When this function returns, the new node is on top of the change list stack:
    //
    //     [... node]
    fn create(&mut self, node: &'bump VNode<'bump>) -> CreateMeta {
        log::warn!("Creating node! ... {:#?}", node);
        match &node.kind {
            VNodeKind::Text(text) => {
                let real_id = self.dom.create_text_node(text.text);
                todo!()
                // text.dom_id.set(real_id);
                // CreateMeta::new(text.is_static, 1)
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
                    static_attrs,
                    static_children,
                    static_listeners,
                } = el;

                let real_id = if let Some(namespace) = namespace {
                    self.dom.create_element(tag_name, Some(namespace))
                } else {
                    self.dom.create_element(tag_name, None)
                };
                // dom_id.set(real_id);

                listeners.iter().enumerate().for_each(|(idx, listener)| {
                    listener.mounted_node.set(real_id);
                    self.dom
                        .new_event_listener(listener.event, listener.scope, idx, real_id);

                    // if the node has an event listener, then it must be visited ?
                    is_static = false;
                });

                for attr in *attributes {
                    is_static = is_static && attr.is_static;
                    self.dom.set_attribute(&attr.name, &attr.value, *namespace);
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
                //     if let VNodeKind::Text(text) = &children[0] {
                //         self.dom.set_text(text.text);
                //         return;
                //     }
                // }

                for child in *children {
                    let child_meta = self.create(child);
                    is_static = is_static && child_meta.is_static;

                    // append whatever children were generated by this call
                    self.dom.append_children(child_meta.added_to_stack);
                }

                if is_static {
                    log::debug!("created a static node {:#?}", node);
                } else {
                    log::debug!("created a dynamic node {:#?}", node);
                }

                // el_is_static.set(is_static);
                CreateMeta::new(is_static, 1)
            }

            VNodeKind::Component(vcomponent) => {
                log::debug!("Mounting a new component");
                let caller = vcomponent.caller.clone();

                let parent_idx = self.cur_idx;

                // Insert a new scope into our component list
                let idx = self
                    .components
                    .with(|components| {
                        components.insert_with_key(|new_idx| {
                            let parent_scope = self.components.try_get(parent_idx).unwrap();
                            let height = parent_scope.height + 1;
                            Scope::new(
                                caller,
                                new_idx,
                                Some(parent_idx),
                                height,
                                self.event_queue.new_channel(height, new_idx),
                                self.components.clone(),
                                vcomponent.children,
                                self.task_queue.new_submitter(),
                            )
                        })
                    })
                    .unwrap();

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
                let inner: &'bump mut _ = unsafe { &mut *self.components.components.get() };
                let new_component = inner.get_mut(idx).unwrap();

                // Actually initialize the caller's slot with the right address
                vcomponent.ass_scope.set(Some(idx));

                // Run the scope for one iteration to initialize it
                new_component.run_scope().unwrap();

                // TODO: we need to delete (IE relcaim this node, otherwise the arena will grow infinitely)
                let nextnode = new_component.next_frame();
                let meta = self.create(nextnode);

                // Finally, insert this node as a seen node.
                self.seen_nodes.insert(idx);

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

            VNodeKind::Suspended => {
                todo!();
                // let id = self.dom.create_placeholder();
                // real.set(id);
                CreateMeta::new(false, 1)
            }
        }
    }
}

impl<'a, 'bump, Dom: RealDom<'bump>> DiffMachine<'a, 'bump, Dom> {
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
    fn diff_listeners(&mut self, old: &[Listener<'_>], new: &[Listener<'_>]) {
        if !old.is_empty() || !new.is_empty() {
            // self.dom.commit_traversal();
        }
        // TODO
        // what does "diffing listeners" even mean?

        'outer1: for (_l_idx, new_l) in new.iter().enumerate() {
            // go through each new listener
            // find its corresponding partner in the old list
            // if any characteristics changed, remove and then re-add

            // if nothing changed, then just move on
            let event_type = new_l.event;

            for old_l in old {
                if new_l.event == old_l.event {
                    new_l.mounted_node.set(old_l.mounted_node.get());
                    // if new_l.id != old_l.id {
                    //     self.dom.remove_event_listener(event_type);
                    //     // TODO! we need to mess with events and assign them by RealDomNode
                    //     // self.dom
                    //     //     .update_event_listener(event_type, new_l.scope, new_l.id)
                    // }

                    continue 'outer1;
                }
            }

            // self.dom
            //     .new_event_listener(event_type, new_l.scope, new_l.id);
        }

        // 'outer2: for old_l in old {
        //     for new_l in new {
        //         if new_l.event == old_l.event {
        //             continue 'outer2;
        //         }
        //     }
        //     self.dom.remove_event_listener(old_l.event);
        // }
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
        old: &'bump [Attribute<'bump>],
        new: &'bump [Attribute<'bump>],
        namespace: Option<&'bump str>,
        // is_namespaced: bool,
    ) {
        // Do O(n^2) passes to add/update and remove attributes, since
        // there are almost always very few attributes.
        //
        // The "fast" path is when the list of attributes name is identical and in the same order
        // With the Rsx and Html macros, this will almost always be the case
        'outer: for new_attr in new {
            if new_attr.is_volatile {
                // self.dom.commit_traversal();
                self.dom
                    .set_attribute(new_attr.name, new_attr.value, namespace);
            } else {
                for old_attr in old {
                    if old_attr.name == new_attr.name {
                        if old_attr.value != new_attr.value {
                            // self.dom.commit_traversal();
                            self.dom
                                .set_attribute(new_attr.name, new_attr.value, namespace);
                        }
                        continue 'outer;
                    } else {
                        // names are different, a varying order of attributes has arrived
                    }
                }

                // self.dom.commit_traversal();
                self.dom
                    .set_attribute(new_attr.name, new_attr.value, namespace);
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
    fn diff_children(&mut self, old: &'bump [VNode<'bump>], new: &'bump [VNode<'bump>]) {
        if new.is_empty() {
            if !old.is_empty() {
                // self.dom.commit_traversal();
                self.remove_all_children(old);
            }
            return;
        }

        if new.len() == 1 {
            match (&old.first(), &new[0]) {
                // (Some(VNodeKind::Text(old_vtext)), VNodeKind::Text(new_vtext))
                //     if old_vtext.text == new_vtext.text =>
                // {
                //     // Don't take this fast path...
                // }

                // (_, VNodeKind::Text(text)) => {
                //     // self.dom.commit_traversal();
                //     log::debug!("using optimized text set");
                //     self.dom.set_text(text.text);
                //     return;
                // }

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
            log::warn!("using the wrong approach");
            self.diff_non_keyed_children(old, new);
            // todo!("Not yet implemented a migration away from temporaries");
            // let t = self.dom.next_temporary();
            // self.diff_keyed_children(old, new);
            // self.dom.set_next_temporary(t);
        } else {
            // log::debug!("diffing non keyed children");
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
    fn diff_keyed_children(&self, old: &'bump [VNode<'bump>], new: &'bump [VNode<'bump>]) {
        // todo!();
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
        &self,
        old: &'bump [VNode<'bump>],
        new: &'bump [VNode<'bump>],
    ) -> KeyedPrefixResult {
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

    // Remove all of a node's children.
    //
    // The change list stack must have this shape upon entry to this function:
    //
    //     [... parent]
    //
    // When this function returns, the change list stack is in the same state.
    pub fn remove_all_children(&mut self, old: &'bump [VNode<'bump>]) {
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
    pub fn create_and_append_children(&mut self, new: &'bump [VNode<'bump>]) {
        for child in new {
            let meta = self.create(child);
            self.dom.append_children(meta.added_to_stack);
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
        mut new: &[VNode<'bump>],
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
        old: &[VNode<'bump>],
        new: &[VNode<'bump>],
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
    fn diff_non_keyed_children(&mut self, old: &'bump [VNode<'bump>], new: &'bump [VNode<'bump>]) {
        // Handled these cases in `diff_children` before calling this function.
        debug_assert!(!new.is_empty());
        debug_assert!(!old.is_empty());

        //     [... parent]
        // self.dom.go_down();
        // self.dom.push_root()
        //     [... parent child]

        // todo!()
        for (i, (new_child, old_child)) in new.iter().zip(old.iter()).enumerate() {
            // [... parent prev_child]
            // self.dom.go_to_sibling(i);
            // [... parent this_child]

            // let did = old_child.get_mounted_id(self.components).unwrap();
            // if did.0 == 0 {
            //     log::debug!("Root is bad: {:#?}", old_child);
            // }
            // self.dom.push_root(did);
            self.diff_node(old_child, new_child);

            // let old_id = old_child.get_mounted_id(self.components).unwrap();
            // let new_id = new_child.get_mounted_id(self.components).unwrap();

            // log::debug!(
            //     "pushed root. {:?}, {:?}",
            //     old_child.get_mounted_id(self.components).unwrap(),
            //     new_child.get_mounted_id(self.components).unwrap()
            // );
            // if old_id != new_id {
            //     log::debug!("Mismatch: {:?}", new_child);
            // }
        }

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

    // Remove the current child and all of its following siblings.
    //
    // The change list stack must have this shape upon entry to this function:
    //
    //     [... parent child]
    //
    // After the function returns, the child is no longer on the change list stack:
    //
    //     [... parent]
    pub fn remove_self_and_next_siblings(&self, old: &[VNode<'bump>]) {
        // debug_assert!(self.dom.traversal_is_committed());
        for child in old {
            if let VNodeKind::Component(vcomp) = child.kind {
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

/// This iterator iterates through a list of virtual children and only returns real children (Elements or Text).
///
/// This iterator is useful when it's important to load the next real root onto the top of the stack for operations like
/// "InsertBefore".
struct RealChildIterator<'a> {
    scopes: &'a SharedArena,

    // Heuristcally we should never bleed into 4 completely nested fragments/components
    // Smallvec lets us stack allocate our little stack machine so the vast majority of cases are sane
    // TODO: use const generics instead of the 4 estimation
    stack: smallvec::SmallVec<[(u16, &'a VNode<'a>); 4]>,
}

impl<'a> RealChildIterator<'a> {
    fn new(starter: &'a VNode<'a>, scopes: &'a SharedArena) -> Self {
        Self {
            scopes,
            stack: smallvec::smallvec![(0, starter)],
        }
    }
}

impl<'a> Iterator for RealChildIterator<'a> {
    type Item = RealDomNode;

    fn next(&mut self) -> Option<RealDomNode> {
        let mut should_pop = false;
        let mut returned_node = None;
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
                        returned_node = Some(node.dom_id.get());
                    }

                    // If we get a fragment we push the next child
                    VNodeKind::Fragment(frag) => {
                        let subcount = *count as usize;

                        if frag.children.len() == 0 {
                            should_pop = true;
                            returned_node = Some(node.dom_id.get());
                        }

                        if subcount >= frag.children.len() {
                            should_pop = true;
                        } else {
                            should_push = Some(&frag.children[subcount]);
                        }
                    }

                    // Immediately abort suspended nodes - can't do anything with them yet
                    // VNodeKind::Suspended => should_pop = true,
                    VNodeKind::Suspended => todo!(),

                    // For components, we load their root and push them onto the stack
                    VNodeKind::Component(sc) => {
                        let scope = self.scopes.try_get(sc.ass_scope.get().unwrap()).unwrap();

                        // Simply swap the current node on the stack with the root of the component
                        *node = scope.root();
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
