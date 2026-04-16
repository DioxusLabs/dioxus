use std::{future::Future, marker::PhantomData};

use generational_box::{AnyStorage, WriteLock};

use crate::path::{PathBuffer, PathSegment, Pathed};
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

    /// Borrow the projected child if the path currently resolves. Reactive
    /// roots (like `Signal`) subscribe the current reactive context here.
    fn try_read(&self) -> Option<<Self::Storage as AnyStorage>::Ref<'static, Self::Target>>;

    /// Like [`try_read`](Self::try_read) but does **not** subscribe the
    /// current reactive context to the underlying reactive root.
    ///
    /// Used by [`Subscribed`](crate::Subscribed) so that path-granular
    /// subscription can replace (rather than layer on top of) the root's
    /// one-big-subscription behavior. The default forwards to `try_read` —
    /// override in bridges that expose a peek-style read.
    fn try_peek(&self) -> Option<<Self::Storage as AnyStorage>::Ref<'static, Self::Target>> {
        self.try_read()
    }
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

impl<A, Op> Combinator<A, Op> {
    /// Build a combinator from a parent accessor and an operation. Used by
    /// the stores derive macro to construct lens chains from generated code.
    pub fn new(parent: A, op: Op) -> Self {
        Self { parent, op }
    }

    /// Borrow the parent accessor.
    pub fn parent(&self) -> &A {
        &self.parent
    }

    /// Borrow the operation.
    pub fn op(&self) -> &Op {
        &self.op
    }
}

impl<A: Copy, Op: Copy> Copy for Combinator<A, Op> {}

impl<A: Clone, Op: Clone> Clone for Combinator<A, Op> {
    fn clone(&self) -> Self {
        Self {
            parent: self.parent.clone(),
            op: self.op.clone(),
        }
    }
}

impl<A: PartialEq, Op: PartialEq> PartialEq for Combinator<A, Op> {
    fn eq(&self, other: &Self) -> bool {
        self.parent == other.parent && self.op == other.op
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
pub struct RefOp<S, U> {
    pub(crate) read: fn(&S) -> &U,
}

impl<S, U> Copy for RefOp<S, U> {}
impl<S, U> Clone for RefOp<S, U> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<S, U> RefOp<S, U> {
    /// Construct a read-only field projection from a `fn` pointer.
    pub const fn new(read: fn(&S) -> &U) -> Self {
        Self { read }
    }

    /// The underlying read function.
    pub const fn read_fn(&self) -> fn(&S) -> &U {
        self.read
    }
}

/// Read+write field projection from `S` to `U`.
pub struct LensOp<S, U> {
    pub(crate) read: fn(&S) -> &U,
    pub(crate) write: fn(&mut S) -> &mut U,
}

impl<S, U> Copy for LensOp<S, U> {}
impl<S, U> Clone for LensOp<S, U> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<S, U> LensOp<S, U> {
    /// Construct a field lens from a paired read/write function pair.
    pub const fn new(read: fn(&S) -> &U, write: fn(&mut S) -> &mut U) -> Self {
        Self { read, write }
    }

    /// The underlying read function.
    pub const fn read_fn(&self) -> fn(&S) -> &U {
        self.read
    }

    /// The underlying mutable-read function.
    pub const fn write_fn(&self) -> fn(&mut S) -> &mut U {
        self.write
    }
}

impl<S, U> PartialEq for LensOp<S, U> {
    fn eq(&self, other: &Self) -> bool {
        (self.read as *const ()) == (other.read as *const ())
            && (self.write as *const ()) == (other.write as *const ())
    }
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

    fn try_peek(&self) -> Option<<A::Storage as AnyStorage>::Ref<'static, U>> {
        self.parent
            .try_peek()
            .map(|r| A::Storage::map(r, self.op.read))
    }
}

impl<A, S, U> Pathed for Combinator<A, RefOp<S, U>>
where
    A: Pathed,
{
    fn visit_path(&self, sink: &mut PathBuffer) {
        self.parent.visit_path(sink);
        sink.push(PathSegment::field_fn(self.op.read));
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

    fn try_peek(&self) -> Option<<A::Storage as AnyStorage>::Ref<'static, U>> {
        self.parent
            .try_peek()
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

impl<A, S, U> Pathed for Combinator<A, LensOp<S, U>>
where
    A: Pathed,
{
    fn visit_path(&self, sink: &mut PathBuffer) {
        self.parent.visit_path(sink);
        sink.push(PathSegment::field_fn(self.op.read));
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

impl<P: Copy> Copy for PrismOp<P> {}

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

    fn try_peek(&self) -> Option<<A::Storage as AnyStorage>::Ref<'static, P::Variant>> {
        let prism = &self.op.prism;
        self.parent
            .try_peek()
            .and_then(|r| A::Storage::try_map(r, |s| prism.try_ref(s)))
    }
}

impl<A, P> Pathed for Combinator<A, PrismOp<P>>
where
    A: Pathed,
    P: Prism + 'static,
{
    fn visit_path(&self, sink: &mut PathBuffer) {
        self.parent.visit_path(sink);
        sink.push(PathSegment::prism_type::<P>());
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

impl<P: Copy> Copy for OptPrismOp<P> {}

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

    fn try_peek(&self) -> Option<<A::Storage as AnyStorage>::Ref<'static, P::Variant>> {
        let prism = &self.op.prism;
        self.parent
            .try_peek()
            .and_then(|r| A::Storage::try_map(r, |s| prism.try_ref(s)))
    }
}

impl<A, P> Pathed for Combinator<A, OptPrismOp<P>>
where
    A: Pathed,
    P: Prism + 'static,
{
    fn visit_path(&self, sink: &mut PathBuffer) {
        self.parent.visit_path(sink);
        sink.push(PathSegment::prism_type::<P>());
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

impl<T> Copy for SomePrism<T> {}

impl<T> Clone for SomePrism<T> {
    fn clone(&self) -> Self {
        *self
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

impl<T, E> Copy for OkPrism<T, E> {}

impl<T, E> Clone for OkPrism<T, E> {
    fn clone(&self) -> Self {
        *self
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

impl<T, E> Copy for ErrPrism<T, E> {}

impl<T, E> Clone for ErrPrism<T, E> {
    fn clone(&self) -> Self {
        *self
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

impl<S, V> Copy for InlinePrism<S, V> {}

impl<S, V> Clone for InlinePrism<S, V> {
    fn clone(&self) -> Self {
        *self
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
// Root carrier: GenerationalBox<T> implements Access / AccessMut / ValueAccess
// ============================================================================

use generational_box::GenerationalBox;

impl<T: 'static, S> Access for GenerationalBox<T, S>
where
    S: generational_box::Storage<T>,
{
    type Target = T;
    type Storage = S;

    fn try_read(&self) -> Option<<S as AnyStorage>::Ref<'static, T>> {
        Some(self.read())
    }
    // GenerationalBox has no reactive subscription of its own, so `try_peek`
    // uses the same path.
}

impl<T: 'static, S> AccessMut for GenerationalBox<T, S>
where
    S: generational_box::Storage<T>,
{
    type WriteMetadata = ();

    fn try_write(
        &self,
    ) -> Option<WriteLock<'static, T, S, ()>> {
        Some(WriteLock::new(self.write()))
    }
}

impl<T, S> ValueAccess<T> for GenerationalBox<T, S>
where
    T: Clone + 'static,
    S: generational_box::Storage<T>,
{
    fn value(&self) -> T {
        (*self.read()).clone()
    }
}

/// Roots contribute no path segments.
impl<T, S> Pathed for GenerationalBox<T, S>
where
    S: generational_box::Storage<T>,
{
    fn visit_path(&self, _sink: &mut PathBuffer) {}
}
