use std::cell::{RefCell, RefMut};
use std::{cell::UnsafeCell, rc::Rc};

use crate::heuristics::*;
use crate::innerlude::*;
use futures_util::stream::FuturesUnordered;
use slotmap::SlotMap;

slotmap::new_key_type! {
    // A dedicated key type for the all the scopes
    pub struct ScopeId;
}

slotmap::new_key_type! {
    // A dedicated key type for every real element that the virtualdom creates.
    //
    // This is a slotmap key because we expect the "mirror" realdom to also maintain a slotmap mapping
    // of virtual element to real element.
    pub struct ElementId;
}

impl ElementId {
    pub fn as_u64(self) -> u64 {
        self.0.as_ffi()
    }
}

type Shared<T> = Rc<RefCell<T>>;

/// These are resources shared among all the components and the virtualdom itself
#[derive(Clone)]
pub struct SharedResources {
    pub components: Rc<UnsafeCell<SlotMap<ScopeId, Scope>>>,

    pub event_queue: Shared<Vec<HeightMarker>>,

    pub events: Shared<Vec<EventTrigger>>,

    pub(crate) heuristics: Shared<HeuristicsEngine>,

    pub(crate) tasks: Shared<FuturesUnordered<FiberTask>>,

    /// We use a SlotSet to keep track of the keys that are currently being used.
    /// However, we don't store any specific data since the "mirror"
    pub raw_elements: Shared<SlotMap<ElementId, ()>>,

    pub task_setter: Rc<dyn Fn(ScopeId)>,
}

impl SharedResources {
    pub fn new() -> Self {
        // preallocate 1000 elements and 20 scopes to avoid dynamic allocation
        let components = Rc::new(UnsafeCell::new(
            SlotMap::<ScopeId, Scope>::with_capacity_and_key(20),
        ));
        let raw_elements = SlotMap::<ElementId, ()>::with_capacity_and_key(1000);

        let event_queue = Rc::new(RefCell::new(Vec::new()));
        let tasks = Vec::new();
        let heuristics = HeuristicsEngine::new();

        let queue = event_queue.clone();
        let _components = components.clone();
        let task_setter = Rc::new(move |idx| {
            let comps = unsafe { &*_components.get() };
            if let Some(scope) = comps.get(idx) {
                queue.borrow_mut().push(HeightMarker {
                    height: scope.height,
                    idx,
                })
            }
        });

        Self {
            event_queue,
            components,
            tasks: Rc::new(RefCell::new(FuturesUnordered::new())),
            events: Rc::new(RefCell::new(tasks)),
            heuristics: Rc::new(RefCell::new(heuristics)),
            raw_elements: Rc::new(RefCell::new(raw_elements)),
            task_setter,
        }
    }

    /// this is unsafe because the caller needs to track which other scopes it's already using
    pub unsafe fn get_scope(&self, idx: ScopeId) -> Option<&Scope> {
        let inner = &*self.components.get();
        inner.get(idx)
    }

    /// this is unsafe because the caller needs to track which other scopes it's already using
    pub unsafe fn get_sope_mut(&self, idx: ScopeId) -> Option<&mut Scope> {
        let inner = &mut *self.components.get();
        inner.get_mut(idx)
    }

    pub fn with_scope<'b, O: 'static>(
        &'b self,
        _id: ScopeId,
        _f: impl FnOnce(&'b mut Scope) -> O,
    ) -> Result<O> {
        todo!()
    }

    // return a bumpframe with a lifetime attached to the arena borrow
    // this is useful for merging lifetimes
    pub fn with_scope_vnode<'b>(
        &self,
        _id: ScopeId,
        _f: impl FnOnce(&mut Scope) -> &VNode<'b>,
    ) -> Result<&VNode<'b>> {
        todo!()
    }

    pub fn try_remove(&self, id: ScopeId) -> Result<Scope> {
        let inner = unsafe { &mut *self.components.get() };
        inner
            .remove(id)
            .ok_or_else(|| Error::FatalInternal("Scope not found"))
    }

    pub fn reserve_node(&self) -> ElementId {
        self.raw_elements.borrow_mut().insert(())
    }

    /// return the id, freeing the space of the original node
    pub fn collect_garbage(&self, id: ElementId) {}

    pub fn borrow_queue(&self) -> RefMut<Vec<HeightMarker>> {
        self.event_queue.borrow_mut()
    }

    pub fn insert_scope_with_key(&self, f: impl FnOnce(ScopeId) -> Scope) -> ScopeId {
        let g = unsafe { &mut *self.components.get() };
        g.insert_with_key(f)
    }

    pub fn schedule_update(&self) -> Rc<dyn Fn(ScopeId)> {
        self.task_setter.clone()
    }

    pub fn submit_task(&self, task: FiberTask) -> TaskHandle {
        self.tasks.borrow_mut().push(task);
        TaskHandle {}
    }
}

pub struct TaskHandle {}

impl TaskHandle {
    pub fn toggle(&self) {}
    pub fn start(&self) {}
    pub fn stop(&self) {}
    pub fn restart(&self) {}
}
