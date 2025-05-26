use std::{
    backtrace::Backtrace,
    future::Future,
    sync::{LazyLock, Mutex, OnceLock},
};

use crate::Result;
use dioxus_cli_telemetry::TelemetryEvent;
use dioxus_dx_wire_format::StructuredOutput;
use futures_channel::mpsc::{UnboundedReceiver, UnboundedSender};

static TELEMETRY_TX: OnceLock<UnboundedSender<TelemetryEvent>> = OnceLock::new();

/// The main entrypoint for the log collector.
/// We don't collect every log from the user's session since that can fill up quickly.
/// Instead,
pub fn main(app: impl Future<Output = Result<StructuredOutput>>) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let _guard = rt.enter();

    let writer_thread = tokio::spawn(async move {
        // while let Some(msg) = msg
    });

    let res = rt.block_on(tokio::spawn(async move {}));
}

/// Set the backtrace, and then initiate a rollup upload of any pending logs.
pub(crate) fn initialize() {
    struct SavedLocation {
        file: String,
        line: u32,
        column: u32,
    }

    static BACKTRACE: OnceLock<(Backtrace, Option<SavedLocation>)> = OnceLock::new();

    // We *don't* want printing here, since it'll break the tui and log ordering.
    //
    // We *will* re-emit the panic after we've drained the tracer, so our panic hook will simply capture the panic
    // and save it.
    std::panic::set_hook(Box::new(move |panic_info| {
        _ = BACKTRACE.set((
            Backtrace::capture(),
            panic_info.location().map(|l| SavedLocation {
                file: l.file().to_string(),
                line: l.line(),
                column: l.column(),
            }),
        ));
    }));
}

pub(crate) fn log_result(result: &Result<StructuredOutput>) {}
