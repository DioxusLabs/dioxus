use std::ops::{Deref, DerefMut};

use crate::store::Store;
use dioxus_signals::{MappedMutSignal, Writable};

pub trait DerefStoreExt {
    type Write;
    type Value;
    type Target: ?Sized;

    fn deref(
        self,
    ) -> Store<
        Self::Target,
        MappedMutSignal<
            Self::Target,
            Self::Write,
            fn(&Self::Value) -> &Self::Target,
            fn(&mut Self::Value) -> &mut Self::Target,
        >,
    >;
}

impl<W, T> DerefStoreExt for Store<T, W>
where
    W: Writable<Target = T> + Copy + 'static,
    T: DerefMut + 'static,
{
    type Write = W;
    type Target = <T as Deref>::Target;
    type Value = T;

    fn deref(self) -> Store<Self::Target, MappedMutSignal<Self::Target, Self::Write>> {
        let map: fn(&T) -> &Self::Target = |value| value.deref();
        let map_mut: fn(&mut T) -> &mut Self::Target = |value| value.deref_mut();
        self.selector().scope_raw(map, map_mut).into()
    }
}
