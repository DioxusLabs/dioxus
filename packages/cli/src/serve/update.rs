use crate::builder::{BuildRequest, TargetPlatform, UpdateBuildProgress};
use axum::extract::ws::Message as WsMessage;
use std::process::ExitStatus;

/// One fat enum to rule them all....
///
/// Thanks to libraries like winit for the inspiration
pub enum ServeUpdate {
    NewConnection,
    Message(WsMessage),

    Progress {
        update: UpdateBuildProgress,
    },

    BuildReady {
        target: TargetPlatform,
    },

    BuildFailed {
        target: TargetPlatform,
        err: crate::Error,
    },

    ProcessExited {
        target_platform: TargetPlatform,
        status: Result<ExitStatus, std::io::Error>,
    },

    FilesChanged {},

    TuiInput {
        rebuild: bool,
    },
}
