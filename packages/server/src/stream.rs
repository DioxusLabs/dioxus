use crate::{IncrementalRendererError, RenderFreshness};
use axum::{body::Body, response::IntoResponse};
use futures_channel::mpsc::Receiver;
use futures_channel::mpsc::{UnboundedReceiver, UnboundedSender};
use futures_util::{Stream, StreamExt};

pub struct StreamingResponse {
    pub freshness: RenderFreshness,
    pub receiver: UnboundedReceiver<Result<String, IncrementalRendererError>>,
    pub cancel_task: Option<tokio::task::JoinHandle<()>>,
}

impl StreamingResponse {
    pub fn new(
        receiver: UnboundedReceiver<Result<String, IncrementalRendererError>>,
        freshness: RenderFreshness,
        cancel_task: Option<tokio::task::JoinHandle<()>>,
    ) -> Self {
        Self {
            freshness,
            receiver,
            cancel_task,
        }
    }
}

impl IntoResponse for StreamingResponse {
    fn into_response(self) -> axum::response::Response {
        let freshness = self.freshness;
        let mut response = axum::response::Html::from(Body::from_stream(self)).into_response();
        freshness.write(response.headers_mut());
        response
    }
}

// When we drop the stream, we need to cancel the task that is feeding values to the stream
impl Drop for StreamingResponse {
    fn drop(&mut self) {
        if let Some(cancel_task) = self.cancel_task.take() {
            cancel_task.abort();
        }
    }
}

impl Stream for StreamingResponse {
    type Item = Result<String, IncrementalRendererError>;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        self.receiver.poll_next_unpin(cx)
    }
}
