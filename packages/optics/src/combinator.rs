use std::{future::Future, marker::PhantomData};

use dioxus_signals::WriteLock;
use generational_box::AnyStorage;

use crate::resource::{AsFuture, AwaitTransform};

// ============================================================================
// Core traits
// ============================================================================

/// Read access to a projected child path.
///
/// `try_read` returns `None` when the path does not currently resolve — for
/// example, a prism into the wrong enum variant, an absent optional field, or
/// a missing collection key. Carriers whose path always resolves (a signal
/// root, a field lens on a required parent) still return `Option` — the
/// `Option` is always `Some` in those cases, and user-facing `read()` on a
/// [`Required`](crate::Required) `Optic` unwraps it.
pub trait Access {
    /// Value type produced by this projection.
    type Target: 'static;
    /// Backing storage the read reference uses.
    type Storage: AnyStorage;

    /// Borrow the projected child if the path currently resolves.
    fn try_read(&self) -> Option<<Self::Storage as AnyStorage>::Ref<'static, Self::Target>>;
}

/// Write access to a projected child path. Same `Option` semantics as [`Access`].
pub trait AccessMut: Access {
    /// Additional data carried alongside the mutable reference (subscription
    /// drop guards, etc.).
    type WriteMetadata;

    /// Mutably borrow the projected child if the path currently resolves.
    fn try_write(
        &self,
    ) -> Option<WriteLock<'static, Self::Target, Self::Storage, Self::WriteMetadata>>;
}

/// Owned value extraction for a projected child.
///
/// This is a separate channel from [`Access`] because carriers may want to
/// clone, resolve from a cache, or flatten differently at the value level
/// than at the reference level.
pub trait ValueAccess<T> {
    /// Materialize the current owned value for this projection.
    fn value(&self) -> T;
}

/// Future extraction for a projected child.
pub trait FutureAccess<Fut>
where
    Fut: Future,
{
    /// Produce the future that resolves the current projection.
    fn future(&self) -> Fut;
}

/// Owned-value transform applied by an op.
///
/// Used by [`Combinator`]'s [`ValueAccess`] and [`FutureAccess`] impls to
/// route owned values through ops like field lenses and prisms.
pub trait Resolve<Op> {
    /// The owned input consumed before the op runs.
    type Input;

    /// Apply the op to the owned input.
    fn resolve(input: Self::Input, op: &Op) -> Self;
}

// ============================================================================
// Combinator
// ============================================================================

/// Applies an optics operation to a parent accessor.
pub struct Combinator<A, Op> {
    pub(crate) parent: A,
    pub(crate) op: Op,
}

impl<A: Clone, Op: Clone> Clone for Combinator<A, Op> {
    fn clone(&self) -> Self {
        Self {
            parent: self.parent.clone(),
            op: self.op.clone(),
        }
    }
}

impl<A, Op, Out> ValueAccess<Out> for Combinator<A, Op>
where
    Out: Resolve<Op>,
    A: ValueAccess<Out::Input>,
{
    fn value(&self) -> Out {
        Out::resolve(self.parent.value(), &self.op)
    }
}

impl<A, Op, Fut, Out> FutureAccess<AsFuture<AwaitTransform<Fut, Op, Out>>> for Combinator<A, Op>
where
    A: FutureAccess<AsFuture<Fut>>,
    Fut: Future<Output = Out::Input>,
    Op: Clone,
    Out: Resolve<Op>,
{
    fn future(&self) -> AsFuture<AwaitTransform<Fut, Op, Out>> {
        AsFuture(AwaitTransform::new(
            self.parent.future().0,
            self.op.clone(),
        ))
    }
}

// ============================================================================
// Field lenses: RefOp (read-only) and LensOp (read+write)
// ============================================================================

/// Read-only field projection from `S` to `U`.
#[derive(Clone)]
pub struct RefOp<S, U> {
    pub(crate) read: fn(&S) -> &U,
}

/// Read+write field projection from `S` to `U`.
#[derive(Clone)]
pub struct LensOp<S, U> {
    pub(crate) read: fn(&S) -> &U,
    pub(crate) write: fn(&mut S) -> &mut U,
}

impl<S, U> Resolve<RefOp<S, U>> for U
where
    U: Clone,
{
    type Input = S;
    fn resolve(input: S, op: &RefOp<S, U>) -> Self {
        (op.read)(&input).clone()
    }
}

impl<S, U> Resolve<RefOp<S, U>> for Option<U>
where
    U: Clone,
{
    type Input = Option<S>;
    fn resolve(input: Option<S>, op: &RefOp<S, U>) -> Self {
        input.map(|v| (op.read)(&v).clone())
    }
}

impl<S, U> Resolve<LensOp<S, U>> for U
where
    U: Clone,
{
    type Input = S;
    fn resolve(input: S, op: &LensOp<S, U>) -> Self {
        (op.read)(&input).clone()
    }
}

impl<S, U> Resolve<LensOp<S, U>> for Option<U>
where
    U: Clone,
{
    type Input = Option<S>;
    fn resolve(input: Option<S>, op: &LensOp<S, U>) -> Self {
        input.map(|v| (op.read)(&v).clone())
    }
}

impl<A, S: 'static, U: 'static> Access for Combinator<A, RefOp<S, U>>
where
    A: Access<Target = S>,
{
    type Target = U;
    type Storage = A::Storage;

    fn try_read(&self) -> Option<<A::Storage as AnyStorage>::Ref<'static, U>> {
        self.parent
            .try_read()
            .map(|r| A::Storage::map(r, self.op.read))
    }
}

impl<A, S: 'static, U: 'static> Access for Combinator<A, LensOp<S, U>>
where
    A: Access<Target = S>,
{
    type Target = U;
    type Storage = A::Storage;

    fn try_read(&self) -> Option<<A::Storage as AnyStorage>::Ref<'static, U>> {
        self.parent
            .try_read()
            .map(|r| A::Storage::map(r, self.op.read))
    }
}

impl<A, S: 'static, U: 'static> AccessMut for Combinator<A, LensOp<S, U>>
where
    A: AccessMut<Target = S>,
{
    type WriteMetadata = A::WriteMetadata;

    fn try_write(
        &self,
    ) -> Option<WriteLock<'static, U, A::Storage, A::WriteMetadata>> {
        self.parent
            .try_write()
            .map(|w| WriteLock::map(w, self.op.write))
    }
}

// ============================================================================
// Prism: partial projection into a sum-type variant
// ============================================================================

/// Partial projection into one variant of a sum type.
///
/// `Prism` is the primitive behind `map_some`, `map_ok`, `map_err`, and any
/// user-defined enum variant projection. Methods take `&self` so implementors
/// can carry runtime state (e.g. closures for
/// [`map_variant_with`](crate::Optic::map_variant_with)); stateless prisms
/// are typically zero-sized types that also implement [`Default`].
pub trait Prism {
    /// The whole sum-type being projected into.
    type Source: 'static;
    /// The variant payload produced when the path matches.
    type Variant: 'static;

    fn try_ref<'a>(&self, source: &'a Self::Source) -> Option<&'a Self::Variant>;
    fn try_mut<'a>(&self, source: &'a mut Self::Source) -> Option<&'a mut Self::Variant>;
    fn try_into_variant(&self, source: Self::Source) -> Option<Self::Variant>;
}

/// Op wrapping a [`Prism`] for use inside a [`Combinator`].
pub struct PrismOp<P> {
    pub(crate) prism: P,
}

impl<P: Clone> Clone for PrismOp<P> {
    fn clone(&self) -> Self {
        Self { prism: self.prism.clone() }
    }
}

impl<A, P> Access for Combinator<A, PrismOp<P>>
where
    A: Access<Target = P::Source>,
    P: Prism,
{
    type Target = P::Variant;
    type Storage = A::Storage;

    fn try_read(&self) -> Option<<A::Storage as AnyStorage>::Ref<'static, P::Variant>> {
        let prism = &self.op.prism;
        self.parent
            .try_read()
            .and_then(|r| A::Storage::try_map(r, |s| prism.try_ref(s)))
    }
}

impl<A, P> AccessMut for Combinator<A, PrismOp<P>>
where
    A: AccessMut<Target = P::Source>,
    P: Prism,
{
    type WriteMetadata = A::WriteMetadata;

    fn try_write(
        &self,
    ) -> Option<WriteLock<'static, P::Variant, A::Storage, A::WriteMetadata>> {
        let prism = &self.op.prism;
        self.parent
            .try_write()
            .and_then(|w| WriteLock::filter_map(w, |s| prism.try_mut(s)))
    }
}

impl<P> Resolve<PrismOp<P>> for Option<P::Variant>
where
    P: Prism,
{
    type Input = P::Source;
    fn resolve(input: P::Source, op: &PrismOp<P>) -> Self {
        op.prism.try_into_variant(input)
    }
}

/// Variant projection applied to an already-optional parent.
///
/// Shares the same [`Access`] / [`AccessMut`] impls with [`PrismOp`] (both
/// just thread `Option<Ref>` through the prism); distinct only so that the
/// owned-value plumbing ([`Resolve`]) can accept an `Option<Source>` input
/// rather than a bare `Source`.
pub struct OptPrismOp<P> {
    pub(crate) prism: P,
}

impl<P: Clone> Clone for OptPrismOp<P> {
    fn clone(&self) -> Self {
        Self { prism: self.prism.clone() }
    }
}

impl<A, P> Access for Combinator<A, OptPrismOp<P>>
where
    A: Access<Target = P::Source>,
    P: Prism,
{
    type Target = P::Variant;
    type Storage = A::Storage;

    fn try_read(&self) -> Option<<A::Storage as AnyStorage>::Ref<'static, P::Variant>> {
        let prism = &self.op.prism;
        self.parent
            .try_read()
            .and_then(|r| A::Storage::try_map(r, |s| prism.try_ref(s)))
    }
}

impl<A, P> AccessMut for Combinator<A, OptPrismOp<P>>
where
    A: AccessMut<Target = P::Source>,
    P: Prism,
{
    type WriteMetadata = A::WriteMetadata;

    fn try_write(
        &self,
    ) -> Option<WriteLock<'static, P::Variant, A::Storage, A::WriteMetadata>> {
        let prism = &self.op.prism;
        self.parent
            .try_write()
            .and_then(|w| WriteLock::filter_map(w, |s| prism.try_mut(s)))
    }
}

impl<P> Resolve<OptPrismOp<P>> for Option<P::Variant>
where
    P: Prism,
{
    type Input = Option<P::Source>;
    fn resolve(input: Option<P::Source>, op: &OptPrismOp<P>) -> Self {
        input.and_then(|s| op.prism.try_into_variant(s))
    }
}

// ============================================================================
// Stdlib prisms
// ============================================================================

/// Prism onto the `Some` variant of `Option<T>`.
pub struct SomePrism<T>(PhantomData<fn() -> T>);

impl<T> SomePrism<T> {
    pub const fn new() -> Self {
        Self(PhantomData)
    }
}

impl<T> Default for SomePrism<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Clone for SomePrism<T> {
    fn clone(&self) -> Self {
        Self(PhantomData)
    }
}

impl<T: 'static> Prism for SomePrism<T> {
    type Source = Option<T>;
    type Variant = T;

    fn try_ref<'a>(&self, source: &'a Option<T>) -> Option<&'a T> {
        source.as_ref()
    }
    fn try_mut<'a>(&self, source: &'a mut Option<T>) -> Option<&'a mut T> {
        source.as_mut()
    }
    fn try_into_variant(&self, source: Option<T>) -> Option<T> {
        source
    }
}

/// Prism onto the `Ok` variant of `Result<T, E>`.
pub struct OkPrism<T, E>(PhantomData<fn() -> (T, E)>);

impl<T, E> OkPrism<T, E> {
    pub const fn new() -> Self {
        Self(PhantomData)
    }
}

impl<T, E> Default for OkPrism<T, E> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T, E> Clone for OkPrism<T, E> {
    fn clone(&self) -> Self {
        Self(PhantomData)
    }
}

impl<T: 'static, E: 'static> Prism for OkPrism<T, E> {
    type Source = Result<T, E>;
    type Variant = T;

    fn try_ref<'a>(&self, source: &'a Result<T, E>) -> Option<&'a T> {
        source.as_ref().ok()
    }
    fn try_mut<'a>(&self, source: &'a mut Result<T, E>) -> Option<&'a mut T> {
        source.as_mut().ok()
    }
    fn try_into_variant(&self, source: Result<T, E>) -> Option<T> {
        source.ok()
    }
}

/// Prism onto the `Err` variant of `Result<T, E>`.
pub struct ErrPrism<T, E>(PhantomData<fn() -> (T, E)>);

impl<T, E> ErrPrism<T, E> {
    pub const fn new() -> Self {
        Self(PhantomData)
    }
}

impl<T, E> Default for ErrPrism<T, E> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T, E> Clone for ErrPrism<T, E> {
    fn clone(&self) -> Self {
        Self(PhantomData)
    }
}

impl<T: 'static, E: 'static> Prism for ErrPrism<T, E> {
    type Source = Result<T, E>;
    type Variant = E;

    fn try_ref<'a>(&self, source: &'a Result<T, E>) -> Option<&'a E> {
        source.as_ref().err()
    }
    fn try_mut<'a>(&self, source: &'a mut Result<T, E>) -> Option<&'a mut E> {
        source.as_mut().err()
    }
    fn try_into_variant(&self, source: Result<T, E>) -> Option<E> {
        source.err()
    }
}

/// Inline prism built from caller-supplied `fn` pointers.
pub struct InlinePrism<S, V> {
    pub(crate) try_ref: fn(&S) -> Option<&V>,
    pub(crate) try_mut: fn(&mut S) -> Option<&mut V>,
    pub(crate) try_into: fn(S) -> Option<V>,
}

impl<S, V> InlinePrism<S, V> {
    pub const fn new(
        try_ref: fn(&S) -> Option<&V>,
        try_mut: fn(&mut S) -> Option<&mut V>,
        try_into: fn(S) -> Option<V>,
    ) -> Self {
        Self { try_ref, try_mut, try_into }
    }
}

impl<S, V> Clone for InlinePrism<S, V> {
    fn clone(&self) -> Self {
        Self {
            try_ref: self.try_ref,
            try_mut: self.try_mut,
            try_into: self.try_into,
        }
    }
}

impl<S: 'static, V: 'static> Prism for InlinePrism<S, V> {
    type Source = S;
    type Variant = V;

    fn try_ref<'a>(&self, s: &'a S) -> Option<&'a V> {
        (self.try_ref)(s)
    }
    fn try_mut<'a>(&self, s: &'a mut S) -> Option<&'a mut V> {
        (self.try_mut)(s)
    }
    fn try_into_variant(&self, s: S) -> Option<V> {
        (self.try_into)(s)
    }
}

// ============================================================================
// Bridge: dioxus_signals::Readable / Writable carriers implement Access
// ============================================================================

/// Marker routing every [`dioxus_signals::Readable`] into [`Access`] without
/// triggering coherence conflicts with per-Op impls.
pub trait ReadCarrier {
    type Target: 'static;
    type Storage: AnyStorage;

    fn read_carrier(&self) -> <Self::Storage as AnyStorage>::Ref<'static, Self::Target>;
}

/// Marker routing every [`dioxus_signals::Writable`] into [`AccessMut`].
pub trait WriteCarrier: ReadCarrier {
    type WriteMetadata;

    fn write_carrier(
        &self,
    ) -> WriteLock<'static, Self::Target, Self::Storage, Self::WriteMetadata>;
}

impl<R> ReadCarrier for R
where
    R: dioxus_signals::Readable,
    R::Target: Sized + 'static,
{
    type Target = R::Target;
    type Storage = R::Storage;

    fn read_carrier(&self) -> <Self::Storage as AnyStorage>::Ref<'static, Self::Target> {
        <R as dioxus_signals::Readable>::try_read_unchecked(self)
            .expect("optics: carrier read failed")
    }
}

impl<W> WriteCarrier for W
where
    W: dioxus_signals::Writable,
    W::Target: Sized + 'static,
{
    type WriteMetadata = <W as dioxus_signals::Writable>::WriteMetadata;

    fn write_carrier(
        &self,
    ) -> WriteLock<'static, Self::Target, Self::Storage, Self::WriteMetadata> {
        <W as dioxus_signals::Writable>::try_write_unchecked(self)
            .expect("optics: carrier write failed")
    }
}

impl<R> Access for R
where
    R: ReadCarrier,
{
    type Target = <R as ReadCarrier>::Target;
    type Storage = <R as ReadCarrier>::Storage;

    fn try_read(&self) -> Option<<Self::Storage as AnyStorage>::Ref<'static, Self::Target>> {
        Some(ReadCarrier::read_carrier(self))
    }
}

impl<W> AccessMut for W
where
    W: WriteCarrier,
{
    type WriteMetadata = <W as WriteCarrier>::WriteMetadata;

    fn try_write(
        &self,
    ) -> Option<WriteLock<'static, Self::Target, Self::Storage, Self::WriteMetadata>> {
        Some(WriteCarrier::write_carrier(self))
    }
}

impl<R, T> ValueAccess<T> for R
where
    R: dioxus_signals::Readable<Target = T>,
    T: Clone + 'static,
{
    fn value(&self) -> T {
        <R as dioxus_signals::Readable>::try_read_unchecked(self)
            .expect("optics: carrier read failed")
            .clone()
    }
}
