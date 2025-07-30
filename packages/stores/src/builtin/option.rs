use crate::store::Store;
use dioxus_signals::{MappedMutSignal, ReadableExt, Writable};

pub trait OptionStoreExt {
    type Data;
    type Write;

    fn is_some(self) -> bool;
    fn is_none(self) -> bool;

    fn as_option(
        self,
    ) -> Option<
        Store<
            Self::Data,
            MappedMutSignal<
                Self::Data,
                Self::Write,
                impl Fn(&Option<Self::Data>) -> &Self::Data + Copy + 'static,
                impl Fn(&mut Option<Self::Data>) -> &mut Self::Data + Copy + 'static,
            >,
        >,
    >;

    fn unwrap(
        self,
    ) -> Store<
        Self::Data,
        MappedMutSignal<
            Self::Data,
            Self::Write,
            impl Fn(&Option<Self::Data>) -> &Self::Data + Copy + 'static,
            impl Fn(&mut Option<Self::Data>) -> &mut Self::Data + Copy + 'static,
        >,
    >;
}

impl<W: Writable<Target = Option<T>> + Copy + 'static, T: 'static> OptionStoreExt
    for Store<Option<T>, W>
{
    type Data = T;
    type Write = W;

    fn is_some(self) -> bool {
        self.selector().track_shallow();
        self.selector().write.read().is_some()
    }

    fn is_none(self) -> bool {
        self.selector().track_shallow();
        self.selector().write.read().is_none()
    }

    fn as_option(
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
    > {
        self.is_some().then(|| {
            self.selector()
                .child(
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
                )
                .into()
        })
    }

    fn unwrap(
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
