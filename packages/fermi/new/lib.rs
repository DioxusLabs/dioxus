use dioxus_core::prelude::ScopeId;
use dioxus_core::ScopeState;
use std::any::Any;
use std::cell::RefCell;
use std::collections::{HashSet, VecDeque};
use std::sync::Arc;
use std::{collections::HashMap, rc::Rc};

mod aliases;
pub use aliases::*;

mod root;

mod hooks {
    mod usestate;
}

pub struct AtomRoot {
    update_any: Arc<dyn Fn(ScopeId)>,
    atoms: RefCell<HashMap<AtomId, Slot>>,
    selections: RefCell<HashMap<SelectorId, Selection>>,
}

struct Selection {
    id: SelectorId,
    deps: HashSet<Dep>,
    val: *mut (),
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

        let s_ptr = SelectorId(selector as *const ());

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

            // if old != new {
            //     // update all dep selectors and components
            // }
        }

        selection.val = value as *mut ();
    }

    fn update_atom<V: 'static>(&self, atom: Atom<V>, value: V) {
        let mut atoms = self.atoms.borrow_mut();

        let id = atom.static_id();

        let mut slot = atoms.get_mut(&id).expect("Atom to already be initialized");

        // Set the new value
        slot.value = Rc::new(value);

        // iterate all subscribers and force them dirty
        for sub in slot.subscribed_scopes.iter() {
            (self.update_any)(*sub);
        }

        // iterate all selectors and force their subscription sites to be dirty and require regeneration
        let mut selections = self.selections.borrow_mut();

        let mut queue = VecDeque::new();
        queue.extend(slot.subscribed_selectors.iter().copied());

        while let Some(selector) = queue.pop_front() {
            let selection = selections.get_mut(&selector).expect("Selector to exist");

            for dep in selection.deps.iter() {
                match dep {
                    Dep::Atom(atom) => {
                        let slot = atoms.get_mut(atom).expect("Atom to exist");
                        for sub in slot.subscribed_scopes.iter() {
                            (self.update_any)(*sub);
                        }
                    }
                    Dep::Selector(selector) => {
                        queue.push_back(*selector);
                    }
                }
            }
        }
    }

    fn mark_selector_as_dirty(&self, selector: SelectorId) {}

    fn get_selector<V>(&self, selector: Selector<V>) -> *mut V {
        let s = self.selections.borrow_mut();
        let s_ptr = SelectorId(selector as *const ());
        s.get(&s_ptr).map(|s| s.val as *mut V).unwrap()
    }
}

pub struct Slot {
    pub value: Rc<dyn Any>,
    pub subscribed_scopes: HashSet<ScopeId>,
    pub subscribed_selectors: HashSet<SelectorId>,
}

impl Slot {
    pub fn new(value: Rc<dyn Any>) -> Self {
        Self {
            value,
            subscribed_scopes: HashSet::new(),
            subscribed_selectors: HashSet::new(),
        }
    }
}

pub struct Select<'a> {
    root: &'a AtomRoot,
}

impl<'a> Select<'a> {
    pub fn get<V: Default + 'static>(&self, atom: Atom<V>) -> &'a V {
        let mut atoms = self.root.atoms.borrow_mut();

        let def = atom();

        let atom = atoms.entry(atom.static_id()).or_insert_with(|| Slot {
            value: Rc::new(def),
            subscribed_scopes: HashSet::new(),
            subscribed_selectors: HashSet::new(),
        });

        // recast that lifetime bb
        let p: &V = atom.value.as_ref().downcast_ref().unwrap();

        unsafe { std::mem::transmute(p) }
    }

    pub fn select<V>(&self, selector: fn(Select<'a>) -> V) -> &'a V {
        todo!()
    }
}

pub fn use_selector<'a, V: PartialEq>(cx: &'a ScopeState, selector: fn(Select<'a>) -> V) -> &'a V {
    let root = use_atom_root(cx);

    struct UseSelector {
        root: Rc<AtomRoot>,
        id: ScopeId,
        val: Option<*mut ()>,
    }

    let selector: Selector<V> = unsafe { std::mem::transmute(selector) };

    let mut root = cx.use_hook(|| {
        let id = cx.scope_id();

        // massage the lifetimes so that we can store the pointer in the hook
        root.register_selector(selector, id);

        UseSelector {
            root: root.clone(),
            val: None,
            id,
        }
    });

    if root.root.needs_update(root.id) {
        if root.root.needs_selector_updated(selector) {
            // Create the value on the fly and then store it in the main atom root
            let s = Select { root: &root.root };
            let v = selector(s);
            let boxed = Box::new(v);
            let ptr = Box::into_raw(boxed);
            root.val = Some(ptr as _);

            root.root.update_selector(selector, ptr);
        } else {
            root.val = Some(root.root.get_selector(selector) as *mut ());
        }
    };

    // gimme that pointer
    let p: *mut () = root.val.unwrap() as _;
    let r = p as *mut V;

    unsafe { &*r }
}

// Returns the atom root, initiaizing it at the root of the app if it does not exist.
pub fn use_atom_root(cx: &ScopeState) -> &Rc<AtomRoot> {
    cx.use_hook(|| match cx.consume_context::<Rc<AtomRoot>>() {
        Some(root) => root,
        None => cx.provide_root_context(Rc::new(AtomRoot::new(cx.schedule_update_any()))),
    })
}
