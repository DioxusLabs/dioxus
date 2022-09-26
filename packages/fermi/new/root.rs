use dioxus_core::prelude::ScopeId;
use dioxus_core::ScopeState;
use std::any::Any;
use std::cell::RefCell;
use std::collections::HashSet;
use std::sync::Arc;
use std::{collections::HashMap, rc::Rc};

pub struct AtomRoot {
    atoms: RefCell<HashMap<AtomId, Slot>>,
    update_any: Arc<dyn Fn(ScopeId)>,
    selections: RefCell<HashMap<SelectorId, Selection>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Dep {
    Atom(AtomId),
    Selector(SelectorId),
}

impl AtomRoot {
    pub(crate) fn new(update_any: Arc<dyn Fn(ScopeId)>) -> Self {
        Self {
            update_any,
            atoms: RefCell::new(HashMap::new()),
            selections: RefCell::new(HashMap::new()),
        }
    }

    fn register_selector<V>(&self, selector: Selector<V>, id: ScopeId) {}

    fn needs_selector_updated<V>(&self, selector: Selector<V>) -> bool {
        true
    }

    // Value is dirty but hasn't been regenerated
    fn needs_update(&self, id: ScopeId) -> bool {
        true
    }

    fn update_selector<V: PartialEq>(&self, selector: Selector<V>, value: *mut V) {
        let mut s = self.selections.borrow_mut();

        let s_ptr = selector as *const ();

        let selection = s.entry(s_ptr).or_insert_with(|| Selection {
            id: s_ptr,
            deps: HashSet::new(),
            val: value as *mut (),
        });

        unsafe {
            let old = selection.val as *const V;
            let new = value as *const V;

            let old = &*old;
            let new = &*new;
            if old != new {
                // update all dep selectors and components
            }
        }

        selection.val = value as *mut ();
    }

    fn get_selector<V>(&self, selector: Selector<V>) -> *mut V {
        let s = self.selections.borrow_mut();
        let s_ptr = selector as *const ();
        s.get(&s_ptr).map(|s| s.val as *mut V).unwrap()
    }
}

pub struct Slot {
    pub value: Rc<dyn Any>,
    pub subscribers: HashSet<ScopeId>,
}
