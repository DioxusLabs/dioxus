use std::rc::Rc;

use crate::AtomRoot;
use dioxus_core::ScopeState;

// Returns the atom root, initiaizing it at the root of the app if it does not exist.
pub fn use_atom_root(cx: &ScopeState) -> &Rc<AtomRoot> {
    cx.use_hook(|| match cx.consume_context::<Rc<AtomRoot>>() {
        Some(root) => root,
        None => panic!("No atom root found in context. Did you forget to call use_init_atom_root at the top of your app?"),
    })
}
