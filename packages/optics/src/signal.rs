use std::{
    collections::{BTreeMap, HashMap},
    future::Future,
    hash::BuildHasher,
    marker::PhantomData,
};

use dioxus_core::current_owner;
use generational_box::{AnyStorage, GenerationalBox, UnsyncStorage, WriteLock};

use crate::{
    collection::{EachBTreeMap, EachHashMap, EachVec, FlattenSome, FlattenSomeOp, GetProjection},
    combinator::{
        Access, AccessMut, Combinator, ErrPrism, FutureAccess, InlinePrism, LensOp, OkPrism,
        OptPrismOp, Prism, PrismOp, RefOp, SomePrism, ValueAccess,
    },
    path::Pathed,
    subscribed::{Subscribed, SubscriptionTree},
};

/// Marker for an optics path that is expected to exist (user-facing `read()` /
/// `write()` unwrap the underlying `Option`).
pub struct Required;

/// Marker for an optics path that may be absent (user-facing `read_opt()` /
/// `write_opt()` return `Option` directly).
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

impl<T: 'static> Optic<GenerationalBox<T, UnsyncStorage>> {
    /// Create a new root optic backed by a [`GenerationalBox`] allocated in
    /// the current Dioxus scope's owner.
    #[must_use]
    #[track_caller]
    pub fn new(value: T) -> Self {
        let owner = current_owner::<UnsyncStorage>();
        Self {
            access: owner.insert_rc(value),
            _marker: PhantomData,
        }
    }
}

impl<T: 'static> From<T> for Optic<GenerationalBox<T, UnsyncStorage>> {
    fn from(value: T) -> Self {
        Self::new(value)
    }
}

impl<A> Optic<A> {
    /// Wrap an arbitrary access carrier in the optics facade.
    ///
    /// Accepts any [`dioxus_signals::Readable`] — `CopyValue`, `Signal`,
    /// `SyncSignal`, `Memo`, `ReadSignal`, `WriteSignal`, `Store`,
    /// `GlobalSignal`, `GlobalMemo`, etc.
    #[must_use]
    pub fn from_access(access: A) -> Self {
        Self {
            access,
            _marker: PhantomData,
        }
    }
}

// ============================================================================
// Methods available regardless of Path
// ============================================================================

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
        A: ValueAccess<Value>,
    {
        self.access.value()
    }

    /// Extract the future produced by this optics path.
    pub fn future<Fut>(&self) -> Fut
    where
        A: FutureAccess<Fut>,
        Fut: Future,
    {
        self.access.future()
    }

    /// Read an optional child projection.
    pub fn read_opt(&self) -> Option<<A::Storage as AnyStorage>::Ref<'static, A::Target>>
    where
        A: Access,
    {
        self.access.try_read()
    }

    /// Write an optional child projection.
    pub fn write_opt(
        &self,
    ) -> Option<WriteLock<'static, A::Target, A::Storage, A::WriteMetadata>>
    where
        A: AccessMut,
    {
        self.access.try_write()
    }

    /// Project a child field through a read-only function.
    ///
    /// Use this for carriers that only expose [`Access`] — for example a
    /// [`dioxus_signals::Memo`].
    #[must_use]
    pub fn map_ref<T, U>(
        self,
        read: fn(&T) -> &U,
    ) -> Optic<Combinator<A, RefOp<T, U>>, Path>
    where
        T: 'static,
        U: 'static,
    {
        Optic {
            access: Combinator {
                parent: self.access,
                op: RefOp { read },
            },
            _marker: PhantomData,
        }
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

    /// Project a child from a collection or keyed container lookup.
    #[must_use]
    pub fn get<Key>(&self, key: Key) -> Optic<<A as GetProjection<Key>>::Child, Optional>
    where
        A: GetProjection<Key>,
    {
        Optic {
            access: self.access.get_projection(key),
            _marker: PhantomData,
        }
    }

    /// Wrap this optic in a [`Subscribed`] carrier that performs
    /// path-granular subscription tracking.
    ///
    /// Reads through the returned optic subscribe the current
    /// [`ReactiveContext`](dioxus_core::ReactiveContext) at this accessor's
    /// path; writes wake subscribers on overlapping paths. The tree is
    /// fresh — use [`Optic::subscribed_with`] to share a tree between
    /// multiple chains (for cross-chain reactivity).
    ///
    /// Plain optics that never call `.subscribed()` pay zero cost for this
    /// machinery.
    #[must_use]
    pub fn subscribed(self) -> Optic<Subscribed<A>, Path>
    where
        A: Pathed,
    {
        Optic {
            access: Subscribed::new(self.access),
            _marker: PhantomData,
        }
    }

    /// Wrap this optic in a [`Subscribed`] carrier sharing an existing
    /// subscription tree.
    #[must_use]
    pub fn subscribed_with(self, tree: SubscriptionTree) -> Optic<Subscribed<A>, Path>
    where
        A: Pathed,
    {
        Optic {
            access: Subscribed::with_tree(self.access, tree),
            _marker: PhantomData,
        }
    }
}

// ============================================================================
// Required-only conveniences: read / write unwrap, each*, map_some / map_ok
// ============================================================================

impl<A> Optic<A, Required> {
    /// Read through the carrier. Panics if the projected path is currently
    /// absent — if the path is optional, use `read_opt` instead.
    pub fn read(&self) -> <A::Storage as AnyStorage>::Ref<'static, A::Target>
    where
        A: Access,
    {
        self.access
            .try_read()
            .expect("optics: required path produced no value")
    }

    /// Write through the carrier. Panics if the path is currently absent.
    pub fn write(&self) -> WriteLock<'static, A::Target, A::Storage, A::WriteMetadata>
    where
        A: AccessMut,
    {
        self.access
            .try_write()
            .expect("optics: required path produced no value")
    }

    /// Lift `Option<T>` from inside the carrier to an optional child path.
    #[must_use]
    pub fn map_some<T>(self) -> Optic<Combinator<A, PrismOp<SomePrism<T>>>, Optional>
    where
        T: 'static,
    {
        self.map_variant::<SomePrism<T>>()
    }

    /// Lift `Result<T, E>::Ok(T)` into an optional child path.
    #[must_use]
    pub fn map_ok<T, E>(self) -> Optic<Combinator<A, PrismOp<OkPrism<T, E>>>, Optional>
    where
        T: 'static,
        E: 'static,
    {
        self.map_variant::<OkPrism<T, E>>()
    }

    /// Lift `Result<T, E>::Err(E)` into an optional child path.
    #[must_use]
    pub fn map_err<T, E>(self) -> Optic<Combinator<A, PrismOp<ErrPrism<T, E>>>, Optional>
    where
        T: 'static,
        E: 'static,
    {
        self.map_variant::<ErrPrism<T, E>>()
    }

    /// Project into a variant of any sum type through a user-defined [`Prism`].
    #[must_use]
    pub fn map_variant<P>(self) -> Optic<Combinator<A, PrismOp<P>>, Optional>
    where
        P: Prism + Default,
    {
        self.map_variant_with_prism(P::default())
    }

    /// Project into a variant through a specific prism instance.
    #[must_use]
    pub fn map_variant_with_prism<P>(
        self,
        prism: P,
    ) -> Optic<Combinator<A, PrismOp<P>>, Optional>
    where
        P: Prism,
    {
        Optic {
            access: Combinator {
                parent: self.access,
                op: PrismOp { prism },
            },
            _marker: PhantomData,
        }
    }

    /// Project into a variant using inline `fn` pointers.
    #[must_use]
    pub fn map_variant_with<S, V>(
        self,
        try_ref: fn(&S) -> Option<&V>,
        try_mut: fn(&mut S) -> Option<&mut V>,
        try_into: fn(S) -> Option<V>,
    ) -> Optic<Combinator<A, PrismOp<InlinePrism<S, V>>>, Optional>
    where
        S: 'static,
        V: 'static,
    {
        self.map_variant_with_prism(InlinePrism::new(try_ref, try_mut, try_into))
    }

    /// Treat a `Vec<T>` child as an iterable collection of child optics.
    #[must_use]
    pub fn each<T>(self) -> Optic<EachVec<A, T>>
    where
        A: Access<Target = Vec<T>>,
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
        A: Access<Target = HashMap<K, V, S>>,
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
        A: Access<Target = BTreeMap<K, V>>,
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

// ============================================================================
// Optional-only conveniences: to_option, optional-chained variants
// ============================================================================

impl<A> Optic<A, Optional> {
    /// Materialize the current optional path as `Option<Optic<_, Required>>`.
    ///
    /// If the path currently resolves, returns a `Required`-tagged optic whose
    /// subsequent `read`/`write` calls will unwrap. If the path shape later
    /// changes to absent, those calls panic.
    pub fn to_option(self) -> Option<Optic<A, Required>>
    where
        A: Access,
    {
        self.access.try_read().is_some().then(|| Optic {
            access: self.access,
            _marker: PhantomData,
        })
    }

    /// Lift `Option<T>` from inside an already-optional child path.
    #[must_use]
    pub fn map_some<T>(self) -> Optic<Combinator<A, OptPrismOp<SomePrism<T>>>, Optional>
    where
        T: 'static,
    {
        self.map_variant::<SomePrism<T>>()
    }

    /// Lift `Result<T, E>::Ok(T)` from inside an already-optional child path.
    #[must_use]
    pub fn map_ok<T, E>(self) -> Optic<Combinator<A, OptPrismOp<OkPrism<T, E>>>, Optional>
    where
        T: 'static,
        E: 'static,
    {
        self.map_variant::<OkPrism<T, E>>()
    }

    /// Lift `Result<T, E>::Err(E)` from inside an already-optional child path.
    #[must_use]
    pub fn map_err<T, E>(self) -> Optic<Combinator<A, OptPrismOp<ErrPrism<T, E>>>, Optional>
    where
        T: 'static,
        E: 'static,
    {
        self.map_variant::<ErrPrism<T, E>>()
    }

    /// Project into a variant through a user-defined [`Prism`].
    #[must_use]
    pub fn map_variant<P>(self) -> Optic<Combinator<A, OptPrismOp<P>>, Optional>
    where
        P: Prism + Default,
    {
        self.map_variant_with_prism(P::default())
    }

    /// Project into a variant through a specific prism instance.
    #[must_use]
    pub fn map_variant_with_prism<P>(
        self,
        prism: P,
    ) -> Optic<Combinator<A, OptPrismOp<P>>, Optional>
    where
        P: Prism,
    {
        Optic {
            access: Combinator {
                parent: self.access,
                op: OptPrismOp { prism },
            },
            _marker: PhantomData,
        }
    }

    /// Project into a variant using inline `fn` pointers.
    #[must_use]
    pub fn map_variant_with<S, V>(
        self,
        try_ref: fn(&S) -> Option<&V>,
        try_mut: fn(&mut S) -> Option<&mut V>,
        try_into: fn(S) -> Option<V>,
    ) -> Optic<Combinator<A, OptPrismOp<InlinePrism<S, V>>>, Optional>
    where
        S: 'static,
        V: 'static,
    {
        self.map_variant_with_prism(InlinePrism::new(try_ref, try_mut, try_into))
    }
}
