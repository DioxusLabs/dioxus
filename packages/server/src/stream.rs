use crate::{IncrementalRendererError, RenderFreshness};
use crate::{RenderChunk, Result};
use axum::{body::Body, response::IntoResponse};
use futures_channel::mpsc::Receiver;
use futures_channel::mpsc::{UnboundedReceiver, UnboundedSender};
use futures_util::{Stream, StreamExt};

pub struct StreamingResponse {
    pub sender: UnboundedSender<Result<RenderChunk>>,
    pub receiver: UnboundedReceiver<Result<RenderChunk>>,
    // pub freshness: RenderFreshness,
    // pub cancel_task: Option<tokio::task::JoinHandle<()>>,
}

impl StreamingResponse {
    pub fn new(// freshness: RenderFreshness,
        // cancel_task: Option<tokio::task::JoinHandle<()>>,
    ) -> Self {
        let (sender, receiver) = futures_channel::mpsc::unbounded();
        Self {
            // freshness,
            sender,
            receiver,
            // cancel_task,
        }
    }

    pub fn tx(&self) -> UnboundedSender<Result<RenderChunk>> {
        self.sender.clone()
    }

    pub async fn next(&mut self) -> Option<Result<RenderChunk>> {
        self.receiver.next().await
    }
}

impl IntoResponse for StreamingResponse {
    fn into_response(self) -> axum::response::Response {
        // let freshness = self.freshness;
        let mut response = axum::response::Html::from(Body::from_stream(self)).into_response();
        // freshness.write(response.headers_mut());
        response
    }
}

impl Stream for StreamingResponse {
    type Item = Result<RenderChunk>;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        self.receiver.poll_next_unpin(cx)
    }
}

// When we drop the stream, we need to cancel the task that is feeding values to the stream
impl Drop for StreamingResponse {
    fn drop(&mut self) {
        // if let Some(cancel_task) = self.cancel_task.take() {
        //     cancel_task.abort();
        // }
    }
}
