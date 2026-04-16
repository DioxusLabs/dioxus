use std::{cell::Ref, future::Future, marker::PhantomData};

use dioxus_signals::{UnsyncStorage, WriteLock};
use generational_box::GenerationalRef;

use crate::resource::{AsFuture, AwaitTransform};

/// Low-level live read access for a projected child.
pub trait ReadProjection<T: 'static> {
    /// Borrow the projected child.
    fn read_projection(&self) -> GenerationalRef<Ref<'static, T>>;
}

/// Low-level live write access for a projected child.
pub trait WriteProjection<T: 'static> {
    /// Borrow the projected child mutably.
    fn write_projection(&self) -> WriteLock<'static, T, UnsyncStorage>;
}

/// Low-level optional live read access for a projected child.
pub trait ReadProjectionOpt<T: 'static> {
    /// Borrow the projected child if it exists.
    fn read_projection_opt(&self) -> Option<GenerationalRef<Ref<'static, T>>>;
}

/// Low-level optional live write access for a projected child.
pub trait WriteProjectionOpt<T: 'static> {
    /// Borrow the projected child mutably if it exists.
    fn write_projection_opt(&self) -> Option<WriteLock<'static, T, UnsyncStorage>>;
}

/// Low-level owned value extraction for a projected child.
pub trait ValueProjection<T> {
    /// Clone or otherwise materialize the current owned value.
    fn value_projection(&self) -> T;
}

/// Low-level future extraction for a projected child.
pub trait FutureProjection<Fut>
where
    Fut: Future,
{
    /// Produce the future for this projected child.
    fn future_projection(&self) -> Fut;
}

/// Generic combinator that applies an optics operation to its parent carrier.
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

/// Convert one carrier output into another through an operation `Op`.
pub trait Transform<Op> {
    /// The input required before the operation is applied.
    type Input;

    /// Apply the transform.
    fn transform(input: Self::Input, op: &Op) -> Self;
}

/// Convert an owned resolved value into another owned value through `Op`.
pub trait Resolve<Op> {
    /// The owned input consumed before the operation is applied.
    type Input;

    /// Apply the owned transform.
    fn resolve(input: Self::Input, op: &Op) -> Self;
}

impl<A, Op, Out> ValueProjection<Out> for Combinator<A, Op>
where
    Out: Transform<Op>,
    A: ValueProjection<Out::Input>,
{
    fn value_projection(&self) -> Out {
        Out::transform(self.parent.value_projection(), &self.op)
    }
}

impl<A, Op, Fut, Out> FutureProjection<AsFuture<AwaitTransform<Fut, Op, Out>>> for Combinator<A, Op>
where
    A: FutureProjection<AsFuture<Fut>>,
    Fut: Future<Output = Out::Input>,
    Op: Clone,
    Out: Resolve<Op>,
{
    fn future_projection(&self) -> AsFuture<AwaitTransform<Fut, Op, Out>> {
        AsFuture(AwaitTransform::new(
            self.parent.future_projection().0,
            self.op.clone(),
        ))
    }
}

/// Field projection from `S` to `U` through paired read/write functions.
#[derive(Clone)]
pub struct LensOp<S, U> {
    pub(crate) read: fn(&S) -> &U,
    pub(crate) write: fn(&mut S) -> &mut U,
}

impl<S: 'static, U: 'static> Transform<LensOp<S, U>> for GenerationalRef<Ref<'static, U>> {
    type Input = GenerationalRef<Ref<'static, S>>;

    fn transform(input: Self::Input, op: &LensOp<S, U>) -> Self {
        GenerationalRef::map(input, |r| Ref::map(r, op.read))
    }
}

impl<S, U> Transform<LensOp<S, U>> for U
where
    U: Clone,
{
    type Input = S;

    fn transform(input: Self::Input, op: &LensOp<S, U>) -> Self {
        (op.read)(&input).clone()
    }
}

impl<S, U> Transform<LensOp<S, U>> for Option<U>
where
    U: Clone,
{
    type Input = Option<S>;

    fn transform(input: Self::Input, op: &LensOp<S, U>) -> Self {
        input.map(|value| (op.read)(&value).clone())
    }
}

impl<S: 'static, U: 'static> Transform<LensOp<S, U>> for WriteLock<'static, U, UnsyncStorage> {
    type Input = WriteLock<'static, S, UnsyncStorage>;

    fn transform(input: Self::Input, op: &LensOp<S, U>) -> Self {
        WriteLock::map(input, op.write)
    }
}

impl<S: 'static, U: 'static> Transform<LensOp<S, U>> for Option<GenerationalRef<Ref<'static, U>>> {
    type Input = Option<GenerationalRef<Ref<'static, S>>>;

    fn transform(input: Self::Input, op: &LensOp<S, U>) -> Self {
        input.map(|inner| GenerationalRef::map(inner, |r| Ref::map(r, op.read)))
    }
}

impl<S: 'static, U: 'static> Transform<LensOp<S, U>>
    for Option<WriteLock<'static, U, UnsyncStorage>>
{
    type Input = Option<WriteLock<'static, S, UnsyncStorage>>;

    fn transform(input: Self::Input, op: &LensOp<S, U>) -> Self {
        input.map(|inner| WriteLock::map(inner, op.write))
    }
}

impl<A, S: 'static, U: 'static> ReadProjection<U> for Combinator<A, LensOp<S, U>>
where
    A: ReadProjection<S>,
{
    fn read_projection(&self) -> GenerationalRef<Ref<'static, U>> {
        let input = self.parent.read_projection();
        GenerationalRef::map(input, |r| Ref::map(r, self.op.read))
    }
}

impl<A, S: 'static, U: 'static> WriteProjection<U> for Combinator<A, LensOp<S, U>>
where
    A: WriteProjection<S>,
{
    fn write_projection(&self) -> WriteLock<'static, U, UnsyncStorage> {
        WriteLock::map(self.parent.write_projection(), self.op.write)
    }
}

impl<A, S: 'static, U: 'static> ReadProjectionOpt<U> for Combinator<A, LensOp<S, U>>
where
    A: ReadProjectionOpt<S>,
{
    fn read_projection_opt(&self) -> Option<GenerationalRef<Ref<'static, U>>> {
        self.parent
            .read_projection_opt()
            .map(|inner| GenerationalRef::map(inner, |r| Ref::map(r, self.op.read)))
    }
}

impl<A, S: 'static, U: 'static> WriteProjectionOpt<U> for Combinator<A, LensOp<S, U>>
where
    A: WriteProjectionOpt<S>,
{
    fn write_projection_opt(&self) -> Option<WriteLock<'static, U, UnsyncStorage>> {
        self.parent
            .write_projection_opt()
            .map(|inner| WriteLock::map(inner, self.op.write))
    }
}

/// Option-lifting operation used by required-path `map_some`.
pub struct UnwrapSomeOp<T>(pub(crate) PhantomData<fn() -> T>);

impl<T> Clone for UnwrapSomeOp<T> {
    fn clone(&self) -> Self {
        Self(PhantomData)
    }
}

impl<T: 'static> Transform<UnwrapSomeOp<T>> for Option<GenerationalRef<Ref<'static, T>>> {
    type Input = GenerationalRef<Ref<'static, Option<T>>>;

    fn transform(input: Self::Input, _: &UnwrapSomeOp<T>) -> Self {
        input.try_map(|r| Ref::filter_map(r, |o| o.as_ref()).ok())
    }
}

impl<T: 'static> Transform<UnwrapSomeOp<T>> for Option<WriteLock<'static, T, UnsyncStorage>> {
    type Input = WriteLock<'static, Option<T>, UnsyncStorage>;

    fn transform(input: Self::Input, _: &UnwrapSomeOp<T>) -> Self {
        WriteLock::filter_map(input, |o| o.as_mut())
    }
}

impl<T> Transform<UnwrapSomeOp<T>> for Option<T> {
    type Input = Option<T>;

    fn transform(input: Self::Input, _: &UnwrapSomeOp<T>) -> Self {
        input
    }
}

impl<A, T: 'static> ReadProjectionOpt<T> for Combinator<A, UnwrapSomeOp<T>>
where
    A: ReadProjection<Option<T>>,
{
    fn read_projection_opt(&self) -> Option<GenerationalRef<Ref<'static, T>>> {
        self.parent
            .read_projection()
            .try_map(|r| Ref::filter_map(r, |o| o.as_ref()).ok())
    }
}

impl<A, T: 'static> WriteProjectionOpt<T> for Combinator<A, UnwrapSomeOp<T>>
where
    A: WriteProjection<Option<T>>,
{
    fn write_projection_opt(&self) -> Option<WriteLock<'static, T, UnsyncStorage>> {
        WriteLock::filter_map(self.parent.write_projection(), |o| o.as_mut())
    }
}

/// Option-lifting operation used by optional-path `map_some`.
pub struct UnwrapSomeOptionalOp<T>(pub(crate) PhantomData<fn() -> T>);

impl<T> Clone for UnwrapSomeOptionalOp<T> {
    fn clone(&self) -> Self {
        Self(PhantomData)
    }
}

impl<T: 'static> Transform<UnwrapSomeOptionalOp<T>> for Option<GenerationalRef<Ref<'static, T>>> {
    type Input = Option<GenerationalRef<Ref<'static, Option<T>>>>;

    fn transform(input: Self::Input, _: &UnwrapSomeOptionalOp<T>) -> Self {
        input.and_then(|inner| inner.try_map(|r| Ref::filter_map(r, |o| o.as_ref()).ok()))
    }
}

impl<T: 'static> Transform<UnwrapSomeOptionalOp<T>>
    for Option<WriteLock<'static, T, UnsyncStorage>>
{
    type Input = Option<WriteLock<'static, Option<T>, UnsyncStorage>>;

    fn transform(input: Self::Input, _: &UnwrapSomeOptionalOp<T>) -> Self {
        input.and_then(|inner| WriteLock::filter_map(inner, |o| o.as_mut()))
    }
}

impl<T> Transform<UnwrapSomeOptionalOp<T>> for Option<T> {
    type Input = Option<Option<T>>;

    fn transform(input: Self::Input, _: &UnwrapSomeOptionalOp<T>) -> Self {
        input.flatten()
    }
}

impl<A, T: 'static> ReadProjectionOpt<T> for Combinator<A, UnwrapSomeOptionalOp<T>>
where
    A: ReadProjectionOpt<Option<T>>,
{
    fn read_projection_opt(&self) -> Option<GenerationalRef<Ref<'static, T>>> {
        self.parent
            .read_projection_opt()
            .and_then(|inner| inner.try_map(|r| Ref::filter_map(r, |o| o.as_ref()).ok()))
    }
}

impl<A, T: 'static> WriteProjectionOpt<T> for Combinator<A, UnwrapSomeOptionalOp<T>>
where
    A: WriteProjectionOpt<Option<T>>,
{
    fn write_projection_opt(&self) -> Option<WriteLock<'static, T, UnsyncStorage>> {
        self.parent
            .write_projection_opt()
            .and_then(|inner| WriteLock::filter_map(inner, |o| o.as_mut()))
    }
}

/// `Result<T, E>::Ok(T)` projection used by required-path `map_ok`.
pub struct UnwrapOkOp<T, E>(pub(crate) PhantomData<fn() -> Result<T, E>>);

impl<T, E> Clone for UnwrapOkOp<T, E> {
    fn clone(&self) -> Self {
        Self(PhantomData)
    }
}

impl<T: 'static, E: 'static> Transform<UnwrapOkOp<T, E>>
    for Option<GenerationalRef<Ref<'static, T>>>
{
    type Input = GenerationalRef<Ref<'static, Result<T, E>>>;

    fn transform(input: Self::Input, _: &UnwrapOkOp<T, E>) -> Self {
        input.try_map(|r| Ref::filter_map(r, |result| result.as_ref().ok()).ok())
    }
}

impl<T: 'static, E: 'static> Transform<UnwrapOkOp<T, E>>
    for Option<WriteLock<'static, T, UnsyncStorage>>
{
    type Input = WriteLock<'static, Result<T, E>, UnsyncStorage>;

    fn transform(input: Self::Input, _: &UnwrapOkOp<T, E>) -> Self {
        WriteLock::filter_map(input, |result| result.as_mut().ok())
    }
}

impl<T, E> Transform<UnwrapOkOp<T, E>> for Option<T> {
    type Input = Result<T, E>;

    fn transform(input: Self::Input, _: &UnwrapOkOp<T, E>) -> Self {
        input.ok()
    }
}

impl<A, T: 'static, E: 'static> ReadProjectionOpt<T> for Combinator<A, UnwrapOkOp<T, E>>
where
    A: ReadProjection<Result<T, E>>,
{
    fn read_projection_opt(&self) -> Option<GenerationalRef<Ref<'static, T>>> {
        self.parent
            .read_projection()
            .try_map(|r| Ref::filter_map(r, |result| result.as_ref().ok()).ok())
    }
}

impl<A, T: 'static, E: 'static> WriteProjectionOpt<T> for Combinator<A, UnwrapOkOp<T, E>>
where
    A: WriteProjection<Result<T, E>>,
{
    fn write_projection_opt(&self) -> Option<WriteLock<'static, T, UnsyncStorage>> {
        WriteLock::filter_map(self.parent.write_projection(), |result| {
            result.as_mut().ok()
        })
    }
}

/// `Result<T, E>::Ok(T)` projection used by optional-path `map_ok`.
pub struct UnwrapOkOptionalOp<T, E>(pub(crate) PhantomData<fn() -> Result<T, E>>);

impl<T, E> Clone for UnwrapOkOptionalOp<T, E> {
    fn clone(&self) -> Self {
        Self(PhantomData)
    }
}

impl<T: 'static, E: 'static> Transform<UnwrapOkOptionalOp<T, E>>
    for Option<GenerationalRef<Ref<'static, T>>>
{
    type Input = Option<GenerationalRef<Ref<'static, Result<T, E>>>>;

    fn transform(input: Self::Input, _: &UnwrapOkOptionalOp<T, E>) -> Self {
        input.and_then(|inner| {
            inner.try_map(|r| Ref::filter_map(r, |result| result.as_ref().ok()).ok())
        })
    }
}

impl<T: 'static, E: 'static> Transform<UnwrapOkOptionalOp<T, E>>
    for Option<WriteLock<'static, T, UnsyncStorage>>
{
    type Input = Option<WriteLock<'static, Result<T, E>, UnsyncStorage>>;

    fn transform(input: Self::Input, _: &UnwrapOkOptionalOp<T, E>) -> Self {
        input.and_then(|inner| WriteLock::filter_map(inner, |result| result.as_mut().ok()))
    }
}

impl<T, E> Transform<UnwrapOkOptionalOp<T, E>> for Option<T> {
    type Input = Option<Result<T, E>>;

    fn transform(input: Self::Input, _: &UnwrapOkOptionalOp<T, E>) -> Self {
        input.and_then(Result::ok)
    }
}

impl<A, T: 'static, E: 'static> ReadProjectionOpt<T> for Combinator<A, UnwrapOkOptionalOp<T, E>>
where
    A: ReadProjectionOpt<Result<T, E>>,
{
    fn read_projection_opt(&self) -> Option<GenerationalRef<Ref<'static, T>>> {
        self.parent.read_projection_opt().and_then(|inner| {
            inner.try_map(|r| Ref::filter_map(r, |result| result.as_ref().ok()).ok())
        })
    }
}

impl<A, T: 'static, E: 'static> WriteProjectionOpt<T> for Combinator<A, UnwrapOkOptionalOp<T, E>>
where
    A: WriteProjectionOpt<Result<T, E>>,
{
    fn write_projection_opt(&self) -> Option<WriteLock<'static, T, UnsyncStorage>> {
        self.parent
            .write_projection_opt()
            .and_then(|inner| WriteLock::filter_map(inner, |result| result.as_mut().ok()))
    }
}

/// `Result<T, E>::Err(E)` projection used by required-path `map_err`.
pub struct UnwrapErrOp<T, E>(pub(crate) PhantomData<fn() -> Result<T, E>>);

impl<T, E> Clone for UnwrapErrOp<T, E> {
    fn clone(&self) -> Self {
        Self(PhantomData)
    }
}

impl<T: 'static, E: 'static> Transform<UnwrapErrOp<T, E>>
    for Option<GenerationalRef<Ref<'static, E>>>
{
    type Input = GenerationalRef<Ref<'static, Result<T, E>>>;

    fn transform(input: Self::Input, _: &UnwrapErrOp<T, E>) -> Self {
        input.try_map(|r| Ref::filter_map(r, |result| result.as_ref().err()).ok())
    }
}

impl<T: 'static, E: 'static> Transform<UnwrapErrOp<T, E>>
    for Option<WriteLock<'static, E, UnsyncStorage>>
{
    type Input = WriteLock<'static, Result<T, E>, UnsyncStorage>;

    fn transform(input: Self::Input, _: &UnwrapErrOp<T, E>) -> Self {
        WriteLock::filter_map(input, |result| result.as_mut().err())
    }
}

impl<T, E> Transform<UnwrapErrOp<T, E>> for Option<E> {
    type Input = Result<T, E>;

    fn transform(input: Self::Input, _: &UnwrapErrOp<T, E>) -> Self {
        input.err()
    }
}

impl<A, T: 'static, E: 'static> ReadProjectionOpt<E> for Combinator<A, UnwrapErrOp<T, E>>
where
    A: ReadProjection<Result<T, E>>,
{
    fn read_projection_opt(&self) -> Option<GenerationalRef<Ref<'static, E>>> {
        self.parent
            .read_projection()
            .try_map(|r| Ref::filter_map(r, |result| result.as_ref().err()).ok())
    }
}

impl<A, T: 'static, E: 'static> WriteProjectionOpt<E> for Combinator<A, UnwrapErrOp<T, E>>
where
    A: WriteProjection<Result<T, E>>,
{
    fn write_projection_opt(&self) -> Option<WriteLock<'static, E, UnsyncStorage>> {
        WriteLock::filter_map(self.parent.write_projection(), |result| {
            result.as_mut().err()
        })
    }
}

/// `Result<T, E>::Err(E)` projection used by optional-path `map_err`.
pub struct UnwrapErrOptionalOp<T, E>(pub(crate) PhantomData<fn() -> Result<T, E>>);

impl<T, E> Clone for UnwrapErrOptionalOp<T, E> {
    fn clone(&self) -> Self {
        Self(PhantomData)
    }
}

impl<T: 'static, E: 'static> Transform<UnwrapErrOptionalOp<T, E>>
    for Option<GenerationalRef<Ref<'static, E>>>
{
    type Input = Option<GenerationalRef<Ref<'static, Result<T, E>>>>;

    fn transform(input: Self::Input, _: &UnwrapErrOptionalOp<T, E>) -> Self {
        input.and_then(|inner| {
            inner.try_map(|r| Ref::filter_map(r, |result| result.as_ref().err()).ok())
        })
    }
}

impl<T: 'static, E: 'static> Transform<UnwrapErrOptionalOp<T, E>>
    for Option<WriteLock<'static, E, UnsyncStorage>>
{
    type Input = Option<WriteLock<'static, Result<T, E>, UnsyncStorage>>;

    fn transform(input: Self::Input, _: &UnwrapErrOptionalOp<T, E>) -> Self {
        input.and_then(|inner| WriteLock::filter_map(inner, |result| result.as_mut().err()))
    }
}

impl<T, E> Transform<UnwrapErrOptionalOp<T, E>> for Option<E> {
    type Input = Option<Result<T, E>>;

    fn transform(input: Self::Input, _: &UnwrapErrOptionalOp<T, E>) -> Self {
        input.and_then(Result::err)
    }
}

impl<A, T: 'static, E: 'static> ReadProjectionOpt<E> for Combinator<A, UnwrapErrOptionalOp<T, E>>
where
    A: ReadProjectionOpt<Result<T, E>>,
{
    fn read_projection_opt(&self) -> Option<GenerationalRef<Ref<'static, E>>> {
        self.parent.read_projection_opt().and_then(|inner| {
            inner.try_map(|r| Ref::filter_map(r, |result| result.as_ref().err()).ok())
        })
    }
}

impl<A, T: 'static, E: 'static> WriteProjectionOpt<E> for Combinator<A, UnwrapErrOptionalOp<T, E>>
where
    A: WriteProjectionOpt<Result<T, E>>,
{
    fn write_projection_opt(&self) -> Option<WriteLock<'static, E, UnsyncStorage>> {
        self.parent
            .write_projection_opt()
            .and_then(|inner| WriteLock::filter_map(inner, |result| result.as_mut().err()))
    }
}
