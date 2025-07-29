use crate::{store_impls, store_read_impls, SelectorScope, Storable};
use dioxus_signals::{MappedMutSignal, UnsyncStorage, Writable, WriteSignal};
use std::{
    collections::{BTreeMap, BTreeSet, BinaryHeap, HashSet, LinkedList, VecDeque},
    ffi::OsString,
    marker::PhantomData,
    path::PathBuf,
    rc::Rc,
    sync::Arc,
    time::Duration,
};

pub struct ForeignStore<W, T> {
    selector: SelectorScope<W>,
    _phantom: PhantomData<T>,
}

impl<W, T> ForeignStore<W, T> {
    /// Creates a new `ForeignStore` with the given selector.
    pub fn new(selector: SelectorScope<W>) -> Self {
        Self {
            selector,
            _phantom: PhantomData,
        }
    }
}

store_impls!(T => ForeignStore<W, T>);
store_read_impls!(T => ForeignStore<W, T>);

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
            type Store<View: Writable<Target = Self>> = ForeignStore<View, $ty $(< $($gen),* >)?>;

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
mark_foreign_type!(PathBuf);
mark_foreign_type!(OsString);
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
