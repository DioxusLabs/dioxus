use std::{
    backtrace::Backtrace,
    future::Future,
    io::BufReader,
    panic::AssertUnwindSafe,
    sync::{Mutex, OnceLock},
};

use crate::{CliSettings, Result, TraceSrc, Workspace};
use anyhow::Error;
use dioxus_cli_telemetry::{Reporter, TelemetryEvent};
use dioxus_dx_wire_format::StructuredOutput;
use futures_channel::mpsc::{UnboundedReceiver, UnboundedSender};
use futures_util::FutureExt;
use posthog_rs::ClientOptions;
use serde_json::Value;
use target_lexicon::Triple;
use uuid::Uuid;

/// A trait that emits an anonymous JSON representation of the object, suitable for telemetry.
pub(crate) trait Anonymized {
    fn anonymized(&self) -> Value;
}

/// The main entrypoint for the telemetry side loop.
///
/// As the app runs, we simply fire off messages into the TelemetryTx handle.
///
/// Once the session is over, or the tx is flushed manually, we then log to a file.
/// This prevents any performance issues from building up during long session.
/// For `dx serve`, we asynchronously flush after full rebuilds are *completed*.
/// Initialize a user session with a stable ID.
///
///
pub(crate) fn initializer_telemetry_session() -> Result<UnboundedSender<TelemetryEvent>> {
    let (tx, rx) = futures_channel::mpsc::unbounded();

    let sessions_folder = Workspace::dioxus_home_dir().join("stats");

    // Create the sessions folder if it doesn't exist
    if !sessions_folder.exists() {
        std::fs::create_dir_all(&sessions_folder)?;
    }

    // Get the UUID for the session, or create a new one if it doesn't exist.
    let stable_session_file = sessions_folder.join("stable_id.json");
    let reported_id = if stable_session_file.exists() {
        let contents = std::fs::read_to_string(stable_session_file)?;
        serde_json::from_str::<Uuid>(&contents)?
    } else {
        let new_id = Uuid::new_v4();
        std::fs::write(stable_session_file, serde_json::to_string(&new_id)?)?;
        new_id
    };

    // loop receive, writing to our custom file

    Ok(tx)
}

pub fn send_telemetry_event(event: TelemetryEvent) {
    if CliSettings::telemetry_disabled() {
        return;
    }

    if let Some(tx) = TELEMETRY_TX.get() {
        let _ = tx.unbounded_send(event);
    }
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

    let mut log_file = match std::fs::File::options()
        .create(true)
        .append(true)
        .open(Workspace::telemetry_pending_file())
    {
        Ok(file) => file,
        Err(err) => {
            tracing::trace!("Failed to open telemetry file: {}", err);
            return;
        }
    };

    while let Ok(Some(msg)) = rx.try_next() {
        _ = serde_json::to_writer(&mut log_file, &msg);
    }
}

const KEY: &str = "phc_d2jQTZMqAWxSkzv3NQ8TlxCP49vtBZ5ZmlYMIZLFNNU";

pub fn flush_old_telemetry() {
    let file = Workspace::telemetry_pending_file();
    let Ok(file_contents) = std::fs::File::open(&file) else {
        return;
    };

    // dioxus_cli_telemetry::set_reporter(
    //     Triple::host().to_string(),
    //     std::env::var("CI").is_ok(),
    //     crate::VERSION.to_string(),
    //     reported_id.as_u128(),
    // );

    let device_triple = Triple::host().to_string();
    let is_ci = std::env::var("CI").is_ok();
    let cli_version = crate::VERSION.to_string();

    let reporter = Reporter {
        device_triple,
        is_ci,
        cli_version,
        reporter_id: reported_id.as_u128(),
        session_id: Uuid::new_v4().as_u128(),
    };

    // If the no events exist or the first event was logged less than 7 days ago, we don't need to flush.
    let mut iter = serde_json::Deserializer::from_reader(BufReader::new(file_contents))
        .into_iter::<TelemetryEvent>()
        .peekable();

    let Some(Ok(first_event)) = iter.peek() else {
        return;
    };

    let time = first_event.time.naive_local();
    let now = chrono::Utc::now().naive_local();
    if now.signed_duration_since(time).num_weeks() < 7 {
        return;
    }

    let events = iter
        .flatten()
        .map(|event| {
            let mut ph_event =
                posthog_rs::Event::new(event.name, event.reporter.session_id.to_string());

            _ = ph_event.insert_prop("device_triple", event.reporter.device_triple);
            _ = ph_event.insert_prop("is_ci", event.reporter.is_ci);
            _ = ph_event.insert_prop("cli_version", event.reporter.cli_version);
            _ = ph_event.insert_prop("message", event.message);
            _ = ph_event.insert_prop("module", event.module);
            _ = ph_event.insert_prop("stage", event.stage);
            _ = ph_event.insert_prop("timestamp", event.time);

            for (key, value) in event.values {
                _ = ph_event.insert_prop(key, value);
            }

            ph_event
        })
        .collect::<Vec<_>>();

    // Send the events in the background
    tokio::spawn(async move {
        let res = posthog_rs::client(ClientOptions::from(KEY))
            .await
            .capture_batch(events)
            .await
            .inspect_err(|error| {
                tracing::trace!(dx_src = ?TraceSrc::Dev, "Failed to send telemetry events: {}", error)
            });

        if res.is_ok() {
            _ = std::fs::remove_file(file)
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

pub fn fatal_error(error: &Error) -> TelemetryEvent {
    let mut telemetry_event = TelemetryEvent::new("fatal_error", None, error.to_string(), "error");
    let backtrace = error.backtrace();
    if backtrace.status() == std::backtrace::BacktraceStatus::Captured {
        telemetry_event = telemetry_event.with_value("backtrace", clean_backtrace(backtrace));
    }
    let chain = error.chain();
    telemetry_event.with_value(
        "error_chain",
        chain.map(|e| format!("{e}")).collect::<Vec<_>>(),
    )
}

pub fn clean_backtrace(backtrace: &Backtrace) -> String {
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
