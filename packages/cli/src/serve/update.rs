use crate::{BuildUpdate, Platform, TraceMsg};
use axum::extract::ws::Message as WsMessage;
use std::{path::PathBuf, process::ExitStatus};

/// One fat enum to rule them all....
///
/// Thanks to libraries like winit for the inspiration
#[allow(clippy::large_enum_variant)]
pub(crate) enum ServeUpdate {
    NewConnection,
    WsMessage(WsMessage),

    /// A build update from the build engine
    BuildUpdate(BuildUpdate),

    /// A running process has received a stdout.
    /// May or may not be a complete line - do not treat it as a line. It will include a line if it is a complete line.
    ///
    /// We will poll lines and any content in a 50ms interval
    StdoutReceived {
        platform: Platform,
        msg: String,
    },

    /// A running process has received a stderr.
    /// May or may not be a complete line - do not treat it as a line. It will include a line if it is a complete line.
    ///
    /// We will poll lines and any content in a 50ms interval
    StderrReceived {
        platform: Platform,
        msg: String,
    },

    ProcessExited {
        platform: Platform,
        status: ExitStatus,
    },

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
