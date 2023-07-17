use crate::{AtomId, AtomRoot, Readable};
use std::cell::RefCell;

pub struct AtomRefBuilder;
pub struct AtomRef<T>(pub fn(AtomRefBuilder) -> T);

impl<V> Readable<RefCell<V>> for &'static AtomRef<V> {
    fn read(&self, _root: AtomRoot) -> Option<RefCell<V>> {
        todo!()
    }

    fn init(&self) -> RefCell<V> {
        RefCell::new(self.0(AtomRefBuilder))
    }

    fn unique_id(&self) -> AtomId {
        *self as *const AtomRef<V> as *const ()
    }
}

#[test]
fn atom_compiles() {
    static TEST_ATOM: AtomRef<Vec<String>> = AtomRef(|_| vec![]);
    dbg!((&TEST_ATOM).init());
}
