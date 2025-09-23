use dioxus_core::use_hook;
use dioxus_signals::{ReadableExt, Signal, WritableExt};
use futures_channel::oneshot::{Canceled, Receiver, Sender};
use futures_util::{future::Shared, FutureExt};

/// A hook that provides a waker for other hooks to provide async/await capabilities.
pub fn use_waker<T: Clone + 'static>() -> UseWaker<T> {
    // We use a oneshot channel to send the value to the awaiter.
    // The shared future allows multiple awaiters to wait on the same value.
    let (task_tx, task_rx) = use_hook(|| {
        let (tx, rx) = futures_channel::oneshot::channel::<T>();
        let shared = rx.shared();
        (Signal::new(tx), Signal::new(shared))
    });

    UseWaker { task_tx, task_rx }
}

#[derive(Debug)]
pub struct UseWaker<T: 'static> {
    task_tx: Signal<Sender<T>>,
    task_rx: Signal<Shared<Receiver<T>>>,
}

impl<T: Clone + 'static> UseWaker<T> {
    /// Wake the current task with the provided value.
    /// All awaiters will receive a clone of the value.
    pub fn wake(&mut self, value: T) {
        // We ignore the error because it means the task has already been woken.
        let (tx, rx) = futures_channel::oneshot::channel::<T>();
        let shared = rx.shared();

        let tx = self.task_tx.replace(tx);
        let _rx = self.task_rx.replace(shared);

        let _ = tx.send(value);
    }

    /// Returns a future that resolves when the task is woken.
    pub async fn wait(&self) -> Result<T, Canceled> {
        self.task_rx.cloned().await
    }
}

// you can await it directly!
impl<T: Clone + 'static> std::future::Future for UseWaker<T> {
    type Output = Result<T, Canceled>;

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        self.task_rx.peek().clone().poll_unpin(cx)
    }
}

impl<T> Copy for UseWaker<T> {}
impl<T> Clone for UseWaker<T> {
    fn clone(&self) -> Self {
        *self
    }
}
