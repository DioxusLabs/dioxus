use dioxus_core::try_consume_context;
use dioxus_signals::{ReadableExt, Signal, WritableExt};

/// The status of the streaming response
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum StreamingStatus {
    /// The initial chunk is still being rendered. The http response parts can still be modified at this point.
    RenderingInitialChunk,

    /// The initial chunk has been committed and the response is now streaming. The http response parts
    /// have already been sent to the client and can no longer be modified.
    InitialChunkCommitted,
}

/// The context dioxus fullstack provides for the status of streaming responses on the server
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StreamingContext {
    current_status: Signal<StreamingStatus>,
}

impl Default for StreamingContext {
    fn default() -> Self {
        Self::new()
    }
}

impl StreamingContext {
    /// Create a new streaming context. You should not need to call this directly. Dioxus fullstack will
    /// provide this context for you.
    pub fn new() -> Self {
        Self {
            current_status: Signal::new(StreamingStatus::RenderingInitialChunk),
        }
    }

    /// Commit the initial chunk of the response. This will be called automatically if you are using the
    /// dioxus router when the suspense boundary above the router is resolved. Otherwise, you will need
    /// to call this manually to start the streaming part of the response.
    ///
    /// Once this method has been called, the http response parts can no longer be modified.
    pub fn commit_initial_chunk(&mut self) {
        self.current_status
            .set(StreamingStatus::InitialChunkCommitted);
    }

    /// Get the current status of the streaming response. This method is reactive and will cause
    /// the current reactive context to rerun when the status changes.
    pub fn current_status(&self) -> StreamingStatus {
        *self.current_status.read()
    }
}

/// Commit the initial chunk of the response. This will be called automatically if you are using the
/// dioxus router when the suspense boundary above the router is resolved. Otherwise, you will need
/// to call this manually to start the streaming part of the response.
///
/// On the client, this will do nothing.
///
/// # Example
/// ```rust, no_run
/// # use dioxus::prelude::*;
/// # use dioxus_fullstack_core::*;
/// # fn Children() -> Element { unimplemented!() }
/// fn App() -> Element {
///     // This will start streaming immediately after the current render is complete.
///     use_hook(commit_initial_chunk);
///
///     rsx! { Children {} }
/// }
/// ```
pub fn commit_initial_chunk() {
    crate::history::finalize_route();
    if let Some(mut streaming) = try_consume_context::<StreamingContext>() {
        streaming.commit_initial_chunk();
    }
}

/// Get the current status of the streaming response. This method is reactive and will cause
/// the current reactive context to rerun when the status changes.
///
/// On the client, this will always return `StreamingStatus::InitialChunkCommitted`.
///
/// # Example
/// ```rust, no_run
/// # use dioxus::prelude::*;
/// # use dioxus_fullstack_core::*;
/// #[component]
/// fn MetaTitle(title: String) -> Element {
///     // If streaming has already started, warn the user that the meta tag will not show
///     // up in the initial chunk.
///     use_hook(|| {
///         if current_status() == StreamingStatus::InitialChunkCommitted {
///            dioxus::logger::tracing::warn!("Since `MetaTitle` was rendered after the initial chunk was committed, the meta tag will not show up in the head without javascript enabled.");
///         }
///     });
///
///     rsx! { meta { property: "og:title", content: title } }
/// }
/// ```
pub fn current_status() -> StreamingStatus {
    if let Some(streaming) = try_consume_context::<StreamingContext>() {
        streaming.current_status()
    } else {
        StreamingStatus::InitialChunkCommitted
    }
}
