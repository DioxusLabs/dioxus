use crate::{Atom, AtomId, AtomRoot, Readable, Writable};

pub struct AtomBuilder;

impl<V: 'static> Readable<V> for Atom<V> {
    fn read<'a>(&self, root: &'a AtomRoot) -> Option<&'a V> {
        dbg!(self.unique_id());

        let o = root.get_raw(self.unique_id()).downcast::<V>().unwrap();

        // give this thing a lifetime, but nix the lifetime later
        Some(root.arena.alloc(o))
    }
    fn init(&self) -> V {
        (*self)(AtomBuilder)
    }
    fn unique_id(&self) -> AtomId {
        AtomId::new(*self)
    }
}

impl<V: 'static> Writable<V> for Atom<V> {
    fn write(&self, _root: AtomRoot, _value: V) {
        todo!()
    }
}

#[test]
fn atom_compiles() {
    static TEST_ATOM: Atom<&str> = |_| "hello";
    dbg!(TEST_ATOM.init());
}
