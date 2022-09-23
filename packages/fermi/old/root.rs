use std::{
    any::{Any, TypeId},
    cell::RefCell,
    collections::HashMap,
    rc::Rc,
    sync::Arc,
};

use dioxus_core::{Element, Scope, ScopeId, ScopeState};
use im_rc::HashSet;

use crate::{Readable, Select, Selector};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AtomId {
    pub(crate) ptr: *const (),
    pub(crate) type_name: &'static str,
}

pub fn AtomRoot(cx: Scope) -> Element {
    todo!()
}

pub struct AtomRoot {
    pub atoms: RefCell<HashMap<AtomId, Slot>>,
    pub update_any: Arc<dyn Fn(ScopeId)>,
}

pub struct Slot {
    pub value: Rc<dyn Any>,
    pub subscribers: HashSet<ScopeId>,
}

impl AtomRoot {
    pub(crate) fn new(update_any: Arc<dyn Fn(ScopeId)>) -> Self {
        Self {
            update_any,
            atoms: RefCell::new(HashMap::new()),
        }
    }

    pub(crate) fn initialize<V: 'static>(&self, f: impl Readable<V>) {
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

    pub(crate) fn select<'a, O: ?Sized + 'a>(&'a self, f: Selector<'a, O>) -> Rc<&'a O> {
        let selector = Select::new(
            self,
            AtomId {
                ptr: f as *const (),
                type_name: todo!(),
            },
        );

        todo!()
    }

    pub(crate) fn get<V>(&self, f: impl Readable<V>) -> Rc<V> {
        todo!()
    }

    pub(crate) fn register<V: 'static>(&self, f: impl Readable<V>, scope: ScopeId) -> Rc<V> {
        let mut atoms = self.atoms.borrow_mut();

        // initialize the value if it's not already initialized
        if let Some(slot) = atoms.get_mut(&f.unique_id()) {
            slot.subscribers.insert(scope);
            match slot.value.clone().downcast() {
                Ok(res) => res,
                Err(e) => panic!(
                    "Downcasting atom failed: {:?}. Has typeid of {:?} but needs typeid of {:?}",
                    f.unique_id(),
                    e.type_id(),
                    TypeId::of::<V>()
                ),
            }
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

    pub(crate) fn set<V: 'static>(&self, ptr: AtomId, value: V) {
        let mut atoms = self.atoms.borrow_mut();

        if let Some(slot) = atoms.get_mut(&ptr) {
            slot.value = Rc::new(value);
            log::trace!("found item with subscribers {:?}", slot.subscribers);

            for scope in &slot.subscribers {
                log::trace!("updating subcsriber");
                (self.update_any)(*scope);
            }
        } else {
            log::trace!("no atoms found for {:?}", ptr);
            atoms.insert(
                ptr,
                Slot {
                    value: Rc::new(value),
                    subscribers: HashSet::new(),
                },
            );
        }
    }

    pub(crate) fn unsubscribe(&self, ptr: AtomId, scope: ScopeId) {
        let mut atoms = self.atoms.borrow_mut();

        if let Some(slot) = atoms.get_mut(&ptr) {
            slot.subscribers.remove(&scope);

            if slot.subscribers.is_empty() {
                // drop the value?
            }
        }
    }

    // force update of all subscribers
    pub(crate) fn force_update(&self, ptr: AtomId) {
        if let Some(slot) = self.atoms.borrow_mut().get(&ptr) {
            for scope in slot.subscribers.iter() {
                log::trace!("updating subcsriber");
                (self.update_any)(*scope);
            }
        }
    }

    pub(crate) fn read<V: 'static>(&self, f: impl Readable<V>) -> Rc<V> {
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
