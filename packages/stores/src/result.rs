use crate::{CreateSelector, SelectorScope, SelectorStorage, Storable, Store};
use dioxus_signals::{MappedMutSignal, ReadableExt, UnsyncStorage, Writable};
use std::marker::PhantomData;

impl<T, E> Storable for Result<T, E> {
    type Store<View, S: SelectorStorage> = ResultSelector<View, T, E, S>;
}

pub struct ResultSelector<W, T, E, S: SelectorStorage = UnsyncStorage> {
    selector: SelectorScope<W, S>,
    _phantom: std::marker::PhantomData<(T, E)>,
}

impl<W, T, E, S: SelectorStorage> PartialEq for ResultSelector<W, T, E, S>
where
    W: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.selector == other.selector
    }
}

impl<W, T, E, S: SelectorStorage> Clone for ResultSelector<W, T, E, S>
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

impl<W, T, E, S: SelectorStorage> Copy for ResultSelector<W, T, E, S> where W: Copy {}

impl<W, T, E, S: SelectorStorage> CreateSelector for ResultSelector<W, T, E, S> {
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
        W: Writable<Target = Result<T, E>, Storage = S> + Copy + 'static,
        T: Storable + 'static,
        E: Storable + 'static,
        S: SelectorStorage,
    > ResultSelector<W, T, E, S>
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
            S,
        >,
    >
    where
        T: Storable + 'static,
        W: Writable<Target = Result<T, E>, Storage = S> + Copy + 'static,
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
            S,
        >,
    >
    where
        T: Storable + 'static,
        W: Writable<Target = Result<T, E>, Storage = S> + Copy + 'static,
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
            S,
        >,
        Store<
            E,
            MappedMutSignal<
                E,
                W,
                impl Fn(&Result<T, E>) -> &E + Copy + 'static,
                impl Fn(&mut Result<T, E>) -> &mut E + Copy + 'static,
            >,
            S,
        >,
    >
    where
        T: Storable + 'static,
        W: Writable<Target = Result<T, E>, Storage = S> + Copy + 'static,
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
