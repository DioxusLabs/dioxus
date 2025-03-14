use dioxus_core::prelude::try_consume_context;
use dioxus_signals::{Readable, Signal, Writable};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum StreamingStatus {
    RenderingInitialChunk,
    InitialChunkCommitted,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StreamingContext {
    current_status: Signal<StreamingStatus>,
}

impl StreamingContext {
    pub fn new() -> Self {
        Self {
            current_status: Signal::new(StreamingStatus::RenderingInitialChunk),
        }
    }

    pub fn commit_initial_chunk(&mut self) {
        self.current_status
            .set(StreamingStatus::InitialChunkCommitted);
    }

    pub fn current_status(&self) -> StreamingStatus {
        *self.current_status.read()
    }
}

pub fn commit_initial_chunk() {
    if let Some(mut streaming) = try_consume_context::<StreamingContext>() {
        streaming.commit_initial_chunk();
    }
}
