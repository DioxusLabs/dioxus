use std::{any::Any, cell::RefCell, collections::HashMap, rc::Rc, sync::Arc};

use dioxus_core::ScopeId;
use im_rc::HashSet;

use crate::Readable;

pub type AtomId = *const ();

pub struct AtomRoot {
    pub atoms: RefCell<HashMap<AtomId, Slot>>,
    pub update_any: Arc<dyn Fn(ScopeId)>,
}

pub struct Slot {
    pub value: Rc<dyn Any>,
    pub subscribers: HashSet<ScopeId>,
}

impl AtomRoot {
    pub fn new(update_any: Arc<dyn Fn(ScopeId)>) -> Self {
        Self {
            update_any,
            atoms: RefCell::new(HashMap::new()),
        }
    }

    pub fn initialize<V: 'static>(&self, f: impl Readable<V>) {
        let id = f.unique_id();
        if self.atoms.borrow().get(&id).is_none() {
            self.atoms.borrow_mut().insert(
                id,
                Slot {
                    value: Rc::new(f.init()),
                    subscribers: HashSet::new(),
                },
            );
        }
    }

    pub fn register<V: 'static>(&self, f: impl Readable<V>, scope: ScopeId) -> Rc<V> {
        let mut atoms = self.atoms.borrow_mut();

        // initialize the value if it's not already initialized
        if let Some(slot) = atoms.get_mut(&f.unique_id()) {
            slot.subscribers.insert(scope);
            slot.value.clone().downcast().unwrap()
        } else {
            let value = Rc::new(f.init());
            let mut subscribers = HashSet::new();
            subscribers.insert(scope);

            atoms.insert(
                f.unique_id(),
                Slot {
                    value: value.clone(),
                    subscribers,
                },
            );
            value
        }
    }

    pub fn set<V: 'static>(&self, ptr: AtomId, value: V) {
        let mut atoms = self.atoms.borrow_mut();

        if let Some(slot) = atoms.get_mut(&ptr) {
            slot.value = Rc::new(value);
            tracing::trace!("found item with subscribers {:?}", slot.subscribers);

            for scope in &slot.subscribers {
                tracing::trace!("updating subcsriber");
                (self.update_any)(*scope);
            }
        } else {
            tracing::trace!("no atoms found for {:?}", ptr);
            atoms.insert(
                ptr,
                Slot {
                    value: Rc::new(value),
                    subscribers: HashSet::new(),
                },
            );
        }
    }

    pub fn unsubscribe(&self, ptr: AtomId, scope: ScopeId) {
        let mut atoms = self.atoms.borrow_mut();

        if let Some(slot) = atoms.get_mut(&ptr) {
            slot.subscribers.remove(&scope);
        }
    }

    // force update of all subscribers
    pub fn force_update(&self, ptr: AtomId) {
        if let Some(slot) = self.atoms.borrow_mut().get(&ptr) {
            for scope in slot.subscribers.iter() {
                tracing::trace!("updating subcsriber");
                (self.update_any)(*scope);
            }
        }
    }

    pub fn read<V: 'static>(&self, f: impl Readable<V>) -> Rc<V> {
        let mut atoms = self.atoms.borrow_mut();

        // initialize the value if it's not already initialized
        if let Some(slot) = atoms.get_mut(&f.unique_id()) {
            slot.value.clone().downcast().unwrap()
        } else {
            let value = Rc::new(f.init());
            atoms.insert(
                f.unique_id(),
                Slot {
                    value: value.clone(),
                    subscribers: HashSet::new(),
                },
            );
            value
        }
    }
}
