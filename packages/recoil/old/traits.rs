use crate::{Atom, AtomFamily};

use std::hash::Hash;

pub trait FamilyKey: PartialEq + Hash {}
impl<T: PartialEq + Hash> FamilyKey for T {}

pub trait AtomValue: PartialEq {}
impl<T: PartialEq> AtomValue for T {}

pub trait Readable<T>: 'static {
    fn load(&'static self) -> RecoilItem;
}
