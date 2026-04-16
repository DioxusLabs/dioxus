use std::{
    cell::RefCell,
    future::Future,
    marker::PhantomData,
    pin::Pin,
    rc::Rc,
    task::{Context, Poll, Waker},
};

use dioxus_signals::{CopyValue, ReadableExt, UnsyncStorage, WritableExt, WriteLock};
use generational_box::AnyStorage;

use crate::combinator::{
    Access, AccessMut, FutureAccess, LensOp, Resolve, ValueAccess,
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

impl<T: Clone + 'static> ValueAccess<Option<T>> for Resource<T> {
    fn value(&self) -> Option<T> {
        self.cell.read_unchecked().clone()
    }
}

impl<T: 'static> Resource<T> {
    #[must_use]
    pub fn pending() -> Self {
        Self {
            cell: CopyValue::new(None),
            waiters: Rc::new(RefCell::new(Vec::new())),
        }
    }

    #[must_use]
    pub fn resolved(value: T) -> Self {
        Self {
            cell: CopyValue::new(Some(value)),
            waiters: Rc::new(RefCell::new(Vec::new())),
        }
    }

    pub fn resolve(&self, value: T) {
        *self.cell.write_unchecked() = Some(value);
        let waiters = std::mem::take(&mut *self.waiters.borrow_mut());
        for waiter in waiters {
            waiter.wake();
        }
    }
}

impl<T: 'static> Access for Resource<T> {
    type Target = T;
    type Storage = UnsyncStorage;

    fn try_read(&self) -> Option<<UnsyncStorage as AnyStorage>::Ref<'static, T>> {
        UnsyncStorage::try_map(self.cell.read_unchecked(), |o| o.as_ref())
    }
}

impl<T: 'static> AccessMut for Resource<T> {
    type WriteMetadata = ();

    fn try_write(&self) -> Option<WriteLock<'static, T, UnsyncStorage, ()>> {
        WriteLock::filter_map(self.cell.write_unchecked(), |o| o.as_mut())
    }
}

impl<T: Clone + 'static> FutureAccess<AsFuture<ResourceFuture<T>>> for Resource<T> {
    fn future(&self) -> AsFuture<ResourceFuture<T>> {
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

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<T> {
        match self.cell.read_unchecked().as_ref() {
            Some(v) => Poll::Ready(v.clone()),
            None => {
                let mut waiters = self.waiters.borrow_mut();
                if !waiters.iter().any(|waiter| waiter.will_wake(cx.waker())) {
                    waiters.push(cx.waker().clone());
                }
                Poll::Pending
            }
        }
    }
}

/// Generic await-time transform that maps the resolved future output through
/// an optics op.
pub struct AwaitTransform<Fut, Op, Out> {
    future: Fut,
    op: Op,
    _marker: PhantomData<fn() -> Out>,
}

impl<Fut, Op, Out> AwaitTransform<Fut, Op, Out> {
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

// The owned-value `Resolve` impls for LensOp / PrismOp / OptPrismOp /
// FlattenSomeOp live in `combinator.rs` / `collection.rs`; the future channel
// reuses them via the generic `AwaitTransform<Fut, Op, Out>`.
