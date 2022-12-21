use crate::{use_atom_root, AtomId, AtomRoot, Readable, Select, Selector};
use dioxus_core::{ScopeId, ScopeState};
use std::rc::Rc;

pub fn use_read<V: 'static>(cx: &ScopeState, f: impl Readable<V>) -> &V {
    use_read_rc(cx, f).as_ref()
}

pub fn use_read_rc<V: 'static>(cx: &ScopeState, f: impl Readable<V>) -> &Rc<V> {
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

    let inner = cx.use_hook(|| UseReadInner {
        value: None,
        root: root.clone(),
        scope_id: cx.scope_id(),
        id: f.unique_id(),
    });

    dbg!(inner.id);

    let value = inner.root.register(f, cx.scope_id());

    inner.value = Some(value);
    inner.value.as_ref().unwrap()
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

    // todo!()

    if root.root.needs_update(root.id) {
        if root.root.needs_selector_updated(selector) {
            // Create the value on the fly and then store it in the main atom root
            let s = Select::new(&root.root);
            let v = selector(s);

            let boxed = Box::new(v);
            let ptr = Box::into_raw(boxed);
            root.val = Some(ptr as _);

            root.root.update_selector(selector, ptr);
        } else {
            root.val = Some(root.root.get_selector(selector) as *mut ());
        }
    };

    // // gimme that pointer
    // let p: *mut () = root.val.unwrap() as _;
    // let r = p as *mut V;

    // unsafe { &*r }

    todo!("bong")
}
