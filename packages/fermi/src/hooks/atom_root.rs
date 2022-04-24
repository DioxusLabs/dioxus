use crate::AtomRoot;
use dioxus_core::ScopeState;
use std::rc::Rc;

// Returns the atom root, initiaizing it at the root of the app if it does not exist.
pub fn use_atom_root(cx: &ScopeState) -> &Rc<AtomRoot> {
    cx.use_hook(|_| match cx.consume_context::<Rc<AtomRoot>>() {
        Some(root) => root,
        None => cx.provide_root_context(Rc::new(AtomRoot::new(cx.schedule_update_any()))),
    })
}
