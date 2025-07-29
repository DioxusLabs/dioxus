use crate::{SelectorScope, Storable, Store};
use dioxus_signals::{MappedMutSignal, Writable};
use std::ops::{Deref, DerefMut};

fn deref_selector<W>(
    selector: SelectorScope<W>,
) -> Store<<W::Target as Deref>::Target, MappedMutSignal<<W::Target as Deref>::Target, W>>
where
    W: Writable,
    W::Target: DerefMut + 'static,
    <W::Target as Deref>::Target: Storable + 'static,
{
    let selector = selector.scope_raw(
        (|value| value.deref()) as fn(&W::Target) -> &<W::Target as Deref>::Target,
        (|value| value.deref_mut()) as fn(&mut W::Target) -> &mut <W::Target as Deref>::Target,
    );
    <W::Target as Deref>::Target::create_selector(selector)
}

impl<T: Storable + 'static> Storable for Box<T> {
    type Store<View: Writable<Target = Self>> = Store<T, MappedMutSignal<T, View>>;

    fn create_selector<View: Writable<Target = Self>>(
        selector: SelectorScope<View>,
    ) -> Self::Store<View> {
        deref_selector(selector)
    }
}
