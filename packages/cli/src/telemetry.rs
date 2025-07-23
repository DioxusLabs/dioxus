use std::{
    backtrace::Backtrace,
    future::Future,
    io::BufReader,
    panic::AssertUnwindSafe,
    sync::{Mutex, OnceLock},
};

use crate::{CliSettings, Result, TraceSrc, Workspace};
use dioxus_cli_telemetry::{set_identity, TelemetryEvent};
use dioxus_dx_wire_format::StructuredOutput;
use futures_channel::mpsc::{UnboundedReceiver, UnboundedSender};
use futures_util::FutureExt;
use posthog_rs::ClientOptions;
use target_lexicon::Triple;

static TELEMETRY_TX: OnceLock<UnboundedSender<TelemetryEvent>> = OnceLock::new();
static TELEMETRY_RX: OnceLock<Mutex<UnboundedReceiver<TelemetryEvent>>> = OnceLock::new();

/// The main entrypoint for the log collector.
///
/// As the app runs, we simply fire off messages into the TelemetryTx handle.
///
/// Once the session is over, or the tx is flushed manually, we then log to a file.
/// This prevents any performance issues from building up during long session.
/// For `dx serve`, we asynchronously flush after full rebuilds are *completed*.
pub fn main(app: impl Future<Output = Result<StructuredOutput>>) -> Result<StructuredOutput> {
    let (tx, rx) = futures_channel::mpsc::unbounded();
    TELEMETRY_TX.set(tx).expect("Failed to set telemetry tx");
    TELEMETRY_RX
        .set(Mutex::new(rx))
        .expect("Failed to set telemetry rx");
    set_identity(
        Triple::host().to_string(),
        std::env::var("CI").is_ok(),
        crate::VERSION.to_string(),
    );

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Failed building the Runtime")
        .block_on(async move {
            check_flush_file().await;
            capture_panics();
            let result = AssertUnwindSafe(app).catch_unwind().await;
            let result = handle_panic(result);
            flush_telemetry_to_file();
            result
        })
}

pub fn send_telemetry_event(event: TelemetryEvent) {
    if CliSettings::telemetry_disabled() {
        return;
    }

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
    // If the no events exist or the first event was logged less than 7 days ago, we don't need to flush.
    {
        let Some(Ok(event)) = iter.peek() else {
            return;
        };
        let time = event.time.naive_local();
        let now = chrono::Utc::now().naive_local();
        let elapsed = now.signed_duration_since(time).num_weeks();
        if elapsed < 7 {
            return;
        }
    }

    let mut events = Vec::new();
    for event in iter.flatten() {
        let event: TelemetryEvent = event;
        let mut posthog_event =
            posthog_rs::Event::new(event.name, event.identity.session_id.to_string());
        _ = posthog_event.insert_prop("device_triple", event.identity.device_triple);
        _ = posthog_event.insert_prop("is_ci", event.identity.is_ci);
        _ = posthog_event.insert_prop("cli_version", event.identity.cli_version);
        _ = posthog_event.insert_prop("message", event.message);
        _ = posthog_event.insert_prop("module", event.module);
        _ = posthog_event.insert_prop("stage", event.stage);
        _ = posthog_event.insert_prop("timestamp", event.time);
        for (key, value) in event.values {
            _ = posthog_event.insert_prop(key, value);
        }
        events.push(posthog_event);
    }
    // Remove the file
    std::fs::remove_file(file).unwrap();
    // Send the events in the background
    tokio::spawn(async move {
        let client = posthog_rs::client(ClientOptions::from(KEY)).await;
        if let Err(error) = client.capture_batch(events).await {
            tracing::trace!(
                dx_src = ?TraceSrc::Dev,
                "Failed to send telemetry events: {}",
                error
            );
        }
    });
}

struct SavedLocation {
    file: String,
    line: u32,
    column: u32,
}
static BACKTRACE: OnceLock<(Backtrace, Option<SavedLocation>)> = OnceLock::new();

/// Set the backtrace, and then initiate a rollup upload of any pending logs.
pub(crate) fn capture_panics() {
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

fn fatal_error(error: &anyhow::Error) -> TelemetryEvent {
    let mut telemetry_event = TelemetryEvent::new("fatal_error", None, error.to_string(), "error");
    let backtrace = error.backtrace();
    if backtrace.status() == std::backtrace::BacktraceStatus::Captured {
        telemetry_event = telemetry_event.with_value("backtrace", clean_backtrace(&backtrace));
    }
    let chain = error.chain();
    telemetry_event.with_value(
        "error_chain",
        chain.map(|e| format!("{e}")).collect::<Vec<_>>(),
    )
}

fn clean_backtrace(backtrace: &Backtrace) -> String {
    let mut backtrace_display = backtrace.to_string();

    // split at the line that ends with ___rust_try for short backtraces
    if std::env::var("RUST_BACKTRACE") == Ok("1".to_string()) {
        backtrace_display = backtrace_display
            .split(" ___rust_try\n")
            .next()
            .map(|f| format!("{f} ___rust_try"))
            .unwrap_or_default();
    }

    backtrace_display
}

fn handle_panic(
    result: Result<anyhow::Result<StructuredOutput>, Box<dyn std::any::Any + Send>>,
) -> Result<StructuredOutput> {
    match result {
        Ok(Ok(_res)) => Ok(StructuredOutput::Success),
        Ok(Err(e)) => {
            // Add the fatal error to the telemetry
            send_telemetry_event(fatal_error(&e));
            Err(e)
        }
        Err(panic_err) => {
            // And then print the panic itself.
            let as_str = if let Some(p) = panic_err.downcast_ref::<String>() {
                p.as_ref()
            } else if let Some(p) = panic_err.downcast_ref::<&str>() {
                p
            } else {
                "<unknown panic>"
            };

            // Attempt to emulate the default panic hook
            let message = match BACKTRACE.get() {
                Some((back, location)) => {
                    let location_display = location
                        .as_ref()
                        .map(|l| format!("{}:{}:{}", l.file, l.line, l.column))
                        .unwrap_or_else(|| "<unknown>".to_string());

                    let backtrace_display = clean_backtrace(back);

                    // Add the fatal error to the telemetry
                    send_telemetry_event(TelemetryEvent::new(
                        "panic",
                        Some(location_display.clone()),
                        &backtrace_display,
                        "error",
                    ));

                    format!("dx serve panicked at {location_display}\n{as_str}\n{backtrace_display} ___rust_try")
                }
                None => {
                    // Add the fatal error to the telemetry
                    send_telemetry_event(TelemetryEvent::new("panic", None, as_str, "error"));
                    format!("dx serve panicked: {as_str}")
                }
            };

            Err(anyhow::anyhow!(message))
        }
    }
}
