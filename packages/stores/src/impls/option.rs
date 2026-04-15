//! All `Store<Option<T>, _>` projection methods live on the
//! [`ProjectOption`](crate::ProjectOption) trait in `project.rs`. Since
//! `Store` implements [`Project`](crate::Project), every trait method is
//! callable directly on `Store<Option<T>, _>`.
//!
//! `as_slice` stays here — it produces a `Store<[T], _>` (slice DST view),
//! which doesn't fit the generic `Project::Child` shape.

use crate::{store::Store, MappedStore};
use dioxus_signals::Readable;

impl<Lens, T> Store<Option<T>, Lens>
where
    Lens: Readable<Target = Option<T>> + Copy + 'static,
    T: 'static,
{
    /// Return an `[T]` view of the option: `&[value]` if `Some`, `&[]` if `None`.
    pub fn as_slice(self) -> MappedStore<[T], Lens> {
        let map: fn(&Option<T>) -> &[T] = |value| value.as_slice();
        let map_mut: fn(&mut Option<T>) -> &mut [T] = |value| value.as_mut_slice();
        self.into_selector().map(map, map_mut).into()
    }
}
