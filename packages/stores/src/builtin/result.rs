use crate::store::Store;
use dioxus_signals::{MappedMutSignal, ReadableExt, Writable};

pub trait ResultStoreExt {
    type Ok;
    type Err;
    type Write;

    fn is_ok(self) -> bool;

    fn is_err(self) -> bool;

    fn ok(
        self,
    ) -> Option<
        Store<
            Self::Ok,
            MappedMutSignal<
                Self::Ok,
                Self::Write,
                impl Fn(&Result<Self::Ok, Self::Err>) -> &Self::Ok + Copy + 'static,
                impl Fn(&mut Result<Self::Ok, Self::Err>) -> &mut Self::Ok + Copy + 'static,
            >,
        >,
    >;

    fn err(
        self,
    ) -> Option<
        Store<
            Self::Err,
            MappedMutSignal<
                Self::Err,
                Self::Write,
                impl Fn(&Result<Self::Ok, Self::Err>) -> &Self::Err + Copy + 'static,
                impl Fn(&mut Result<Self::Ok, Self::Err>) -> &mut Self::Err + Copy + 'static,
            >,
        >,
    >;

    fn as_result(
        self,
    ) -> Result<
        Store<
            Self::Ok,
            MappedMutSignal<
                Self::Ok,
                Self::Write,
                impl Fn(&Result<Self::Ok, Self::Err>) -> &Self::Ok + Copy + 'static,
                impl Fn(&mut Result<Self::Ok, Self::Err>) -> &mut Self::Ok + Copy + 'static,
            >,
        >,
        Store<
            Self::Err,
            MappedMutSignal<
                Self::Err,
                Self::Write,
                impl Fn(&Result<Self::Ok, Self::Err>) -> &Self::Err + Copy + 'static,
                impl Fn(&mut Result<Self::Ok, Self::Err>) -> &mut Self::Err + Copy + 'static,
            >,
        >,
    >;
}

impl<W, T, E> ResultStoreExt for Store<Result<T, E>, W>
where
    W: Writable<Target = Result<T, E>> + Copy + 'static,
    T: 'static,
    E: 'static,
{
    type Ok = T;
    type Err = E;
    type Write = W;

    fn is_ok(self) -> bool {
        self.selector().track();
        self.selector().write.read().is_ok()
    }

    fn is_err(self) -> bool {
        self.selector().track();
        self.selector().write.read().is_err()
    }

    fn ok(
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
    > {
        self.is_ok().then(|| {
            Store::new(self.selector().scope(
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

    fn err(
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
        W: Writable<Target = Result<T, E>> + Copy + 'static,
    {
        self.is_err().then(|| {
            Store::new(self.selector().scope(
                1,
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

    fn as_result(
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
        W: Writable<Target = Result<T, E>> + Copy + 'static,
    {
        if self.is_ok() {
            Ok(Store::new(self.selector().scope(
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
            Err(Store::new(self.selector().scope(
                1,
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
