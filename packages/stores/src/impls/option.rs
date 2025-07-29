use crate::{SelectorScope, Storable, Store};
use dioxus_signals::{MappedMutSignal, ReadableExt, UnsyncStorage, Writable, WriteSignal};
use std::marker::PhantomData;

impl<T> Storable for Option<T> {
    type Store<View: Writable<Target = Self>> = OptionSelector<View, T>;

    fn create_selector<View: Writable<Target = Self>>(
        selector: SelectorScope<View>,
    ) -> Self::Store<View> {
        OptionSelector::new(selector)
    }
}

pub struct OptionSelector<W, T> {
    selector: SelectorScope<W>,
    _phantom: std::marker::PhantomData<T>,
}

impl<W, T> PartialEq for OptionSelector<W, T>
where
    W: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.selector == other.selector
    }
}

impl<W, T> Clone for OptionSelector<W, T>
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

impl<W, T> Copy for OptionSelector<W, T> where W: Copy {}

impl<
        T,
        W: Writable<Storage = UnsyncStorage> + 'static,
        F: Fn(&W::Target) -> &Option<T> + 'static,
        FMut: Fn(&mut W::Target) -> &mut Option<T> + 'static,
    > ::std::convert::From<OptionSelector<MappedMutSignal<Option<T>, W, F, FMut>, T>>
    for OptionSelector<WriteSignal<Option<T>>, T>
{
    fn from(value: OptionSelector<MappedMutSignal<Option<T>, W, F, FMut>, T>) -> Self {
        OptionSelector {
            selector: value.selector.map(::std::convert::Into::into),
            _phantom: PhantomData,
        }
    }
}

impl<W, T> OptionSelector<W, T> {
    fn new(selector: SelectorScope<W>) -> Self {
        Self {
            selector,
            _phantom: PhantomData,
        }
    }
}

impl<W: Writable<Target = Option<T>> + Copy + 'static, T: Storable + 'static> OptionSelector<W, T> {
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
        >,
    >
    where
        T: Storable + 'static,
        W: Writable<Target = Option<T>> + Copy + 'static,
    {
        self.is_some().then(|| {
            T::create_selector(self.selector.scope(
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
    > {
        self.as_option().unwrap()
    }
}
