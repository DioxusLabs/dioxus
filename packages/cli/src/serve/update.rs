use crate::{BuildId, BuilderUpdate, Platform, TraceMsg};
use axum::extract::ws::Message as WsMessage;
use std::path::PathBuf;

/// One fat enum to rule them all....
///
/// Thanks to libraries like winit for the inspiration
#[allow(clippy::large_enum_variant)]
pub(crate) enum ServeUpdate {
    NewConnection {
        id: BuildId,
        aslr_reference: Option<u64>,
    },
    WsMessage {
        platform: Platform,
        msg: WsMessage,
    },

    /// An update regarding the state of the build and running app from an AppBuilder
    BuilderUpdate {
        id: BuildId,
        update: BuilderUpdate,
    },

    FilesChanged {
        files: Vec<PathBuf>,
    },

    OpenApp,

    RequestRebuild,

    ToggleShouldRebuild,

    Redraw,

    TracingLog {
        log: TraceMsg,
    },

    Exit {
        error: Option<Box<dyn std::error::Error + Send + Sync>>,
    },
}
