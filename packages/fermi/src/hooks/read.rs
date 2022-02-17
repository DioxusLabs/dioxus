use crate::{use_atom_root, AtomId, AtomRoot, Readable};
use dioxus_core::{ScopeId, ScopeState};
use std::rc::Rc;

pub fn use_read<'a, V: 'static>(cx: &'a ScopeState, f: impl Readable<V>) -> &'a V {
    use_read_rc(cx, f).as_ref()
}

pub fn use_read_rc<'a, V: 'static>(cx: &'a ScopeState, f: impl Readable<V>) -> &'a Rc<V> {
    let root = use_atom_root(cx);

    struct UseReadInner<V> {
        root: Rc<AtomRoot>,
        id: AtomId,
        scope_id: ScopeId,
        value: Option<Rc<V>>,
    }

    impl<V> Drop for UseReadInner<V> {
        fn drop(&mut self) {
            self.root.unsubscribe(self.id, self.scope_id)
        }
    }

    let inner = cx.use_hook(|_| UseReadInner {
        value: None,
        root: root.clone(),
        scope_id: cx.scope_id(),
        id: f.unique_id(),
    });

    let value = inner.root.register(f, cx.scope_id());

    inner.value = Some(value);
    inner.value.as_ref().unwrap()
}
