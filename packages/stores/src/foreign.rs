use crate::{SelectorScope, Storable};
use dioxus_core::{IntoAttributeValue, IntoDynNode, Subscribers};
use dioxus_signals::{
    read_impls, write_impls, BorrowError, BorrowMutError, MappedMutSignal, Readable, ReadableExt,
    ReadableRef, UnsyncStorage, Writable, WritableExt, WritableRef, WriteSignal,
};
use std::{
    collections::{BTreeMap, BTreeSet, BinaryHeap, HashSet, LinkedList, VecDeque},
    ffi::{OsStr, OsString},
    marker::PhantomData,
    ops::Deref,
    path::{Path, PathBuf},
    rc::Rc,
    sync::Arc,
    time::Duration,
};

pub struct ForeignStore<T: ?Sized, W> {
    selector: SelectorScope<W>,
    phantom: PhantomData<T>,
}

impl<W, T: ?Sized> Clone for ForeignStore<T, W>
where
    W: Clone,
{
    fn clone(&self) -> Self {
        Self {
            selector: self.selector.clone(),
            phantom: PhantomData,
        }
    }
}

impl<W, T: ?Sized> Copy for ForeignStore<T, W> where W: Copy {}

impl<W, T: ?Sized> ForeignStore<T, W> {
    /// Creates a new `ForeignStore` with the given selector.
    pub fn new(selector: SelectorScope<W>) -> Self {
        Self {
            selector,
            phantom: PhantomData,
        }
    }
}

read_impls!(ForeignStore<T, W> where W: Readable<Target = T>);
write_impls!(ForeignStore<T, W> where W: Writable<Target = T>);

impl<T, W> IntoAttributeValue for ForeignStore<T, W>
where
    T: Clone + IntoAttributeValue + 'static,
    W: Writable<Target = T>,
{
    fn into_value(self) -> dioxus_core::AttributeValue {
        self.with(|f| f.clone().into_value())
    }
}

impl<T, W> IntoDynNode for ForeignStore<T, W>
where
    T: Clone + IntoDynNode + 'static,
    W: Writable<Target = T>,
{
    fn into_dyn_node(self) -> dioxus_core::DynamicNode {
        self.with(|f| f.clone().into_dyn_node())
    }
}

impl<T: Clone + 'static, W: Writable<Target = T> + 'static> Deref for ForeignStore<T, W> {
    type Target = dyn Fn() -> T;

    fn deref(&self) -> &Self::Target {
        unsafe { ReadableExt::deref_impl(self) }
    }
}

impl<W, T: ?Sized + 'static> Readable for ForeignStore<T, W>
where
    W: Readable<Target = T>,
{
    type Target = T;

    type Storage = W::Storage;

    fn try_read_unchecked(&self) -> Result<ReadableRef<'static, Self>, BorrowError> {
        self.selector.try_read_unchecked()
    }

    fn try_peek_unchecked(&self) -> Result<ReadableRef<'static, Self>, BorrowError> {
        self.selector.try_peek_unchecked()
    }

    fn subscribers(&self) -> Option<Subscribers> {
        self.selector.subscribers()
    }
}

impl<W, T: ?Sized + 'static> Writable for ForeignStore<T, W>
where
    W: Writable<Target = T>,
{
    type WriteMetadata = <W as Writable>::WriteMetadata;

    fn try_write_unchecked(&self) -> Result<WritableRef<'static, Self>, BorrowMutError> {
        self.selector.try_write_unchecked()
    }
}

impl<W: PartialEq, T: ?Sized> PartialEq for ForeignStore<T, W> {
    fn eq(&self, other: &Self) -> bool {
        self.selector == other.selector
    }
}

impl<
        T,
        W: Writable<Storage = UnsyncStorage> + 'static,
        F: Fn(&W::Target) -> &T + 'static,
        FMut: Fn(&mut W::Target) -> &mut T + 'static,
    > ::std::convert::From<ForeignStore<T, MappedMutSignal<T, W, F, FMut>>>
    for ForeignStore<T, WriteSignal<T>>
{
    fn from(value: ForeignStore<T, MappedMutSignal<T, W, F, FMut>>) -> Self {
        ForeignStore {
            selector: value.selector.map(::std::convert::Into::into),
            phantom: PhantomData,
        }
    }
}

macro_rules! mark_foreign_type {
    (
        // accept a path without angle brackets
        $ty:ident
        // Accept generics
        $(< $($gen:ident $(: $gen_bound:path)?),* >)?
        // Accept extra bounds
        $(
            where
                $(
                    $extra_bound_ty:ident: $extra_bound:path
                ),+
        )?) => {
        impl
        $(
            <
                $(
                    $gen $(: $gen_bound)?
                ),*
            >
        )?
        Storable for $ty $(< $($gen),* >)?
        where
            $($($extra_bound_ty: $extra_bound),*)?
        {
            type Store<View: Writable<Target = Self>> = ForeignStore<$ty $(< $($gen),* >)?, View>;

            fn create_selector<View: Writable<Target = Self>>(selector: SelectorScope<View>) -> Self::Store<View> {
                ForeignStore::new(selector)
            }
        }
    };
}

// Primitive foreign types
mark_foreign_type!(u8);
mark_foreign_type!(u16);
mark_foreign_type!(u32);
mark_foreign_type!(u64);
mark_foreign_type!(u128);
mark_foreign_type!(i8);
mark_foreign_type!(i16);
mark_foreign_type!(i32);
mark_foreign_type!(i64);
mark_foreign_type!(i128);
mark_foreign_type!(f32);
mark_foreign_type!(f64);
mark_foreign_type!(bool);
mark_foreign_type!(char);
mark_foreign_type!(usize);
mark_foreign_type!(isize);

// Common foreign types
mark_foreign_type!(String);
mark_foreign_type!(str);
mark_foreign_type!(Path);
mark_foreign_type!(PathBuf);
mark_foreign_type!(OsString);
mark_foreign_type!(OsStr);
mark_foreign_type!(Duration);

// Common foreign data structures
mark_foreign_type!(HashSet<T>);
mark_foreign_type!(BTreeMap<K, V>);
mark_foreign_type!(BTreeSet<T>);
mark_foreign_type!(LinkedList<T>);
mark_foreign_type!(BinaryHeap<T>);
mark_foreign_type!(VecDeque<T>);

mark_foreign_type!(Rc<T>);
mark_foreign_type!(Arc<T>);
