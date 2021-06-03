//! # VirtualDOM Implementation for Rust
//! This module provides the primary mechanics to create a hook-based, concurrent VDOM for Rust.
//!
//! In this file, multiple items are defined. This file is big, but should be documented well to
//! navigate the innerworkings of the Dom. We try to keep these main mechanics in this file to limit
//! the possible exposed API surface (keep fields private). This particular implementation of VDOM
//! is extremely efficient, but relies on some unsafety under the hood to do things like manage
//! micro-heaps for components. We are currently working on refactoring the safety out into safe(r)
//! abstractions, but current tests (MIRI and otherwise) show no issues with the current implementation.
//!
//! Included is:
//! - The [`VirtualDom`] itself
//! - The [`Scope`] object for mangning component lifecycle
//! - The [`ActiveFrame`] object for managing the Scope`s microheap
//! - The [`Context`] object for exposing VirtualDOM API to components
//! - The [`NodeCtx`] object for lazyily exposing the `Context` API to the nodebuilder API
//! - The [`Hook`] object for exposing state management in components.
//!
//! This module includes just the barebones for a complete VirtualDOM API.
//! Additional functionality is defined in the respective files.

pub use crate::scope::*;
pub use crate::support::*;
use crate::{arena::ScopeArena, innerlude::*};
use bumpalo::Bump;
use generational_arena::Arena;
use std::sync::atomic::AtomicU32;
use std::{
    any::{Any, TypeId},
    cell::RefCell,
    collections::{HashMap, HashSet, VecDeque},
    fmt::Debug,
    future::Future,
    ops::Deref,
    pin::Pin,
    rc::{Rc, Weak},
};

/// An integrated virtual node system that progresses events and diffs UI trees.
/// Differences are converted into patches which a renderer can use to draw the UI.
pub struct VirtualDom {
    /// All mounted components are arena allocated to make additions, removals, and references easy to work with
    /// A generational arena is used to re-use slots of deleted scopes without having to resize the underlying arena.
    ///
    /// This is wrapped in an UnsafeCell because we will need to get mutable access to unique values in unique bump arenas
    /// and rusts's guartnees cannot prove that this is safe. We will need to maintain the safety guarantees manually.
    pub components: ScopeArena,

    /// The index of the root component
    /// Should always be the first (gen=0, id=0)
    pub base_scope: ScopeIdx,

    /// All components dump their updates into a queue to be processed
    pub(crate) event_queue: EventQueue,

    /// a strong allocation to the "caller" for the original component and its props
    #[doc(hidden)]
    _root_caller: Rc<OpaqueComponent<'static>>,

    /// Type of the original ctx. This is stored as TypeId so VirtualDom does not need to be generic.
    ///
    /// Whenver props need to be updated, an Error will be thrown if the new props do not
    /// match the props used to create the VirtualDom.
    #[doc(hidden)]
    _root_prop_type: std::any::TypeId,

    seen_nodes: HashSet<ScopeIdx>,
}

// These impls are actually wrong. The DOM needs to have a mutex implemented.
unsafe impl Sync for VirtualDom {}
unsafe impl Send for VirtualDom {}

impl VirtualDom {
    pub fn new(root: impl Fn(Context<()>) -> VNode + 'static) -> Self {
        Self::new_with_props(root, ())
    }

    pub fn new_with_props<P: Properties + 'static>(
        root: impl Fn(Context<P>) -> VNode + 'static,
        root_props: P,
    ) -> Self {
        let components = ScopeArena::new(Arena::new());

        // Normally, a component would be passed as a child in the RSX macro which automatically produces OpaqueComponents
        // Here, we need to make it manually, using an RC to force the Weak reference to stick around for the main scope.
        let _root_caller: Rc<OpaqueComponent<'static>> = Rc::new(move |scope| {
            // the lifetime of this closure is just as long as the lifetime on the scope reference
            // this closure moves root props (which is static) into this closure
            let props = unsafe { &*(&root_props as *const _) };
            root(Context { props, scope })
        });

        // Create a weak reference to the OpaqueComponent for the root scope to use as its render function
        let caller_ref = Rc::downgrade(&_root_caller);

        // Build a funnel for hooks to send their updates into. The `use_hook` method will call into the update funnel.
        let event_queue = EventQueue::default();
        let _event_queue = event_queue.clone();

        // Make the first scope
        // We don't run the component though, so renderers will need to call "rebuild" when they initialize their DOM
        let link = components.clone();
        let event_channel = Rc::new(move || {});
        let base_scope = components
            .with(|arena| {
                arena.insert_with(move |myidx| {
                    Scope::new(caller_ref, myidx, None, 0, event_channel, link, &[])
                })
            })
            .unwrap();

        Self {
            _root_caller,
            base_scope,
            event_queue,
            components,
            _root_prop_type: TypeId::of::<P>(),
            seen_nodes: Default::default(),
        }
    }
}

impl VirtualDom {
    pub fn progress_with_event(&mut self, event: EventTrigger) -> Result<EditList> {
        todo!()
        // let id = event.component_id.clone();

        // self.components.try_get_mut(id)?.call_listener(event)?;

        // let mut diff_machine = DiffMachine::new();
        // self.progress_completely(&mut diff_machine)?;

        // Ok(diff_machine.consume())
    }

    pub(crate) fn progress_completely<'s>(
        &'s mut self,
        // diff_machine: &'_ mut DiffMachine<'s>,
    ) -> Result<()> {
        // Add this component to the list of components that need to be difed
        #[allow(unused_assignments)]
        let mut cur_height: u32 = 0;

        let mut updates = self.event_queue.0.as_ref().borrow_mut();

        // Order the nodes by their height, we want the biggest nodes on the top
        // This prevents us from running the same component multiple times
        updates.sort_unstable();

        // Iterate through the triggered nodes (sorted by height) and begin to diff them
        for update in updates.drain(..) {
            // Make sure this isn't a node we've already seen, we don't want to double-render anything
            // If we double-renderer something, this would cause memory safety issues
            if self.seen_nodes.contains(&update.idx) {
                continue;
            }

            // Now, all the "seen nodes" are nodes that got notified by running this listener
            self.seen_nodes.insert(update.idx.clone());

            // Start a new mutable borrow to components
            // We are guaranteeed that this scope is unique because we are tracking which nodes have modified

            let cur_component = self.components.try_get_mut(update.idx).unwrap();

            cur_component.run_scope()?;
            let change_list = EditMachine::new();

            self.diff_node(
                &mut change_list,
                cur_component.old_frame(),
                cur_component.next_frame(),
            );

            // cur_height = cur_component.height;

            // log::debug!(
            //     "Processing update: {:#?} with height {}",
            //     &update.idx,
            //     cur_height
            // );
        }

        Ok(())
    }

    pub fn diff_node<'a>(
        &mut self,
        change_list: &mut EditMachine<'a>,
        old: &VNode<'a>,
        new: &VNode<'a>,
    ) {
        // pub fn diff_node(&mut self, old: &VNode<'a>, new: &VNode<'a>) {
        /*
        For each valid case, we "commit traversal", meaning we save this current position in the tree.
        Then, we diff and queue an edit event (via chagelist). s single trees - when components show up, we save that traversal and then re-enter later.
        When re-entering, we reuse the EditList in DiffState
        */
        match (old, new) {
            (VNode::Text(VText { text: old_text }), VNode::Text(VText { text: new_text })) => {
                if old_text != new_text {
                    change_list.commit_traversal();
                    change_list.set_text(new_text);
                }
            }

            (VNode::Text(_), VNode::Element(_)) => {
                change_list.commit_traversal();
                self.create(change_list, new);
                change_list.replace_with();
            }

            (VNode::Element(_), VNode::Text(_)) => {
                change_list.commit_traversal();
                self.create(change_list, new);
                change_list.replace_with();
            }

            (VNode::Element(eold), VNode::Element(enew)) => {
                // If the element type is completely different, the element needs to be re-rendered completely
                if enew.tag_name != eold.tag_name || enew.namespace != eold.namespace {
                    change_list.commit_traversal();
                    change_list.replace_with();
                    return;
                }
                todo!()

                // self.diff_listeners(eold.listeners, enew.listeners);
                // self.diff_attr(eold.attributes, enew.attributes, enew.namespace.is_some());
                // self.diff_children(eold.children, enew.children);
            }

            (VNode::Component(cold), VNode::Component(cnew)) => {
                // todo!("should not happen")
                // change_list.commit_traversal();
                if cold.user_fc == cnew.user_fc {
                    // todo: create a stable addr
                    let caller = Rc::downgrade(&cnew.caller);
                    let id = cold.stable_addr.borrow().unwrap();
                    *cnew.stable_addr.borrow_mut() = Some(id);
                    *cnew.ass_scope.borrow_mut() = *cold.ass_scope.borrow();

                    let scope = Rc::downgrade(&cold.ass_scope);
                    // self.lifecycle_events
                    //     .push_back(LifeCycleEvent::PropsChanged {
                    //         caller,
                    //         root_id: id,
                    //         stable_scope_addr: scope,
                    //     });
                } else {
                    let caller = Rc::downgrade(&cnew.caller);
                    let id = cold.stable_addr.borrow().unwrap();
                    let old_scope = Rc::downgrade(&cold.ass_scope);
                    let new_scope = Rc::downgrade(&cnew.ass_scope);

                    // self.lifecycle_events.push_back(LifeCycleEvent::Replace {
                    //     caller,
                    //     root_id: id,
                    //     old_scope,
                    //     new_scope,
                    // });
                }
            }

            // todo: knock out any listeners
            (_, VNode::Component(_new)) => {
                change_list.commit_traversal();
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

    fn create<'a>(&mut self, change_list: &mut EditMachine<'a>, node: &VNode<'a>) {
        debug_assert!(change_list.traversal_is_committed());
        match node {
            VNode::Text(VText { text }) => {
                change_list.create_text_node(text);
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
                    change_list.create_element_ns(tag_name, namespace);
                } else {
                    change_list.create_element(tag_name);
                }

                listeners.iter().enumerate().for_each(|(_id, listener)| {
                    change_list.new_event_listener(listener.event, listener.scope, listener.id)
                });

                for attr in attributes {
                    change_list.set_attribute(&attr.name, &attr.value, namespace.is_some());
                }

                // Fast path: if there is a single text child, it is faster to
                // create-and-append the text node all at once via setting the
                // parent's `textContent` in a single change list instruction than
                // to emit three instructions to (1) create a text node, (2) set its
                // text content, and finally (3) append the text node to this
                // parent.
                if children.len() == 1 {
                    if let VNode::Text(VText { text }) = children[0] {
                        change_list.set_text(text);
                        return;
                    }
                }

                for child in children {
                    self.create(change_list, child);
                    change_list.append_child();
                }
            }

            /*
            todo: integrate re-entrace
            */
            VNode::Component(component) => {
                change_list.create_text_node("placeholder for vcomponent");

                let id = next_id();
                *component.stable_addr.as_ref().borrow_mut() = Some(id);
                change_list.save_known_root(id);
                let scope = Rc::downgrade(&component.ass_scope);
                // self.lifecycle_events.push_back(LifeCycleEvent::Mount {
                //     caller: Rc::downgrade(&component.caller),
                //     root_id: id,
                //     stable_scope_addr: scope,
                // });
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
}

static COUNTER: AtomicU32 = AtomicU32::new(1);
fn next_id() -> u32 {
    COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

// mod old {
//             // Now, the entire subtree has been invalidated. We need to descend depth-first and process
//             // any updates that the diff machine has proprogated into the component lifecycle queue
//             while let Some(event) = diff_machine.lifecycle_events.pop_front() {
//                 match event {
//                     // A new component has been computed from the diffing algorithm
//                     // create a new component in the arena, run it, move the diffing machine to this new spot, and then diff it
//                     // this will flood the lifecycle queue with new updates to build up the subtree
//                     LifeCycleEvent::Mount {
//                         caller,
//                         root_id: id,
//                         stable_scope_addr,
//                     } => {
//                         log::debug!("Mounting a new component");

//                         // We're modifying the component arena while holding onto references into the assoiated bump arenas of its children
//                         // those references are stable, even if the component arena moves around in memory, thanks to the bump arenas.
//                         // However, there is no way to convey this to rust, so we need to use unsafe to pierce through the lifetime.

//                         // Insert a new scope into our component list
//                         let idx = self.components.with(|components| {
//                             components.insert_with(|new_idx| {
//                                 let height = cur_height + 1;
//                                 Scope::new(
//                                     caller,
//                                     new_idx,
//                                     Some(cur_component.arena_idx),
//                                     height,
//                                     self.event_queue.new_channel(height, new_idx),
//                                     self.components.clone(),
//                                     &[],
//                                 )
//                             })
//                         })?;

//                         {
//                             let cur_component = self.components.try_get_mut(update.idx).unwrap();
//                             let mut ch = cur_component.descendents.borrow_mut();
//                             ch.insert(idx);
//                             std::mem::drop(ch);
//                         }

//                         // Grab out that component
//                         let new_component = self.components.try_get_mut(idx).unwrap();

//                         // Actually initialize the caller's slot with the right address
//                         *stable_scope_addr.upgrade().unwrap().as_ref().borrow_mut() = Some(idx);

//                         // Run the scope for one iteration to initialize it
//                         new_component.run_scope()?;

//                         // Navigate the diff machine to the right point in the output dom
//                         diff_machine.change_list.load_known_root(id);

//                         // And then run the diff algorithm
//                         diff_machine
//                             .diff_node(new_component.old_frame(), new_component.next_frame());

//                         // Finally, insert this node as a seen node.
//                         seen_nodes.insert(idx);
//                     }

//                     // A component has remained in the same location but its properties have changed
//                     // We need to process this component and then dump the output lifecycle events into the queue
//                     LifeCycleEvent::PropsChanged {
//                         caller,
//                         root_id,
//                         stable_scope_addr,
//                     } => {
//                         log::debug!("Updating a component after its props have changed");

//                         // Get the stable index to the target component
//                         // This *should* exist due to guarantees in the diff algorithm
//                         let idx = stable_scope_addr
//                             .upgrade()
//                             .unwrap()
//                             .as_ref()
//                             .borrow()
//                             .unwrap();

//                         // Grab out that component
//                         let component = self.components.try_get_mut(idx).unwrap();

//                         // We have to move the caller over or running the scope will fail
//                         component.update_caller(caller);

//                         // Run the scope
//                         component.run_scope()?;

//                         // Navigate the diff machine to the right point in the output dom
//                         diff_machine.change_list.load_known_root(root_id);

//                         // And then run the diff algorithm
//                         diff_machine.diff_node(component.old_frame(), component.next_frame());

//                         // Finally, insert this node as a seen node.
//                         seen_nodes.insert(idx);
//                     }

//                     // A component's parent has updated, but its properties did not change.
//                     // This means the caller ptr is invalidated and needs to be updated, but the component itself does not need to be re-ran
//                     LifeCycleEvent::SameProps {
//                         caller,
//                         stable_scope_addr,
//                         ..
//                     } => {
//                         // In this case, the parent made a new VNode that resulted in the same props for us
//                         // However, since our caller is located in a Bump frame, we need to update the caller pointer (which is now invalid)
//                         log::debug!("Received the same props");

//                         // Get the stable index to the target component
//                         // This *should* exist due to guarantees in the diff algorithm
//                         let idx = stable_scope_addr
//                             .upgrade()
//                             .unwrap()
//                             .as_ref()
//                             .borrow()
//                             .unwrap();

//                         // Grab out that component
//                         let component = self.components.try_get_mut(idx).unwrap();

//                         // We have to move the caller over or running the scope will fail
//                         component.update_caller(caller);

//                         // This time, we will not add it to our seen nodes since we did not actually run it
//                     }

//                     LifeCycleEvent::Remove {
//                         root_id,
//                         stable_scope_addr,
//                     } => {
//                         let id = stable_scope_addr
//                             .upgrade()
//                             .unwrap()
//                             .as_ref()
//                             .borrow()
//                             .unwrap();

//                         log::warn!("Removing node {:#?}", id);

//                         // This would normally be recursive but makes sense to do linear to
//                         let mut children_to_remove = VecDeque::new();
//                         children_to_remove.push_back(id);

//                         // Accumulate all the child components that need to be removed
//                         while let Some(child_id) = children_to_remove.pop_back() {
//                             let comp = self.components.try_get(child_id).unwrap();
//                             let children = comp.descendents.borrow();
//                             for child in children.iter() {
//                                 children_to_remove.push_front(*child);
//                             }
//                             log::debug!("Removing component: {:#?}", child_id);
//                             self.components
//                                 .with(|components| components.remove(child_id).unwrap())
//                                 .unwrap();
//                         }
//                     }

//                     LifeCycleEvent::Replace {
//                         caller,
//                         root_id: id,
//                         ..
//                     } => {
//                         unimplemented!("This feature (Replace) is unimplemented")
//                     }
//                 }
//             }

// }
