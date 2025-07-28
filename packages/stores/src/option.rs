use crate::{CreateSelector, SelectorScope, SelectorStorage, Storable, Store};
use dioxus_signals::{MappedMutSignal, ReadableExt, UnsyncStorage, Writable};
use std::marker::PhantomData;

impl<T> Storable for Option<T> {
    type Store<View, S: SelectorStorage> = OptionSelector<View, T, S>;
}

pub struct OptionSelector<W, T, S: SelectorStorage = UnsyncStorage> {
    selector: SelectorScope<W, S>,
    _phantom: std::marker::PhantomData<T>,
}

impl<W, T, S: SelectorStorage> PartialEq for OptionSelector<W, T, S>
where
    W: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.selector == other.selector
    }
}

impl<W, T, S: SelectorStorage> Clone for OptionSelector<W, T, S>
where
    W: Clone,
{
    fn clone(&self) -> Self {
        Self {
            selector: self.selector.clone(),
            _phantom: PhantomData,
        }
    }
}

impl<W, T, S: SelectorStorage> Copy for OptionSelector<W, T, S> where W: Copy {}

impl<W, T, S: SelectorStorage> CreateSelector for OptionSelector<W, T, S> {
    type View = W;
    type Storage = S;

    fn new(selector: SelectorScope<Self::View, Self::Storage>) -> Self {
        Self {
            selector,
            _phantom: PhantomData,
        }
    }
}

impl<
        W: Writable<Target = Option<T>, Storage = S> + Copy + 'static,
        T: Storable + 'static,
        S: SelectorStorage,
    > OptionSelector<W, T, S>
{
    pub fn is_some(self) -> bool {
        self.selector.track();
        self.selector.write.read().is_some()
    }

    pub fn is_none(self) -> bool {
        self.selector.track();
        self.selector.write.read().is_none()
    }

    pub fn as_option(
        self,
    ) -> Option<
        Store<
            T,
            MappedMutSignal<
                T,
                W,
                impl Fn(&Option<T>) -> &T + Copy + 'static,
                impl Fn(&mut Option<T>) -> &mut T + Copy + 'static,
            >,
            S,
        >,
    >
    where
        T: Storable + 'static,
        W: Writable<Target = Option<T>, Storage = S> + Copy + 'static,
    {
        self.is_some().then(|| {
            T::Store::new(self.selector.scope(
                0,
                move |value: &Option<T>| {
                    value.as_ref().unwrap_or_else(|| {
                        panic!("Tried to access `Some` on an Option value");
                    })
                },
                move |value: &mut Option<T>| {
                    value.as_mut().unwrap_or_else(|| {
                        panic!("Tried to access `Some` on an Option value");
                    })
                },
            ))
        })
    }

    pub fn unwrap(
        self,
    ) -> Store<
        T,
        MappedMutSignal<
            T,
            W,
            impl Fn(&Option<T>) -> &T + Copy + 'static,
            impl Fn(&mut Option<T>) -> &mut T + Copy + 'static,
        >,
        S,
    > {
        self.as_option().unwrap()
    }
}
