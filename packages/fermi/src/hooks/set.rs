use crate::{use_atom_root, Writable};
use dioxus_core::ScopeState;
use std::rc::Rc;

pub fn use_set<'a, T: 'static>(cx: &'a ScopeState, f: impl Writable<T>) -> &'a Rc<dyn Fn(T)> {
    let root = use_atom_root(cx);
    cx.use_hook(|_| {
        let id = f.unique_id();
        let root = root.clone();
        root.initialize(f);
        Rc::new(move |new| root.set(id, new)) as Rc<dyn Fn(T)>
    })
}
