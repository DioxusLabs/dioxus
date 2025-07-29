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
    T: std::ops::DerefMut + 'static,
{
    type Write = W;
    type Target = <T as std::ops::Deref>::Target;
    type Value = T;

    fn deref(self) -> Store<Self::Target, MappedMutSignal<Self::Target, Self::Write>> {
        Store::new(
            self.selector()
                .scope_raw(move |value| value.deref(), move |value| value.deref_mut()),
        )
    }
}
