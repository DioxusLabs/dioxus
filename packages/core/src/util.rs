use std::cell::Cell;
use std::fmt::Display;

use crate::innerlude::*;

// create a cell with a "none" value
#[inline]
pub fn empty_cell() -> Cell<Option<ElementId>> {
    Cell::new(None)
}

pub fn type_name_of<T>(_: T) -> &'static str {
    std::any::type_name::<T>()
}

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

// use crate::task::{Context, Poll};

/// Cooperatively gives up a timeslice to the task scheduler.
///
/// Calling this function will move the currently executing future to the back
/// of the execution queue, making room for other futures to execute. This is
/// especially useful after running CPU-intensive operations inside a future.
///
/// See also [`task::spawn_blocking`].
///
/// [`task::spawn_blocking`]: fn.spawn_blocking.html
///
/// # Examples
///
/// Basic usage:
///
/// ```
/// # async_std::task::block_on(async {
/// #
/// use async_std::task;
///
/// task::yield_now().await;
/// #
/// # })
/// ```
#[inline]
pub async fn yield_now() {
    YieldNow(false).await
}

struct YieldNow(bool);

impl Future for YieldNow {
    type Output = ();

    // The futures executor is implemented as a FIFO queue, so all this future
    // does is re-schedule the future back to the end of the queue, giving room
    // for other futures to progress.
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if !self.0 {
            self.0 = true;
            cx.waker().wake_by_ref();
            Poll::Pending
        } else {
            Poll::Ready(())
        }
    }
}

/// A component's unique identifier.
///
/// `ScopeId` is a `usize` that is unique across the entire VirtualDOM - but not unique across time. If a component is
/// unmounted, then the `ScopeId` will be reused for a new component.
#[derive(serde::Serialize, serde::Deserialize, Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct ScopeId(pub usize);

/// An Element's unique identifier.
///
/// `ElementId` is a `usize` that is unique across the entire VirtualDOM - but not unique across time. If a component is
/// unmounted, then the `ElementId` will be reused for a new component.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct ElementId(pub usize);
impl Display for ElementId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl ElementId {
    pub fn as_u64(self) -> u64 {
        self.0 as u64
    }
}
