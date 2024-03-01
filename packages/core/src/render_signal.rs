use std::rc::Rc;
use std::task::Waker;
use std::task::Poll;
use std::pin::Pin;
use std::future::Future;
use std::task::Context;
use std::cell::RefCell;

/// A signal is a message that can be sent to all listening tasks at once
#[derive(Default)]
pub struct RenderSignal {
    wakers: Rc<RefCell<Vec<Rc<RefCell<RenderSignalFutureInner>>>>>,
}

impl RenderSignal {
    /// Send the signal to all listening tasks
    pub fn send(&self) {
        let mut wakers = self.wakers.borrow_mut();
        for waker in wakers.drain(..) {
            let mut inner = waker.borrow_mut();
            inner.resolved = true;
            if let Some(waker) = inner.waker.take() {
                waker.wake();
            }
        }
    }

    /// Create a future that resolves when the signal is sent
    pub fn subscribe(& self) -> RenderSignalFuture {
        let inner =Rc::new(RefCell::new(RenderSignalFutureInner {
            resolved: false,
            waker: None,
        }));
        self.wakers.borrow_mut().push(inner.clone());
        let waker = RenderSignalFuture {
            inner
        };
        waker
    }
}

struct RenderSignalFutureInner {
    resolved: bool,
    waker: Option<Waker>,
}

pub(crate) struct RenderSignalFuture {
    inner: Rc<RefCell<RenderSignalFutureInner>>,
}

impl Future for RenderSignalFuture {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        let mut inner = self.inner.borrow_mut();
        if inner.resolved {
            Poll::Ready(())
        } else {
            inner.waker = Some(cx.waker().clone());
            Poll::Pending
        }
    }
}
