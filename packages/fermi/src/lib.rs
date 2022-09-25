use std::any::Any;
use std::cell::RefCell;
use std::collections::HashSet;
use std::sync::Arc;
use std::{collections::HashMap, rc::Rc};

use dioxus_core::prelude::ScopeId;
use dioxus_core::ScopeState;

pub type AtomId = &'static str;
type SelectorId = *const ();

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Dep {
    Atom(AtomId),
    Selector(SelectorId),
}

pub struct AtomRoot {
    atoms: RefCell<HashMap<AtomId, Slot>>,
    update_any: Arc<dyn Fn(ScopeId)>,
    selections: RefCell<HashMap<SelectorId, Selection>>,
}

struct Selection {
    id: SelectorId,
    deps: HashSet<Dep>,
    val: *mut (),
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

#[derive(Debug)]
pub struct Atom<V> {
    f: fn() -> V,
    id: AtomId,
}

impl<V> Clone for Atom<V> {
    fn clone(&self) -> Self {
        Self {
            f: self.f,
            id: self.id,
        }
    }
}
impl<V> Copy for Atom<V> {}

pub type Selector<V> = fn(Select) -> V;

pub struct Select<'a> {
    root: &'a AtomRoot,
}

impl<'a> Select<'a> {
    pub fn get<V: Default + 'static>(&self, atom: Atom<V>) -> &'a V {
        let def: V = (atom.f)();
        let mut atoms = self.root.atoms.borrow_mut();

        let atom = atoms.entry(atom.id).or_insert_with(|| Slot {
            value: Rc::new(def),
            subscribers: HashSet::new(),
        });

        // recast that lifetime bb
        let p: &V = atom.value.as_ref().downcast_ref().unwrap();

        unsafe { std::mem::transmute(p) }
    }
    pub fn select<V>(&self, selector: fn(Select<'a>) -> V) -> &'a V {
        todo!()
    }
}

pub const fn atom<V>(f: fn() -> V) -> Atom<V> {
    let id = file!();
    Atom { f, id }
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
