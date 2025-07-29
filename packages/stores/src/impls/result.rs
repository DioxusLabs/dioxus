use crate::{CreateSelector, SelectorScope, Storable, Store};
use dioxus_signals::{MappedMutSignal, ReadableExt, UnsyncStorage, Writable, WriteSignal};
use std::marker::PhantomData;

impl<T, E> Storable for Result<T, E> {
    type Store<View> = ResultSelector<View, T, E>;
}

pub struct ResultSelector<W, T, E> {
    selector: SelectorScope<W>,
    _phantom: std::marker::PhantomData<(T, E)>,
}

impl<W, T, E> PartialEq for ResultSelector<W, T, E>
where
    W: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.selector == other.selector
    }
}

impl<W, T, E> Clone for ResultSelector<W, T, E>
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

impl<W, T, E> Copy for ResultSelector<W, T, E> where W: Copy {}

impl<
        T,
        E,
        W: Writable<Storage = UnsyncStorage> + 'static,
        F: Fn(&W::Target) -> &Result<T, E> + 'static,
        FMut: Fn(&mut W::Target) -> &mut Result<T, E> + 'static,
    > ::std::convert::From<ResultSelector<MappedMutSignal<Result<T, E>, W, F, FMut>, T, E>>
    for ResultSelector<WriteSignal<Result<T, E>>, T, E>
{
    fn from(value: ResultSelector<MappedMutSignal<Result<T, E>, W, F, FMut>, T, E>) -> Self {
        ResultSelector {
            selector: value.selector.map(::std::convert::Into::into),
            _phantom: PhantomData,
        }
    }
}

impl<W, T, E> CreateSelector for ResultSelector<W, T, E> {
    type View = W;

    fn new(selector: SelectorScope<Self::View>) -> Self {
        Self {
            selector,
            _phantom: PhantomData,
        }
    }
}

impl<
        W: Writable<Target = Result<T, E>> + Copy + 'static,
        T: Storable + 'static,
        E: Storable + 'static,
    > ResultSelector<W, T, E>
{
    pub fn is_ok(self) -> bool {
        self.selector.track();
        self.selector.write.read().is_ok()
    }

    pub fn is_err(self) -> bool {
        self.selector.track();
        self.selector.write.read().is_err()
    }

    pub fn ok(
        self,
    ) -> Option<
        Store<
            T,
            MappedMutSignal<
                T,
                W,
                impl Fn(&Result<T, E>) -> &T + Copy + 'static,
                impl Fn(&mut Result<T, E>) -> &mut T + Copy + 'static,
            >,
        >,
    >
    where
        T: Storable + 'static,
        W: Writable<Target = Result<T, E>> + Copy + 'static,
    {
        self.is_ok().then(|| {
            T::Store::new(self.selector.scope(
                0,
                move |value: &Result<T, E>| {
                    value.as_ref().unwrap_or_else(|_| {
                        panic!("Tried to access `ok` on an Err value");
                    })
                },
                move |value: &mut Result<T, E>| {
                    value.as_mut().unwrap_or_else(|_| {
                        panic!("Tried to access `ok` on an Err value");
                    })
                },
            ))
        })
    }

    pub fn err(
        self,
    ) -> Option<
        Store<
            E,
            MappedMutSignal<
                E,
                W,
                impl Fn(&Result<T, E>) -> &E + Copy + 'static,
                impl Fn(&mut Result<T, E>) -> &mut E + Copy + 'static,
            >,
        >,
    >
    where
        T: Storable + 'static,
        W: Writable<Target = Result<T, E>> + Copy + 'static,
    {
        self.is_err().then(|| {
            E::Store::new(self.selector.scope(
                0,
                move |value: &Result<T, E>| match value {
                    Ok(_) => panic!("Tried to access `err` on an Ok value"),
                    Err(e) => e,
                },
                move |value: &mut Result<T, E>| match value {
                    Ok(_) => panic!("Tried to access `err` on an Ok value"),
                    Err(e) => e,
                },
            ))
        })
    }

    pub fn as_result(
        self,
    ) -> Result<
        Store<
            T,
            MappedMutSignal<
                T,
                W,
                impl Fn(&Result<T, E>) -> &T + Copy + 'static,
                impl Fn(&mut Result<T, E>) -> &mut T + Copy + 'static,
            >,
        >,
        Store<
            E,
            MappedMutSignal<
                E,
                W,
                impl Fn(&Result<T, E>) -> &E + Copy + 'static,
                impl Fn(&mut Result<T, E>) -> &mut E + Copy + 'static,
            >,
        >,
    >
    where
        T: Storable + 'static,
        W: Writable<Target = Result<T, E>> + Copy + 'static,
    {
        if self.is_ok() {
            Ok(T::Store::new(self.selector.scope(
                0,
                move |value: &Result<T, E>| {
                    value.as_ref().unwrap_or_else(|_| {
                        panic!("Tried to access `ok` on an Err value");
                    })
                },
                move |value: &mut Result<T, E>| {
                    value.as_mut().unwrap_or_else(|_| {
                        panic!("Tried to access `ok` on an Err value");
                    })
                },
            )))
        } else {
            Err(E::Store::new(self.selector.scope(
                0,
                move |value: &Result<T, E>| match value {
                    Ok(_) => panic!("Tried to access `err` on an Ok value"),
                    Err(e) => e,
                },
                move |value: &mut Result<T, E>| match value {
                    Ok(_) => panic!("Tried to access `err` on an Ok value"),
                    Err(e) => e,
                },
            )))
        }
    }
}
