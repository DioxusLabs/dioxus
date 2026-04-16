use std::{
    cell::Ref,
    collections::{BTreeMap, HashMap},
    future::Future,
    hash::BuildHasher,
    marker::PhantomData,
};

use dioxus_signals::{
    CopyValue, ReadableExt, ReadableRef, UnsyncStorage, WritableExt, WritableRef, WriteLock,
};
use generational_box::GenerationalRef;

use crate::{
    collection::{EachBTreeMap, EachHashMap, EachVec, FlattenSome, FlattenSomeOp},
    combinator::{
        Combinator, FutureProjection, LensOp, ReadProjection, ReadProjectionOpt, UnwrapErrOp,
        UnwrapErrOptionalOp, UnwrapOkOp, UnwrapOkOptionalOp, UnwrapSomeOp, UnwrapSomeOptionalOp,
        ValueProjection, WriteProjection, WriteProjectionOpt,
    },
};

/// Marker for an optics path that is expected to exist.
pub struct Required;

/// Marker for an optics path that may be absent.
pub struct Optional;

/// Experimental carrier-generic optics wrapper.
pub struct Optic<A, Path = Required> {
    pub(crate) access: A,
    pub(crate) _marker: PhantomData<fn() -> Path>,
}

impl<A: Clone, Path> Clone for Optic<A, Path> {
    fn clone(&self) -> Self {
        Self {
            access: self.access.clone(),
            _marker: PhantomData,
        }
    }
}

impl<T: 'static> Optic<RwRoot<T>> {
    /// Create a new root optic backed by a `CopyValue`.
    #[must_use]
    pub fn new(value: T) -> Self {
        Self {
            access: RwRoot {
                cell: CopyValue::new(value),
            },
            _marker: PhantomData,
        }
    }
}

impl<T: 'static> From<T> for Optic<RwRoot<T>> {
    fn from(value: T) -> Self {
        Self::new(value)
    }
}

impl<A> Optic<A> {
    /// Wrap an arbitrary access carrier in the optics facade.
    #[must_use]
    pub fn from_access(access: A) -> Self {
        Self {
            access,
            _marker: PhantomData,
        }
    }
}

impl<A, Path> Optic<A, Path> {
    /// Borrow the underlying access carrier.
    pub fn access(&self) -> &A {
        &self.access
    }

    /// Consume the wrapper and return the underlying access carrier.
    pub fn into_access(self) -> A {
        self.access
    }

    /// Extract the current owned value produced by this optics path.
    pub fn value<Value>(&self) -> Value
    where
        A: ValueProjection<Value>,
    {
        self.access.value_projection()
    }

    /// Extract the future produced by this optics path.
    pub fn future<Fut>(&self) -> Fut
    where
        A: FutureProjection<Fut>,
        Fut: Future,
    {
        self.access.future_projection()
    }

    /// Read through the carrier as a normal root value.
    pub fn read<T>(&self) -> ReadableRef<'_, CopyValue<T>>
    where
        A: ReadProjection<T>,
        T: 'static,
    {
        self.access.read_projection()
    }

    /// Write through the carrier as a normal root value.
    pub fn write<T>(&self) -> WritableRef<'_, CopyValue<T>>
    where
        A: WriteProjection<T>,
        T: 'static,
    {
        WriteLock::downcast_lifetime(self.access.write_projection())
    }

    /// Read an optional child projection.
    pub fn read_opt<T>(&self) -> Option<ReadableRef<'_, CopyValue<T>>>
    where
        A: ReadProjectionOpt<T>,
        T: 'static,
    {
        self.access.read_projection_opt()
    }

    /// Write an optional child projection.
    pub fn write_opt<T>(&self) -> Option<WritableRef<'_, CopyValue<T>>>
    where
        A: WriteProjectionOpt<T>,
        T: 'static,
    {
        self.access
            .write_projection_opt()
            .map(WriteLock::downcast_lifetime)
    }

    /// Project a child field through paired read/write functions.
    #[must_use]
    pub fn map_ref_mut<T, U>(
        self,
        read: fn(&T) -> &U,
        write: fn(&mut T) -> &mut U,
    ) -> Optic<Combinator<A, LensOp<T, U>>, Path>
    where
        T: 'static,
        U: 'static,
    {
        Optic {
            access: Combinator {
                parent: self.access,
                op: LensOp { read, write },
            },
            _marker: PhantomData,
        }
    }

    /// Flatten `Option<Option<T>>` into `Option<T>` at the carrier boundary.
    #[must_use]
    pub fn flatten_some(self) -> Optic<FlattenSome<A>, Path> {
        Optic {
            access: Combinator {
                parent: self.access,
                op: FlattenSomeOp,
            },
            _marker: PhantomData,
        }
    }
}

impl<A> Optic<A, Required> {
    /// Lift `Option<T>` from inside the carrier to an optional child path.
    #[must_use]
    pub fn map_some<T>(self) -> Optic<Combinator<A, UnwrapSomeOp<T>>, Optional>
    where
        T: 'static,
    {
        Optic {
            access: Combinator {
                parent: self.access,
                op: UnwrapSomeOp(PhantomData),
            },
            _marker: PhantomData,
        }
    }

    /// Lift `Result<T, E>::Ok(T)` into an optional child path.
    #[must_use]
    pub fn map_ok<T, E>(self) -> Optic<Combinator<A, UnwrapOkOp<T, E>>, Optional>
    where
        T: 'static,
        E: 'static,
    {
        Optic {
            access: Combinator {
                parent: self.access,
                op: UnwrapOkOp(PhantomData),
            },
            _marker: PhantomData,
        }
    }

    /// Lift `Result<T, E>::Err(E)` into an optional child path.
    #[must_use]
    pub fn map_err<T, E>(self) -> Optic<Combinator<A, UnwrapErrOp<T, E>>, Optional>
    where
        T: 'static,
        E: 'static,
    {
        Optic {
            access: Combinator {
                parent: self.access,
                op: UnwrapErrOp(PhantomData),
            },
            _marker: PhantomData,
        }
    }

    /// Treat a `Vec<T>` child as an iterable collection of child optics.
    #[must_use]
    pub fn each<T>(self) -> Optic<EachVec<A, T>>
    where
        A: ReadProjection<Vec<T>>,
        T: 'static,
    {
        Optic {
            access: EachVec {
                parent: self.access,
                _marker: PhantomData,
            },
            _marker: PhantomData,
        }
    }

    /// Treat a `HashMap<K, V, S>` child as a keyed collection of child optics.
    #[must_use]
    pub fn each_hash_map<K, V, S>(self) -> Optic<EachHashMap<A, K, V, S>>
    where
        A: ReadProjection<HashMap<K, V, S>>,
        K: Eq + std::hash::Hash + 'static,
        V: 'static,
        S: BuildHasher + 'static,
    {
        Optic {
            access: EachHashMap {
                parent: self.access,
                _marker: PhantomData,
            },
            _marker: PhantomData,
        }
    }

    /// Treat a `BTreeMap<K, V>` child as a keyed collection of child optics.
    #[must_use]
    pub fn each_btree_map<K, V>(self) -> Optic<EachBTreeMap<A, K, V>>
    where
        A: ReadProjection<BTreeMap<K, V>>,
        K: Ord + 'static,
        V: 'static,
    {
        Optic {
            access: EachBTreeMap {
                parent: self.access,
                _marker: PhantomData,
            },
            _marker: PhantomData,
        }
    }
}

impl<A> Optic<A, Optional> {
    /// Lift `Option<T>` from inside an already-optional child path.
    #[must_use]
    pub fn map_some<T>(self) -> Optic<Combinator<A, UnwrapSomeOptionalOp<T>>, Optional>
    where
        T: 'static,
    {
        Optic {
            access: Combinator {
                parent: self.access,
                op: UnwrapSomeOptionalOp(PhantomData),
            },
            _marker: PhantomData,
        }
    }

    /// Lift `Result<T, E>::Ok(T)` from inside an already-optional child path.
    #[must_use]
    pub fn map_ok<T, E>(self) -> Optic<Combinator<A, UnwrapOkOptionalOp<T, E>>, Optional>
    where
        T: 'static,
        E: 'static,
    {
        Optic {
            access: Combinator {
                parent: self.access,
                op: UnwrapOkOptionalOp(PhantomData),
            },
            _marker: PhantomData,
        }
    }

    /// Lift `Result<T, E>::Err(E)` from inside an already-optional child path.
    #[must_use]
    pub fn map_err<T, E>(self) -> Optic<Combinator<A, UnwrapErrOptionalOp<T, E>>, Optional>
    where
        T: 'static,
        E: 'static,
    {
        Optic {
            access: Combinator {
                parent: self.access,
                op: UnwrapErrOptionalOp(PhantomData),
            },
            _marker: PhantomData,
        }
    }
}

/// Root read/write carrier used by [`Optic::new`].
pub struct RwRoot<T> {
    pub(crate) cell: CopyValue<T>,
}

impl<T> Clone for RwRoot<T> {
    fn clone(&self) -> Self {
        Self {
            cell: self.cell.clone(),
        }
    }
}

impl<T> ValueProjection<T> for RwRoot<T>
where
    T: Clone + 'static,
{
    fn value_projection(&self) -> T {
        self.cell.read_unchecked().clone()
    }
}

impl<T> ReadProjection<T> for RwRoot<T>
where
    T: 'static,
{
    fn read_projection(&self) -> GenerationalRef<Ref<'static, T>> {
        self.cell.read_unchecked()
    }
}

impl<T> WriteProjection<T> for RwRoot<T>
where
    T: 'static,
{
    fn write_projection(&self) -> WriteLock<'static, T, UnsyncStorage> {
        self.cell.write_unchecked()
    }
}
