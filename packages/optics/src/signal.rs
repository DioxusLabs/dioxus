use std::{future::Future, marker::PhantomData};

use dioxus_core::current_owner;
use generational_box::{AnyStorage, GenerationalBox, UnsyncStorage, WriteLock};

use crate::{
    collection::{Cloned, FlattenSome, FlattenSomeOp, GetProjection},
    combinator::{
        Access, AccessMut, Combinator, ErrPrism, FutureAccess, InlinePrism, LensOp, OkPrism,
        OptPrismOp, Prism, PrismOp, RefOp, SomePrism, ValueAccess,
    },
    iter::{IterShape, OpticIter},
    path::Pathed,
    subscribed::{HasSubscriptionTree, Subscribed, SubscriptionTree},
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

impl<A: Copy, Path> Copy for Optic<A, Path> {}

impl<A: Clone, Path> Clone for Optic<A, Path> {
    fn clone(&self) -> Self {
        Self {
            access: self.access.clone(),
            _marker: PhantomData,
        }
    }
}

impl<A, Path> std::fmt::Debug for Optic<A, Path>
where
    A: Access,
    A::Target: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.access.try_read() {
            Some(value) => f.debug_tuple("Optic").field(&*value).finish(),
            None => f.debug_struct("Optic").field("path", &"absent").finish(),
        }
    }
}

impl<A, Path> std::fmt::Display for Optic<A, Path>
where
    A: Access,
    A::Target: std::fmt::Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.access.try_read() {
            Some(value) => std::fmt::Display::fmt(&*value, f),
            None => Ok(()),
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
    pub fn write_opt(&self) -> Option<WriteLock<'static, A::Target, A::Storage, A::WriteMetadata>>
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
    pub fn map_ref<T, U>(self, read: fn(&T) -> &U) -> Optic<Combinator<A, RefOp<T, U>>, Path>
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

    /// Step into the projected collection, returning a reusable
    /// `Optic<Each*<...>>` carrier. Dispatches based on the target shape:
    /// `Vec<T>` → [`EachVec`](crate::EachVec), `HashMap<K, V, S>` →
    /// [`EachHashMap`](crate::EachHashMap), `BTreeMap<K, V>` →
    /// [`EachBTreeMap`](crate::EachBTreeMap).
    ///
    /// Replaces the older `.each()` / `.each_hash_map()` /
    /// `.each_btree_map()` entry methods.
    #[must_use]
    pub fn iter(&self) -> Optic<<A::Target as IterShape>::Each<A>, Required>
    where
        A: Access + Clone,
        A::Target: IterShape,
    {
        OpticIter::iter(self.access.clone())
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

    /// Build a derived optic whose value is the projected target cloned
    /// out on each read. Use `.value()` to materialize the clone, or
    /// compose with further combinators to stay inside the optic chain.
    #[must_use]
    pub fn cloned(&self) -> Optic<Cloned<A>>
    where
        A: Clone,
    {
        Optic {
            access: Cloned {
                parent: self.access.clone(),
            },
            _marker: PhantomData,
        }
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
    pub fn map_variant_with_prism<P>(self, prism: P) -> Optic<Combinator<A, PrismOp<P>>, Optional>
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

    /// Peek the current `Option<T>` value and, if it's `Some`, return a
    /// [`Required`]-tagged optic projecting the inner `T`. If the value
    /// is `None`, return `None`.
    ///
    /// Equivalent to `self.map_some::<T>().to_option()` and exposed as a
    /// dedicated method so the intent reads cleanly at call sites.
    #[must_use]
    pub fn try_some<T>(self) -> Option<Optic<Combinator<A, PrismOp<SomePrism<T>>>, Required>>
    where
        A: Access<Target = Option<T>>,
        T: 'static,
    {
        self.map_some::<T>().to_option()
    }

    /// Peek the current `Result<T, E>` value and return a
    /// [`Required`]-tagged optic for the matching variant — `Ok(optic)`
    /// for the `Ok` payload, `Err(optic)` for the `Err` payload.
    #[must_use]
    pub fn try_ok<T, E>(
        self,
    ) -> Result<
        Optic<Combinator<A, PrismOp<OkPrism<T, E>>>, Required>,
        Optic<Combinator<A, PrismOp<ErrPrism<T, E>>>, Required>,
    >
    where
        A: Access<Target = Result<T, E>> + Clone,
        T: 'static,
        E: 'static,
    {
        let is_ok = self.access.try_read().map(|r| r.is_ok()).unwrap_or(false);
        if is_ok {
            Ok(self
                .map_ok::<T, E>()
                .to_option()
                .expect("Result was Ok at peek time but absent on read"))
        } else {
            Err(self
                .map_err::<T, E>()
                .to_option()
                .expect("Result was Err at peek time but absent on read"))
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

    /// Assert that the optional path resolves now and convert the optic to a
    /// [`Required`] tag. Panics if the path is currently absent.
    ///
    /// Equivalent to `self.to_option().expect(...)`. Useful when a caller has
    /// already confirmed the projection's presence (for example, a HashMap
    /// `.get(key)` that is known to hit) and wants to drop the `Option`
    /// shape without peeling it manually.
    #[track_caller]
    pub fn unwrap(self) -> Optic<A, Required>
    where
        A: Access,
    {
        self.to_option()
            .expect("optics: called `unwrap` on an absent optional path")
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

// ============================================================================
// Optic<A, Path> implements Access / AccessMut / ValueAccess / FutureAccess /
// Pathed by delegating to its inner accessor `A`. This lets `Optic<...>` slot
// into the same blanket impls (e.g. `OpticExt`, the `#[derive(Store)]`
// extension trait) as a raw `Signal`, `Store`, `Resource`, etc., so a chain
// like `store.iter().get(&id).to_option().expect(...).checked()` keeps
// reaching the macro-generated field accessors.
// ============================================================================

impl<A, Path> Access for Optic<A, Path>
where
    A: Access,
{
    type Target = A::Target;
    type Storage = A::Storage;

    #[inline]
    fn try_read(&self) -> Option<<Self::Storage as AnyStorage>::Ref<'static, Self::Target>> {
        self.access.try_read()
    }

    #[inline]
    fn try_peek(&self) -> Option<<Self::Storage as AnyStorage>::Ref<'static, Self::Target>> {
        self.access.try_peek()
    }
}

impl<A, Path> AccessMut for Optic<A, Path>
where
    A: AccessMut,
{
    type WriteMetadata = A::WriteMetadata;

    #[inline]
    fn try_write(&self) -> Option<WriteLock<'static, A::Target, A::Storage, A::WriteMetadata>> {
        self.access.try_write()
    }

    #[inline]
    fn try_write_silent(
        &self,
    ) -> Option<WriteLock<'static, A::Target, A::Storage, A::WriteMetadata>> {
        self.access.try_write_silent()
    }
}

impl<A, Path, T> ValueAccess<T> for Optic<A, Path>
where
    A: ValueAccess<T>,
{
    #[inline]
    fn value(&self) -> T {
        self.access.value()
    }
}

impl<A, Path, Fut> FutureAccess<Fut> for Optic<A, Path>
where
    A: FutureAccess<Fut>,
    Fut: Future,
{
    #[inline]
    fn future(&self) -> Fut {
        self.access.future()
    }
}

impl<A, Path> Pathed for Optic<A, Path>
where
    A: Pathed,
{
    #[inline]
    fn visit_path(&self, sink: &mut crate::path::PathBuffer) {
        self.access.visit_path(sink);
    }
}

impl<A, Path> HasSubscriptionTree for Optic<A, Path>
where
    A: HasSubscriptionTree,
{
    #[inline]
    fn subscription_tree(&self) -> SubscriptionTree {
        self.access.subscription_tree()
    }
}

// `GenerationalBox` is the storage used by `Optic::new` roots. It doesn't
// carry a real subscription tree on its own — give it a fresh one so methods
// bounded on `HasSubscriptionTree` (e.g. collection `.len()`, `.is_empty()`)
// still compile on test-style `Optic::new(...)` chains.
impl<T, S: 'static> HasSubscriptionTree for generational_box::GenerationalBox<T, S> {
    fn subscription_tree(&self) -> SubscriptionTree {
        SubscriptionTree::new()
    }
}
