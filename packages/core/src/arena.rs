use std::cell::{RefCell, RefMut};
use std::fmt::Display;
use std::{cell::UnsafeCell, rc::Rc};

use crate::heuristics::*;
use crate::innerlude::*;
use futures_channel::mpsc::{UnboundedReceiver, UnboundedSender};
use futures_util::stream::FuturesUnordered;
use fxhash::{FxHashMap, FxHashSet};
use slab::Slab;
use smallvec::SmallVec;

// slotmap::new_key_type! {
//     // A dedicated key type for the all the scopes
//     pub struct ScopeId;
// }
// #[cfg(feature = "serialize", serde::Serialize)]
// #[cfg(feature = "serialize", serde::Serialize)]
#[derive(serde::Serialize, serde::Deserialize, Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct ScopeId(pub usize);

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct ElementId(pub usize);
impl Display for ElementId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl ElementId {
    pub fn as_u64(self) -> u64 {
        self.0 as u64
    }
}

type Shared<T> = Rc<RefCell<T>>;
type UiReceiver = UnboundedReceiver<EventTrigger>;
type UiSender = UnboundedSender<EventTrigger>;

type TaskReceiver = UnboundedReceiver<ScopeId>;
type TaskSender = UnboundedSender<ScopeId>;

/// These are resources shared among all the components and the virtualdom itself
#[derive(Clone)]
pub struct SharedResources {
    pub components: Rc<UnsafeCell<Slab<Scope>>>,

    pub(crate) heuristics: Shared<HeuristicsEngine>,

    // Used by "set_state" and co - is its own queue
    pub immediate_sender: TaskSender,
    pub immediate_receiver: Shared<TaskReceiver>,

    /// Triggered by event listeners
    pub ui_event_sender: UiSender,
    pub ui_event_receiver: Shared<UiReceiver>,

    // Garbage stored
    pub pending_garbage: Shared<FxHashSet<ScopeId>>,

    // In-flight futures
    pub async_tasks: Shared<FuturesUnordered<FiberTask>>,

    /// We use a SlotSet to keep track of the keys that are currently being used.
    /// However, we don't store any specific data since the "mirror"
    pub raw_elements: Shared<Slab<()>>,

    pub task_setter: Rc<dyn Fn(ScopeId)>,
}

impl SharedResources {
    pub fn new() -> Self {
        // preallocate 2000 elements and 20 scopes to avoid dynamic allocation
        let components: Rc<UnsafeCell<Slab<Scope>>> =
            Rc::new(UnsafeCell::new(Slab::with_capacity(100)));

        // elements are super cheap - the value takes no space
        let raw_elements = Slab::with_capacity(2000);

        let (ui_sender, ui_receiver) = futures_channel::mpsc::unbounded();
        let (immediate_sender, immediate_receiver) = futures_channel::mpsc::unbounded();

        let heuristics = HeuristicsEngine::new();

        // we allocate this task setter once to save us from having to allocate later
        let task_setter = {
            let queue = immediate_sender.clone();
            let components = components.clone();
            Rc::new(move |idx: ScopeId| {
                let comps = unsafe { &*components.get() };

                if let Some(scope) = comps.get(idx.0) {
                    // todo!("implement immediates again")
                    //

                    // queue
                    // .unbounded_send(EventTrigger::new(
                    //     V
                    //     idx,
                    //     None,
                    //     EventPriority::High,
                    // ))
                    // .expect("The event queu receiver should *never* be dropped");
                }
            }) as Rc<dyn Fn(ScopeId)>
        };

        Self {
            components,
            async_tasks: Rc::new(RefCell::new(FuturesUnordered::new())),

            ui_event_receiver: Rc::new(RefCell::new(ui_receiver)),
            ui_event_sender: ui_sender,

            immediate_receiver: Rc::new(RefCell::new(immediate_receiver)),
            immediate_sender: immediate_sender,

            pending_garbage: Rc::new(RefCell::new(FxHashSet::default())),

            heuristics: Rc::new(RefCell::new(heuristics)),
            raw_elements: Rc::new(RefCell::new(raw_elements)),
            task_setter,
        }
    }

    /// this is unsafe because the caller needs to track which other scopes it's already using
    pub fn get_scope(&self, idx: ScopeId) -> Option<&Scope> {
        let inner = unsafe { &*self.components.get() };
        inner.get(idx.0)
    }

    /// this is unsafe because the caller needs to track which other scopes it's already using
    pub fn get_scope_mut(&self, idx: ScopeId) -> Option<&mut Scope> {
        let inner = unsafe { &mut *self.components.get() };
        inner.get_mut(idx.0)
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
        Ok(inner.remove(id.0))
        // .try_remove(id.0)
        // .ok_or_else(|| Error::FatalInternal("Scope not found"))
    }

    pub fn reserve_node(&self) -> ElementId {
        ElementId(self.raw_elements.borrow_mut().insert(()))
    }

    /// return the id, freeing the space of the original node
    pub fn collect_garbage(&self, id: ElementId) {
        self.raw_elements.borrow_mut().remove(id.0);
    }

    pub fn insert_scope_with_key(&self, f: impl FnOnce(ScopeId) -> Scope) -> ScopeId {
        let g = unsafe { &mut *self.components.get() };
        let entry = g.vacant_entry();
        let id = ScopeId(entry.key());
        entry.insert(f(id));
        id
    }

    pub fn schedule_update(&self) -> Rc<dyn Fn(ScopeId)> {
        self.task_setter.clone()
    }

    pub fn submit_task(&self, task: FiberTask) -> TaskHandle {
        self.async_tasks.borrow_mut().push(task);
        TaskHandle {}
    }

    pub fn make_trigger_key(&self, trigger: &EventTrigger) -> EventKey {
        let height = self
            .get_scope(trigger.originator)
            .map(|f| f.height)
            .unwrap();

        EventKey {
            height,
            originator: trigger.originator,
            priority: trigger.priority,
        }
    }
}

pub struct TaskHandle {}

impl TaskHandle {
    pub fn toggle(&self) {}
    pub fn start(&self) {}
    pub fn stop(&self) {}
    pub fn restart(&self) {}
}
