use crate::builder::{Stage, TargetPlatform, UpdateBuildProgress, UpdateStage};
use crate::cli::serve::Serve;
use crate::dioxus_crate::DioxusCrate;
use crate::Result;
use futures_util::FutureExt;
use std::future::{poll_fn, Future, IntoFuture};
use std::task::Poll;
use tokio::task::yield_now;

// Grab the output of a future that returns an option or wait forever
pub(crate) fn next_or_pending<F, T>(f: F) -> impl Future<Output = T>
where
    F: IntoFuture<Output = Option<T>>,
{
    let pinned = f.into_future().fuse();
    let mut pinned = Box::pin(pinned);
    poll_fn(move |cx| {
        let next = pinned.as_mut().poll(cx);
        match next {
            Poll::Ready(Some(next)) => Poll::Ready(next),
            _ => Poll::Pending,
        }
    })
    .fuse()
}
