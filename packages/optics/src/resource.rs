use std::{
    cell::Ref,
    cell::RefCell,
    future::Future,
    marker::PhantomData,
    pin::Pin,
    rc::Rc,
    task::{Context, Poll, Waker},
};

use dioxus_signals::{CopyValue, ReadableExt, UnsyncStorage, WritableExt, WriteLock};
use generational_box::GenerationalRef;

use crate::{
    collection::FlattenSomeOp,
    combinator::{
        FutureProjection, LensOp, ReadProjectionOpt, Resolve, Transform, UnwrapErrOp,
        UnwrapErrOptionalOp, UnwrapOkOp, UnwrapOkOptionalOp, UnwrapSomeOp, UnwrapSomeOptionalOp,
        ValueProjection, WriteProjectionOpt,
    },
};

/// Resource carrier that offers an immediate optional value and an eventual
/// future value.
pub struct Resource<T> {
    cell: CopyValue<Option<T>>,
    waiters: Rc<RefCell<Vec<Waker>>>,
}

impl<T> Clone for Resource<T> {
    fn clone(&self) -> Self {
        Self {
            cell: self.cell.clone(),
            waiters: self.waiters.clone(),
        }
    }
}

impl<T: 'static> Default for Resource<T> {
    fn default() -> Self {
        Self::pending()
    }
}

impl<T: Clone + 'static> ValueProjection<Option<T>> for Resource<T> {
    fn value_projection(&self) -> Option<T> {
        self.cell.read_unchecked().clone()
    }
}

impl<T: 'static> Resource<T> {
    /// Create a pending resource.
    #[must_use]
    pub fn pending() -> Self {
        Self {
            cell: CopyValue::new(None),
            waiters: Rc::new(RefCell::new(Vec::new())),
        }
    }

    /// Create a resolved resource.
    #[must_use]
    pub fn resolved(value: T) -> Self {
        Self {
            cell: CopyValue::new(Some(value)),
            waiters: Rc::new(RefCell::new(Vec::new())),
        }
    }

    /// Replace the resource value.
    pub fn resolve(&self, value: T) {
        *self.cell.write_unchecked() = Some(value);
        let waiters = std::mem::take(&mut *self.waiters.borrow_mut());
        for waiter in waiters {
            waiter.wake();
        }
    }
}

impl<T: 'static> ReadProjectionOpt<T> for Resource<T> {
    fn read_projection_opt(&self) -> Option<GenerationalRef<Ref<'static, T>>> {
        self.cell
            .read_unchecked()
            .try_map(|r| Ref::filter_map(r, |o| o.as_ref()).ok())
    }
}

impl<T: 'static> WriteProjectionOpt<T> for Resource<T> {
    fn write_projection_opt(&self) -> Option<WriteLock<'static, T, UnsyncStorage>> {
        WriteLock::filter_map(self.cell.write_unchecked(), |o| o.as_mut())
    }
}

impl<T: Clone + 'static> FutureProjection<AsFuture<ResourceFuture<T>>> for Resource<T> {
    fn future_projection(&self) -> AsFuture<ResourceFuture<T>> {
        AsFuture(ResourceFuture {
            cell: self.cell.clone(),
            waiters: self.waiters.clone(),
        })
    }
}

/// Future yielded by [`Resource`].
pub struct ResourceFuture<T> {
    cell: CopyValue<Option<T>>,
    waiters: Rc<RefCell<Vec<Waker>>>,
}

impl<T: Clone + 'static> Future for ResourceFuture<T> {
    type Output = T;

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<T> {
        match self.cell.read_unchecked().as_ref() {
            Some(v) => Poll::Ready(v.clone()),
            None => {
                let mut waiters = self.waiters.borrow_mut();
                if !waiters.iter().any(|waiter| waiter.will_wake(_cx.waker())) {
                    waiters.push(_cx.waker().clone());
                }
                Poll::Pending
            }
        }
    }
}

/// Generic await-time transform that maps the resolved future output through an
/// optics operation.
pub struct AwaitTransform<Fut, Op, Out> {
    future: Fut,
    op: Op,
    _marker: PhantomData<fn() -> Out>,
}

impl<Fut, Op, Out> AwaitTransform<Fut, Op, Out> {
    /// Wrap `future` with an owned transform `op` applied at resolve time.
    pub fn new(future: Fut, op: Op) -> Self {
        Self {
            future,
            op,
            _marker: PhantomData,
        }
    }
}

impl<Fut, Op, Out> Future for AwaitTransform<Fut, Op, Out>
where
    Fut: Future<Output = Out::Input>,
    Out: Resolve<Op>,
{
    type Output = Out;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Out> {
        // Safety: `future` is pinned with `self`, and we never move it after pinning.
        let this = unsafe { self.get_unchecked_mut() };
        match unsafe { Pin::new_unchecked(&mut this.future) }.poll(cx) {
            Poll::Ready(value) => Poll::Ready(Out::resolve(value, &this.op)),
            Poll::Pending => Poll::Pending,
        }
    }
}

/// Future-side field projection.
pub type FutureProject<Fut, S, U> = AsFuture<AwaitTransform<Fut, LensOp<S, U>, U>>;

/// Wrapper that exposes a future as a distinct carrier output.
pub struct AsFuture<Fut>(pub Fut);

impl<Fut: Future> Future for AsFuture<Fut> {
    type Output = Fut::Output;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // Safety: the inner future is pinned together with the wrapper.
        unsafe { self.map_unchecked_mut(|inner| &mut inner.0) }.poll(cx)
    }
}

impl<S, U> Resolve<LensOp<S, U>> for U
where
    U: Clone,
{
    type Input = S;

    fn resolve(input: Self::Input, op: &LensOp<S, U>) -> Self {
        (op.read)(&input).clone()
    }
}

impl<S, U> Resolve<LensOp<S, U>> for Option<U>
where
    U: Clone,
{
    type Input = Option<S>;

    fn resolve(input: Self::Input, op: &LensOp<S, U>) -> Self {
        input.map(|value| (op.read)(&value).clone())
    }
}

impl<T> Resolve<UnwrapSomeOp<T>> for Option<T> {
    type Input = Option<T>;

    fn resolve(input: Self::Input, _: &UnwrapSomeOp<T>) -> Self {
        input
    }
}

impl<T> Resolve<UnwrapSomeOptionalOp<T>> for Option<T> {
    type Input = Option<Option<T>>;

    fn resolve(input: Self::Input, _: &UnwrapSomeOptionalOp<T>) -> Self {
        input.flatten()
    }
}

impl<T, E> Resolve<UnwrapOkOp<T, E>> for Option<T> {
    type Input = Result<T, E>;

    fn resolve(input: Self::Input, _: &UnwrapOkOp<T, E>) -> Self {
        input.ok()
    }
}

impl<T, E> Resolve<UnwrapOkOptionalOp<T, E>> for Option<T> {
    type Input = Option<Result<T, E>>;

    fn resolve(input: Self::Input, _: &UnwrapOkOptionalOp<T, E>) -> Self {
        input.and_then(Result::ok)
    }
}

impl<T, E> Resolve<UnwrapErrOp<T, E>> for Option<E> {
    type Input = Result<T, E>;

    fn resolve(input: Self::Input, _: &UnwrapErrOp<T, E>) -> Self {
        input.err()
    }
}

impl<T, E> Resolve<UnwrapErrOptionalOp<T, E>> for Option<E> {
    type Input = Option<Result<T, E>>;

    fn resolve(input: Self::Input, _: &UnwrapErrOptionalOp<T, E>) -> Self {
        input.and_then(Result::err)
    }
}

impl<X> Resolve<FlattenSomeOp> for Option<X> {
    type Input = Option<Option<X>>;

    fn resolve(input: Self::Input, _: &FlattenSomeOp) -> Self {
        input.flatten()
    }
}

impl<Fut, Op, Out> Transform<Op> for AsFuture<AwaitTransform<Fut, Op, Out>>
where
    Fut: Future,
    Op: Clone,
    Out: Resolve<Op>,
{
    type Input = AsFuture<Fut>;

    fn transform(input: Self::Input, op: &Op) -> Self {
        AsFuture(AwaitTransform::new(input.0, op.clone()))
    }
}
