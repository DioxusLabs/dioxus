use crate::{BuildUpdate, Platform, TraceMsg};
use axum::extract::ws::Message as WsMessage;
use std::{path::PathBuf, process::ExitStatus};

use super::HandleUpdate;

/// One fat enum to rule them all....
///
/// Thanks to libraries like winit for the inspiration
#[allow(clippy::large_enum_variant)]
pub(crate) enum ServeUpdate {
    NewConnection,
    WsMessage(WsMessage),

    /// A build update from the build engine
    BuildUpdate(BuildUpdate),

    /// An update from handle to a running app,
    HandleUpdate(HandleUpdate),

    FilesChanged {
        files: Vec<PathBuf>,
    },

    /// Open an existing app bundle, if it exists
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
