use std::{
    backtrace::Backtrace,
    future::Future,
    io::BufReader,
    sync::{Mutex, OnceLock},
};

use crate::{Result, Workspace};
use dioxus_cli_telemetry::TelemetryEvent;
use dioxus_dx_wire_format::StructuredOutput;
use futures_channel::mpsc::{UnboundedReceiver, UnboundedSender};
use posthog_rs::ClientOptions;

static TELEMETRY_TX: OnceLock<UnboundedSender<TelemetryEvent>> = OnceLock::new();
static TELEMETRY_RX: OnceLock<Mutex<UnboundedReceiver<TelemetryEvent>>> = OnceLock::new();

/// The main entrypoint for the log collector.
///
/// As the app runs, we simply fire off messages into the TelemetryTx handle.
///
/// Once the session is over, or the tx is flushed manually, we then log to a file.
/// This prevents any performance issues from building up during long session.
/// For `dx serve`, we asynchronously flush after full rebuilds are *completed*.
pub fn main(app: impl Future<Output = Result<StructuredOutput>>) {
    let (mut tx, rx) = futures_channel::mpsc::unbounded();
    TELEMETRY_TX.set(tx).expect("Failed to set telemetry tx");
    TELEMETRY_RX
        .set(Mutex::new(rx))
        .expect("Failed to set telemetry rx");
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Failed building the Runtime")
        .block_on(async move {
            check_flush_file().await;
            let result = app.await;
            flush_telemetry_to_file();
        });
}

pub fn send_telemetry_event(event: TelemetryEvent) {
    let Some(tx) = TELEMETRY_TX.get() else {
        tracing::warn!("Telemetry TX is not set, cannot send telemetry.");
        return;
    };
    let _ = tx.unbounded_send(event);
}

/// Manually flush the telemetry queue so not as
pub fn flush_telemetry_to_file() {
    let Some(rx) = TELEMETRY_RX.get() else {
        tracing::warn!("Telemetry RX is not set, cannot flush telemetry.");
        return;
    };
    let Ok(mut rx) = rx.lock() else {
        tracing::warn!("Failed to lock telemetry RX");
        return;
    };
    let mut log_file = std::fs::File::options()
        .create(true)
        .append(true)
        .open(Workspace::telemetry_file())
        .unwrap();

    while let Ok(Some(msg)) = rx.try_next() {
        _ = serde_json::to_writer(&mut log_file, &msg);
    }
}

const KEY: &str = "phc_OTBMYjklqT5Dw4EKWGFrKy2jFOV1jd4MmiSe96TKjLz";

async fn check_flush_file() {
    let file = Workspace::telemetry_file();
    let Ok(file_contents) = std::fs::File::open(&file) else {
        return;
    };
    let mut iter = serde_json::Deserializer::from_reader(BufReader::new(file_contents))
        .into_iter::<TelemetryEvent>()
        .peekable();
    // If the no events exist or the first event was logged less than 30 seconds ago, we don't need to flush.
    {
        let Some(Ok(event)) = iter.peek() else {
            return;
        };
        let time = event.time.naive_local();
        let now = chrono::Utc::now().naive_local();
        let elapsed = now.signed_duration_since(time).num_seconds();
        println!("Telemetry file is {} seconds old", elapsed);
        if elapsed < 30 {
            println!("Telemetry file is recent, skipping flush.");
            return;
        }
    }

    let mut events = Vec::new();
    for event in iter.flatten() {
        let event: TelemetryEvent = event;
        let mut posthog_event = posthog_rs::Event::new_anon(event.name);
        _ = posthog_event.insert_prop("message", event.message);
        _ = posthog_event.insert_prop("stage", event.stage);
        _ = posthog_event.insert_prop("timestamp", event.time);
        for (key, value) in event.values {
            _ = posthog_event.insert_prop(key, value);
        }
        events.push(posthog_event);
    }
    let client = posthog_rs::client(ClientOptions::from(KEY)).await;
    _ = client.capture_batch(events).await;
    // Remove the file
    std::fs::remove_file(file).unwrap();
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
