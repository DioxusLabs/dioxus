//! Direct-on-carrier optic methods.
//!
//! [`OpticExt`] / [`OpticMutExt`] are blanket-implemented for every
//! [`Access`] / [`AccessMut`] carrier so that callers can write
//! `signal.map_ref(...)`, `signal.subscribed()`, `store.get(key)` and so on
//! without first wrapping the carrier in `Optic::from_access(...)`. The
//! methods just forward to the equivalent inherent methods on
//! [`Optic`](crate::Optic).
//!
//! Bridges in `dioxus-signals` register `Signal`, `Memo`, `Store`,
//! `CopyValue`, `ReadSignal`, `WriteSignal`, `GlobalSignal`, etc. as
//! [`Access`] / [`AccessMut`], so they all pick this trait up automatically.

use std::marker::PhantomData;

use generational_box::{AnyStorage, WriteLock};

use crate::collection::{FlattenSome, GetProjection};
use crate::combinator::{
    Access, AccessMut, Combinator, ErrPrism, InlinePrism, LensOp, OkPrism, Prism, PrismOp, RefOp,
    SomePrism,
};
use crate::path::Pathed;
use crate::signal::{Optic, Optional, Required};
use crate::subscribed::{Subscribed, SubscriptionTree};

/// Direct-on-carrier read optic methods.
///
/// Implemented for every [`Access`] carrier. Each method forwards to the
/// inherent method on [`Optic`] so call sites can drop the
/// `Optic::from_access(...)` wrapper.
pub trait OpticExt: Access + Sized {
    /// Wrap this carrier in an [`Optic`].
    #[inline]
    fn as_optic(self) -> Optic<Self, Required> {
        Optic::from_access(self)
    }

    /// Read through the carrier. Panics if the projected path is currently
    /// absent.
    #[inline]
    fn read_required(&self) -> <Self::Storage as AnyStorage>::Ref<'static, Self::Target> {
        self.try_read()
            .expect("optics: required path produced no value")
    }

    /// Read through the carrier as `Option`.
    #[inline]
    fn read_optional(&self) -> Option<<Self::Storage as AnyStorage>::Ref<'static, Self::Target>> {
        self.try_read()
    }

    /// Project a child field through a read-only function.
    #[inline]
    #[must_use]
    fn map_ref<T, U>(self, read: fn(&T) -> &U) -> Optic<Combinator<Self, RefOp<T, U>>, Required>
    where
        T: 'static,
        U: 'static,
    {
        Optic::from_access(self).map_ref(read)
    }

    /// Project a child field through paired read/write functions.
    #[inline]
    #[must_use]
    fn map_ref_mut<T, U>(
        self,
        read: fn(&T) -> &U,
        write: fn(&mut T) -> &mut U,
    ) -> Optic<Combinator<Self, LensOp<T, U>>, Required>
    where
        T: 'static,
        U: 'static,
    {
        Optic::from_access(self).map_ref_mut(read, write)
    }

    /// Lift `Option<T>` from inside the carrier to an optional child path.
    #[inline]
    #[must_use]
    fn map_some<T>(self) -> Optic<Combinator<Self, PrismOp<SomePrism<T>>>, Optional>
    where
        T: 'static,
        Self: Access<Target = Option<T>>,
    {
        Optic::from_access(self).map_some::<T>()
    }

    /// Lift `Result<T, E>::Ok(T)` into an optional child path.
    #[inline]
    #[must_use]
    fn map_ok<T, E>(self) -> Optic<Combinator<Self, PrismOp<OkPrism<T, E>>>, Optional>
    where
        T: 'static,
        E: 'static,
        Self: Access<Target = Result<T, E>>,
    {
        Optic::from_access(self).map_ok::<T, E>()
    }

    /// Lift `Result<T, E>::Err(E)` into an optional child path.
    #[inline]
    #[must_use]
    fn map_err<T, E>(self) -> Optic<Combinator<Self, PrismOp<ErrPrism<T, E>>>, Optional>
    where
        T: 'static,
        E: 'static,
        Self: Access<Target = Result<T, E>>,
    {
        Optic::from_access(self).map_err::<T, E>()
    }

    /// Project into a variant of any sum type through a user-defined [`Prism`].
    #[inline]
    #[must_use]
    fn map_variant<P>(self) -> Optic<Combinator<Self, PrismOp<P>>, Optional>
    where
        P: Prism + Default,
    {
        Optic::from_access(self).map_variant::<P>()
    }

    /// Project into a variant using inline `fn` pointers.
    #[inline]
    #[must_use]
    fn map_variant_with<S, V>(
        self,
        try_ref: fn(&S) -> Option<&V>,
        try_mut: fn(&mut S) -> Option<&mut V>,
        try_into: fn(S) -> Option<V>,
    ) -> Optic<Combinator<Self, PrismOp<InlinePrism<S, V>>>, Optional>
    where
        S: 'static,
        V: 'static,
    {
        Optic::from_access(self).map_variant_with(try_ref, try_mut, try_into)
    }

    /// Flatten `Option<Option<T>>` into `Option<T>` at the carrier boundary.
    #[inline]
    #[must_use]
    fn flatten_some(self) -> Optic<FlattenSome<Self>, Required> {
        Optic::from_access(self).flatten_some()
    }

    /// Project a child from a collection or keyed container lookup.
    #[inline]
    #[must_use]
    fn get<Key>(&self, key: Key) -> Optic<<Self as GetProjection<Key>>::Child, Optional>
    where
        Self: GetProjection<Key>,
    {
        Optic {
            access: self.get_projection(key),
            _marker: PhantomData,
        }
    }

    /// Wrap this carrier in a [`Subscribed`] carrier with a fresh subscription
    /// tree, gaining path-granular reactivity.
    #[inline]
    #[must_use]
    fn subscribed(self) -> Optic<Subscribed<Self>, Required>
    where
        Self: Pathed,
    {
        Optic::from_access(self).subscribed()
    }

    /// Wrap this carrier in a [`Subscribed`] carrier sharing an existing
    /// subscription tree.
    #[inline]
    #[must_use]
    fn subscribed_with(self, tree: SubscriptionTree) -> Optic<Subscribed<Self>, Required>
    where
        Self: Pathed,
    {
        Optic::from_access(self).subscribed_with(tree)
    }

    /// Peek the current `Option<T>` value and, if it's `Some`, return a
    /// [`Required`]-tagged optic projecting the inner `T`.
    #[inline]
    #[must_use]
    fn try_some<T>(self) -> Option<Optic<Combinator<Self, PrismOp<SomePrism<T>>>, Required>>
    where
        Self: Access<Target = Option<T>>,
        T: 'static,
    {
        Optic::from_access(self).try_some::<T>()
    }

    /// Peek the current `Result<T, E>` value and return a
    /// [`Required`]-tagged optic for the matching variant.
    #[inline]
    #[must_use]
    fn try_ok<T, E>(
        self,
    ) -> Result<
        Optic<Combinator<Self, PrismOp<OkPrism<T, E>>>, Required>,
        Optic<Combinator<Self, PrismOp<ErrPrism<T, E>>>, Required>,
    >
    where
        Self: Access<Target = Result<T, E>> + Clone,
        T: 'static,
        E: 'static,
    {
        Optic::from_access(self).try_ok::<T, E>()
    }
}

impl<A: Access> OpticExt for A {}

/// Direct-on-carrier mutable optic helpers.
pub trait OpticMutExt: AccessMut + Sized {
    /// Mutably borrow the underlying value, panicking if the path is absent.
    #[inline]
    fn write_required(
        &self,
    ) -> WriteLock<'static, Self::Target, Self::Storage, Self::WriteMetadata> {
        self.try_write()
            .expect("optics: required path produced no value")
    }

    /// Mutably borrow as `Option`.
    #[inline]
    fn write_optional(
        &self,
    ) -> Option<WriteLock<'static, Self::Target, Self::Storage, Self::WriteMetadata>> {
        self.try_write()
    }
}

impl<A: AccessMut> OpticMutExt for A {}
