use crate::{AtomId, AtomRoot, Readable, Writable};

pub struct Atom<T>(pub fn(AtomBuilder) -> T);
pub struct AtomBuilder;

impl<V> Readable<V> for &'static Atom<V> {
    fn read(&self, _root: AtomRoot) -> Option<V> {
        todo!()
    }
    fn init(&self) -> V {
        self.0(AtomBuilder)
    }
    fn unique_id(&self) -> AtomId {
        *self as *const Atom<V> as *const ()
    }
}

impl<V> Writable<V> for &'static Atom<V> {
    fn write(&self, _root: AtomRoot, _value: V) {
        todo!()
    }
}

#[test]
fn atom_compiles() {
    static TEST_ATOM: Atom<&str> = Atom(|_| "hello");
    dbg!((&TEST_ATOM).init());
}

#[test]
fn atom_is_unique() {
    static TEST_ATOM_1: Atom<&str> = Atom(|_| "hello");
    static TEST_ATOM_2: Atom<&str> = Atom(|_| "hello");
    assert_eq!((&TEST_ATOM_1).unique_id(), (&TEST_ATOM_1).unique_id());
    assert_ne!((&TEST_ATOM_1).unique_id(), (&TEST_ATOM_2).unique_id());
}

#[test]
fn atom_is_unique_2() {
    struct S(String);
    static TEST_ATOM_1: Atom<Vec<S>> = Atom(|_| Vec::new());
    static TEST_ATOM_2: Atom<Vec<String>> = Atom(|_| Vec::new());
    assert_ne!((&TEST_ATOM_1).unique_id(), (&TEST_ATOM_2).unique_id());
}
