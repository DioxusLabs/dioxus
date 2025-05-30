use std::{
    backtrace::Backtrace,
    future::Future,
    sync::{LazyLock, Mutex, OnceLock},
};

use crate::{Result, Workspace};
use dioxus_cli_telemetry::TelemetryEvent;
use dioxus_dx_wire_format::StructuredOutput;
use futures_channel::mpsc::{UnboundedReceiver, UnboundedSender};

static TELEMETRY_TX: OnceLock<UnboundedSender<TelemetryEvent>> = OnceLock::new();
static TELEMETRY_RX: OnceLock<Mutex<UnboundedReceiver<TelemetryEvent>>> = OnceLock::new();

/// The main entrypoint for the log collector.
///
/// As the app runs, we simply fire off messages into the TelemetryTx handle.
///
/// Once the session is over, or the tx is flushed manually, we then log to a file.
/// This prevents any performance issues from building up during long sesssion.
/// For `dx serve`, we asyncronously flush after full rebuilds are *completed*.
pub fn main(app: impl Future<Output = Result<StructuredOutput>>) {
    // let rt = tokio::runtime::Runtime::new().unwrap();
    // let _guard = rt.enter();

    // let res = rt.block_on(tokio::spawn(async move {}));
    manually_flush();
}

/// Manually flush the telemetry queue so not as
pub fn manually_flush() -> tokio::task::JoinHandle<()> {
    tokio::task::spawn(async move {
        let mut log_file = std::fs::File::options()
            .append(true)
            .open(Workspace::telemetry_file())
            .unwrap();

        let mut rx = TELEMETRY_RX.get().unwrap().lock().unwrap();
        while let Ok(Some(msg)) = rx.try_next() {
            _ = serde_json::to_writer(&mut log_file, &msg);
        }
    })
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
