use std::any::Any;

use std::any::TypeId;
use std::cell::{Ref, RefCell, RefMut};
use std::collections::{BTreeMap, BTreeSet, BinaryHeap, HashMap, HashSet, VecDeque};
use std::pin::Pin;

use crate::innerlude::*;

/// The "Mutations" object holds the changes that need to be made to the DOM.
pub struct Mutations<'s> {
    pub edits: Vec<DomEdit<'s>>,
    pub noderefs: Vec<NodeRefMutation<'s>>,
}

impl<'s> Mutations<'s> {
    pub fn new() -> Self {
        let edits = Vec::new();
        let noderefs = Vec::new();
        Self { edits, noderefs }
    }
}

// refs are only assigned once
pub struct NodeRefMutation<'a> {
    element: &'a mut Option<once_cell::sync::OnceCell<Box<dyn Any>>>,
    element_id: ElementId,
}

impl<'a> NodeRefMutation<'a> {
    pub fn downcast_ref<T: 'static>(&self) -> Option<&T> {
        self.element
            .as_ref()
            .and_then(|f| f.get())
            .and_then(|f| f.downcast_ref::<T>())
    }
    pub fn downcast_mut<T: 'static>(&mut self) -> Option<&mut T> {
        self.element
            .as_mut()
            .and_then(|f| f.get_mut())
            .and_then(|f| f.downcast_mut::<T>())
    }
}

pub struct Scheduler {
    current_priority: EventPriority,

    dirty_scopes: [HashSet<DirtyScope>; 3],

    fibers: Vec<Fiber<'static>>,
}

impl Scheduler {
    pub fn new() -> Self {
        Self {
            fibers: Vec::new(),

            current_priority: EventPriority::Low,

            // low, medium, high
            dirty_scopes: [HashSet::new(), HashSet::new(), HashSet::new()],
        }
    }

    pub fn add_dirty_scope(&mut self, scope: ScopeId, priority: EventPriority) {
        //

        // generated_immediates
        //     .entry(dirty_scope)
        //     .and_modify(|cur_priority| {
        //         if *cur_priority > new_priority {
        //             *cur_priority = new_priority;
        //         }
        //     })
        //     .or_insert_with(|| new_priority);
    }

    pub fn has_work() {}

    pub fn progress_work(&mut self, machine: &mut DiffMachine) {}
}

#[derive(PartialEq, Eq, Copy, Clone, Debug)]
pub struct DirtyScope {
    height: u32,
    priority: EventPriority,
    start_tick: u32,
}

// fibers in dioxus aren't exactly the same as React's. Our fibers are more like a "saved state" of the diffing algorithm.
pub struct Fiber<'a> {
    // scopes that haven't been updated yet
    pending_scopes: Vec<ScopeId>,

    pending_nodes: Vec<*const VNode<'a>>,

    // WIP edits
    edits: Vec<DomEdit<'a>>,

    started: bool,

    // a fiber is finished when no more scopes or nodes are pending
    completed: bool,
}

impl Fiber<'_> {
    fn new() -> Self {
        Self {
            pending_scopes: Vec::new(),
            pending_nodes: Vec::new(),
            edits: Vec::new(),
            started: false,
            completed: false,
        }
    }
}
