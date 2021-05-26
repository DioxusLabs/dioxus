//! Provide a memoized wrapper around collections for efficient updates.
//! --------------------------------------------------------------------
//!

use crate::{AtomValue, FamilyKey, Readable, RecoilItem};
#[allow(non_camel_case_types)]
pub struct atom_family<K: FamilyKey, V: AtomValue>(pub fn(&mut AtomFamilyBuilder<K, V>));
pub type AtomFamily<K, V> = atom_family<K, V>;

// impl<K: FamilyKey, V: AtomValue> Readable for &'static AtomFamily<K, V> {
//     fn load(&self) -> RecoilItem {
//         RecoilItem::Atom(*self as *const _ as _)
//     }
// }

pub struct AtomFamilyBuilder<K, V> {
    _never: std::marker::PhantomData<(K, V)>,
}

impl<K: FamilyKey, V: AtomValue> atom_family<K, V> {
    fn select(&'static self, key: &K) -> FamilySelected {
        todo!()
    }
}

struct FamilySelected {}
