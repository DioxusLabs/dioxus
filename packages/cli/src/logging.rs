//! CLI Tracing
//!
//! The CLI's tracing has internal and user-facing logs. User-facing logs are directly routed to the user in some form.
//! Internal logs are stored in a log file for consumption in bug reports and debugging.
//! We use tracing fields to determine whether a log is internal or external and additionally if the log should be
//! formatted or not.
//!
//! These two fields are
//! `dx_src` which tells the logger that this is a user-facing message and should be routed as so.
//! `dx_no_fmt`which tells the logger to avoid formatting the log and to print it as-is.
//!
//! 1. Build general filter
//! 2. Build file append layer for logging to a file. This file is reset on every CLI-run.
//! 3. Build CLI layer for routing tracing logs to the TUI.
//! 4. Build fmt layer for non-interactive logging with a custom writer that prevents output during interactive mode.
//!
//! ## Telemetry
//!
//! The CLI collects anonymized telemetry data to help us understand how the CLI is used. We primarily
//! care about catching panics and fatal errors. Data is uploaded to PostHog through a custom proxy endpoint.
//!
//! Telemetry events are collected while the CLI is running and then flushed to disk at the end of the session.
//! When the CLI starts again, it tries its best to upload the telemetry data from the FS. In CI,
//! the telemetry data is uploaded immediately after the CLI completes with a 5 second timeout.
//!
//! You can opt out in a number of ways:
//! - set TELEMETRY=false in your environment
//! - set DX_TELEMETRY_ENABLED=false in your environment
//! - set `dx config set disable-telemetry true`
//!

use crate::component::ComponentCommand;
use crate::{dx_build_info::GIT_COMMIT_HASH_SHORT, serve::ServeUpdate, Cli, Commands, Verbosity};
use crate::{BundleFormat, CliSettings, Workspace};
use anyhow::{bail, Context, Error, Result};
use cargo_metadata::diagnostic::{Diagnostic, DiagnosticLevel};
use clap::Parser;
use dioxus_cli_telemetry::TelemetryEventData;
use dioxus_dx_wire_format::StructuredOutput;
use futures_channel::mpsc::{UnboundedReceiver, UnboundedSender};
use futures_util::FutureExt;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{any::Any, io::Read, str::FromStr, sync::Arc, time::SystemTime};
use std::{borrow::Cow, sync::OnceLock};
use std::{
    collections::HashMap,
    env,
    fmt::{Debug, Display, Write as _},
    sync::atomic::{AtomicBool, Ordering},
    time::Instant,
};
use std::{future::Future, panic::AssertUnwindSafe};
use tracing::{field::Visit, Level, Subscriber};
use tracing_subscriber::{
    fmt::{
        format::{self, Writer},
        time::FormatTime,
    },
    prelude::*,
    registry::LookupSpan,
    EnvFilter, Layer,
};
use uuid::Uuid;

const LOG_ENV: &str = "DIOXUS_LOG";
const DX_SRC_FLAG: &str = "dx_src";

pub static VERBOSITY: OnceLock<Verbosity> = OnceLock::new();

pub fn verbosity_or_default() -> Verbosity {
    crate::VERBOSITY.get().cloned().unwrap_or_default()
}

fn reset_cursor() {
    use std::io::IsTerminal;

    // if we are running in a terminal, reset the cursor. The tui_active flag is not set for
    // the cargo generate TUI, but we still want to reset the cursor.
    if std::io::stdout().is_terminal() {
        _ = console::Term::stdout().show_cursor();
    }
}

/// A trait that emits an anonymous JSON representation of the object, suitable for telemetry.
pub(crate) trait Anonymized {
    fn anonymized(&self) -> serde_json::Value;
}

/// A custom layer that wraps our special interception logic based on the mode of the CLI and its verbosity.
///
/// Redirects TUI logs, writes to files, and queues telemetry events.
///
/// It is cloned and passed directly as a layer to the tracing subscriber.
#[derive(Clone)]
pub struct TraceController {
    http_client: Option<reqwest::Client>,
    reporter: Option<Reporter>,
    telemetry_tx: UnboundedSender<TelemetryEventData>,
    telemetry_rx: Arc<tokio::sync::Mutex<UnboundedReceiver<TelemetryEventData>>>,
    log_to_file: Option<Arc<tokio::sync::Mutex<std::fs::File>>>,
    tui_active: Arc<AtomicBool>,
    tui_tx: UnboundedSender<TraceMsg>,
    tui_rx: Arc<tokio::sync::Mutex<UnboundedReceiver<TraceMsg>>>,
}

/// An error that contains information about a captured panic, including the error, thread name, and location.
/// This is passed through to the tracing subscriber which is downcasted into a `CapturedPanicError`
#[derive(Debug, Clone)]
struct CapturedPanicError {
    error: Arc<Error>,
    error_type: String,
    thread_name: Option<String>,
    location: Option<SavedLocation>,
}
impl std::fmt::Display for CapturedPanicError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "CapturedBacktrace {{ error: {}, error_type: {}, thread_name: {:?}, location: {:?} }}",
            self.error, self.error_type, self.thread_name, self.location
        )
    }
}
impl std::error::Error for CapturedPanicError {}

#[derive(Debug, Clone)]
struct SavedLocation {
    file: String,
    line: u32,
    column: u32,
}

impl TraceController {
    /// Initialize the CLI and set up the tracing infrastructure
    ///
    /// This captures panics and flushes telemetry to a file after the CLI has run.
    ///
    /// We pass the TraceController around the CLI in a few places, namely the serve command so the TUI
    /// can access things like the logs.
    pub async fn main<F>(run_app: impl FnOnce(Commands, Self) -> F) -> StructuredOutput
    where
        F: Future<Output = Result<StructuredOutput>>,
    {
        let args = Cli::parse();
        let tui_active = Arc::new(AtomicBool::new(false));
        let is_serve_cmd = matches!(args.action, Commands::Serve(_));

        VERBOSITY
            .set(args.verbosity.clone())
            .expect("verbosity should only be set once");

        // Set up a basic env-based filter for the logs
        let env_filter = match env::var(LOG_ENV) {
            Ok(_) => EnvFilter::from_env(LOG_ENV),
            _ if is_serve_cmd => EnvFilter::new("error,dx=trace,dioxus_cli=trace,manganis_cli_support=trace,wasm_split_cli=trace,subsecond_cli_support=trace"),
            _ => EnvFilter::new(format!(
                "error,dx={our_level},dioxus_cli={our_level},manganis_cli_support={our_level},wasm_split_cli={our_level},subsecond_cli_support={our_level}",
                our_level = if args.verbosity.verbose { "debug" } else { "info" }
            ))
        };

        // Listen to a few more tokio events if the tokio-console feature is enabled
        #[cfg(feature = "tokio-console")]
        let env_filter = env_filter
            .add_directive("tokio=trace".parse().unwrap())
            .add_directive("runtime=trace".parse().unwrap());

        // Set up the json filter which lets through JSON traces only if the `json` field is present in the trace metadata
        // If the json is disabled, we filter it completely so it doesn't show up in the logs
        let json_filter = tracing_subscriber::filter::filter_fn(move |meta| {
            if meta.fields().len() == 1 && meta.fields().iter().next().unwrap().name() == "json" {
                return args.verbosity.json_output;
            }
            true
        });

        // We complete filter out a few fields that are not relevant to the user, like `dx_src` and `json`
        let fmt_layer = tracing_subscriber::fmt::layer()
            .with_target(args.verbosity.verbose)
            .fmt_fields(
                format::debug_fn(move |writer, field, value| {
                    if field.name() == "json" && !args.verbosity.json_output {
                        return Ok(());
                    }

                    if field.name() == "dx_src" && !args.verbosity.verbose {
                        return Ok(());
                    }

                    if field.name() == "telemetry" {
                        return Ok(());
                    }

                    write!(writer, "{}", format_field(field.name(), value))
                })
                .delimited(" "),
            )
            .with_timer(PrettyUptime::default());

        // If json output is enabled, we want to format the output as JSON
        // When running in interactive mode (of which serve is the only one), we don't want to log to console directly
        let fmt_layer = if args.verbosity.json_output {
            fmt_layer.json().flatten_event(true).boxed()
        } else {
            fmt_layer.boxed()
        }
        .with_filter(tracing_subscriber::filter::filter_fn({
            let tui_active = tui_active.clone();
            move |re| {
                // If the TUI is active, we don't want to log to the console directly
                if tui_active.load(Ordering::Relaxed) {
                    return false;
                }

                // If the TUI is disabled, then we might be draining the logs during an error report.
                // In this case, we only show debug logs if the verbosity is set to verbose.
                //
                // If we're not running the serve command at all, this isn't relevant, so let the log through.
                if !is_serve_cmd {
                    return true;
                }

                let verbosity = VERBOSITY.get().unwrap();

                // If the verbosity is trace, let through trace logs (most verbose)
                if verbosity.trace {
                    return re.level() <= &Level::TRACE;
                }

                // if the verbosity is verbose, let through debug logs
                if verbosity.verbose {
                    return re.level() <= &Level::DEBUG;
                }

                // Otherwise, only let through info and higher level logs
                re.level() <= &Level::INFO
            }
        }));

        // Set up the tokio console subscriber if enabled
        #[cfg(feature = "tokio-console")]
        let console_layer = console_subscriber::spawn();
        #[cfg(not(feature = "tokio-console"))]
        let console_layer = tracing_subscriber::layer::Identity::new();

        // Construct our custom layer that handles the TUI and file logging
        let log_to_file = args
            .verbosity
            .log_to_file
            .as_deref()
            .map(|file_path| {
                std::fs::OpenOptions::new()
                    .append(true)
                    .create(true)
                    .open(file_path)
                    .map(|file| Arc::new(tokio::sync::Mutex::new(file)))
            })
            .transpose()
            .context("Failed to open specified log_file for writing")
            .unwrap();

        // Create a new session ID for this invocation of the CLI
        let reporter = Self::enroll_reporter().ok();

        // Create a new telemetry uploader
        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .ok();

        // Create a new telemetry channel
        // Note that we only drain the channel at the end of the CLI run, so it's not really being used as a channel - more of a vecdeque
        let (telemetry_tx, telemetry_rx) = futures_channel::mpsc::unbounded();
        let (tui_tx, tui_rx) = futures_channel::mpsc::unbounded();
        let tracer = TraceController {
            reporter,
            telemetry_tx,
            log_to_file,
            tui_tx,
            tui_rx: Arc::new(tokio::sync::Mutex::new(tui_rx)),
            telemetry_rx: Arc::new(tokio::sync::Mutex::new(telemetry_rx)),
            http_client,
            tui_active,
        };

        // Spawn the telemetry uploader in the background
        tokio::spawn(
            tracer
                .clone()
                .upload_telemetry_files(Self::to_invoked_event(&args.action)),
        );

        // Construct the tracing subscriber
        tracing_subscriber::registry()
            .with(env_filter)
            .with(json_filter)
            .with(tracer.clone())
            .with(console_layer)
            .with(fmt_layer)
            .init();

        // Set the panic handler to capture backtraces in case of a panic
        // let initial_thread = std::thread::current();
        std::panic::set_hook(Box::new(move |panic_info| {
            let payload = if let Some(s) = panic_info.payload().downcast_ref::<String>() {
                s.to_string()
            } else if let Some(s) = panic_info.payload().downcast_ref::<&str>() {
                s.to_string()
            } else {
                "<unknown panic>".to_string()
            };

            let current_thread = std::thread::current();
            let thread_name = current_thread.name().map(|s| s.to_string());
            let location = panic_info.location().unwrap();
            let err = anyhow::anyhow!(payload);
            let err_display = format!("{:?}", err)
                .lines()
                .take_while(|line| !line.ends_with("___rust_try"))
                .join("\n");

            let boxed_panic: Box<dyn std::error::Error + Send + 'static> =
                Box::new(CapturedPanicError {
                    error: Arc::new(err),
                    thread_name: thread_name.clone(),
                    error_type: "Rust Panic".to_string(),
                    location: panic_info.location().map(|l| SavedLocation {
                        file: l.file().to_string(),
                        line: l.line(),
                        column: l.column(),
                    }),
                });

            tracing::error!(
                telemetry = %json!({ "event": "Captured Panic" }),
                backtrace = boxed_panic,
                "Thread {} panicked at {location}:\n\n               {err_display}",
                thread_name.as_deref().unwrap_or("<unknown>"),
            );
        }));

        // Run the app, catching panics and errors, early flushing if `ctrl_c` is pressed.
        let app_res = AssertUnwindSafe(run_with_ctrl_c(run_app(args.action, tracer.clone())))
            .catch_unwind()
            .await;

        // Do any final logging cleanup
        tracer.finish(app_res).await
    }

    // Redirects the tracing logs to the TUI if it's active, otherwise it just collects them.
    pub fn redirect_to_tui(&self) {
        self.tui_active.store(true, Ordering::Relaxed);
    }

    /// Wait for the internal logger to send a message
    pub(crate) async fn wait(&self) -> ServeUpdate {
        use futures_util::StreamExt;

        let Some(log) = self.tui_rx.lock().await.next().await else {
            return std::future::pending().await;
        };

        ServeUpdate::TracingLog { log }
    }

    /// Uploads telemetry logs from the filesystem to the telemetry endpoint.
    ///
    /// As the app runs, we simply fire off messages into the TelemetryTx handle.
    ///
    /// Once the session is over, or the tx is flushed manually, we then log to a file.
    /// This prevents any performance issues from building up during long session.
    /// For `dx serve`, we asynchronously flush after full rebuilds are *completed*.
    /// Initialize a user session with a stable ID.
    ///
    /// This also sends a heartbeat event to the telemetry endpoint to indicate that the CLI is alive.
    ///
    /// Docs on how to send posthog:
    ///    <https://posthog.com/docs/api/capture>
    ///
    /// We try to send batched requests *without* the api key in the header. It's usually fine to send
    /// the API key along with the request, but we want to control revoking key on the backend.
    ///
    /// Todo: we should accept some sort of configuration from posthog to allow us to downsample telemetry events.
    ///       otherwise we might end up being flooded by telemetry events.
    ///
    /// We loop receive messages, pushing them into a batch.
    async fn upload_telemetry_files(self, invoked_event: TelemetryEventData) -> Result<()> {
        use fs2::FileExt;

        // Wait a little bit to prevent abuse (spam loops) and not do extra work if it's a simple `--help` call
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        // Send off a heartbeat request. If this fails, we skip anything else.
        self.send_invoked_event(invoked_event).await?;

        // Wait a few seconds to see if we can end up in `dx serve` or a long-running task
        // If we're in CI though, we do want to flush telemetry immediately
        if !CliSettings::is_ci() {
            tokio::time::sleep(std::time::Duration::from_millis(3000)).await;
        }

        // Now start loading telemetry files, locking them, and then uploading them.
        let stats_dir = Workspace::dioxus_data_dir().join("stats").join("sessions");
        for entry in stats_dir.read_dir()?.flatten() {
            // Try to open the file...
            let Ok(mut file) = std::fs::File::open(entry.path()) else {
                continue;
            };

            // And then we hold an exclusive lock on the file while we upload it
            // This prevents multiple processes from trying to upload the same file at the same time which would cause duplicate uploads
            if file.try_lock_exclusive().is_err() {
                continue;
            };

            // Now that we have the lock, we can read the file and upload it
            // todo: validate that _bytes_read is not greater than 20mb - this will fail to upload
            let mut jsonl_file = String::new();
            let Ok(_bytes_read) = file.read_to_string(&mut jsonl_file) else {
                continue;
            };

            // If the file is empty, we delete it and continue
            if jsonl_file.trim().is_empty() {
                _ = std::fs::remove_file(entry.path());
                continue;
            }

            // We assume since this is a jsonl file that every line is valid json. We just concat the lines together
            // and then send them using the batched client.
            // The session ID is the file stem of the file, which is a UUID. If not, we just make up a
            // new session ID on the fly.
            let reporter = self.reporter.as_ref().context("no reporter")?;
            let session_id = entry
                .path()
                .file_stem()
                .and_then(|s| Uuid::from_str(s.to_str()?).ok())
                .unwrap_or_else(Uuid::new_v4);
            let request_body = jsonl_file
                .lines()
                .map(serde_json::from_str::<TelemetryEventData>)
                .filter_map(Result::ok)
                .map(|event| Self::telemetry_to_posthog(session_id, reporter.distinct_id, event))
                .collect::<Vec<_>>();

            // Send the request
            // - If the request fails, we just log the error and continue
            // - If the request succeeds, we remove the file
            match self.upload_to_posthog(&request_body).await {
                Ok(()) => _ = std::fs::remove_file(entry.path()),
                Err(err) => {
                    tracing::trace!(
                        "Failed to upload telemetry file: {} because {}",
                        entry.path().display(),
                        err
                    );
                }
            }
        }

        Ok(())
    }

    /// Uploads a set of telemetry events to the PostHog endpoint.
    async fn upload_to_posthog(&self, body: &Vec<serde_json::Value>) -> Result<()> {
        use hyper::header::CONTENT_TYPE;

        let reporter = self.reporter.as_ref().context("No reporter initialized")?;
        let res = self
            .http_client
            .as_ref()
            .context("HTTP client not initialized")?
            .post(Self::posthog_capture_endpoint())
            .header(CONTENT_TYPE, "application/json")
            .header("X-Reporter-ID", reporter.distinct_id.to_string())
            .json(&body)
            .send()
            .await
            .context("Failed to send telemetry data")?;

        if !res.status().is_success() {
            bail!(
                "Failed to upload telemetry event: {:?}. Response: {:?}",
                res.status(),
                res.text().await
            );
        }

        Ok(())
    }

    async fn send_invoked_event(&self, heartbeat: TelemetryEventData) -> Result<()> {
        let reporter = self.reporter.as_ref().context("No reporter initialized")?;
        let body = Self::telemetry_to_posthog(reporter.session_id, reporter.distinct_id, heartbeat);
        self.upload_to_posthog(&vec![body]).await
    }

    /// Convert the dioxus-cli-telemetry event into a posthog event.
    ///
    /// We try to maintain the same structure for each telemetry event to do advanced filtering on the backend.
    fn telemetry_to_posthog(
        session_id: Uuid,
        distinct_id: Uuid,
        event: TelemetryEventData,
    ) -> serde_json::Value {
        let TelemetryEventData {
            action,
            message,
            time,
            values,
            command,
            stack_frames,
            file,
            line,
            column,
            module,
            error_type,
            error_handled,
        } = event;

        let mut ph_event = posthog_rs::Event::new(action, distinct_id.to_string());

        // The reporter's fields
        _ = ph_event.insert_prop("is_ci", CliSettings::is_ci());
        _ = ph_event.insert_prop("session_id", session_id.to_string());
        _ = ph_event.insert_prop("distinct_id", distinct_id.to_string());
        _ = ph_event.insert_prop("host_os", target_lexicon::HOST.operating_system.to_string());
        _ = ph_event.insert_prop("host_arch", target_lexicon::HOST.architecture.to_string());
        _ = ph_event.insert_prop("host_triple", target_lexicon::Triple::host().to_string());
        _ = ph_event.insert_prop("cli_version_major", crate::dx_build_info::PKG_VERSION_MAJOR);
        _ = ph_event.insert_prop("cli_version_minor", crate::dx_build_info::PKG_VERSION_MINOR);
        _ = ph_event.insert_prop("cli_version_patch", crate::dx_build_info::PKG_VERSION_PATCH);
        _ = ph_event.insert_prop("cli_version_pre", crate::dx_build_info::PKG_VERSION_PRE);
        _ = ph_event.insert_prop("cli_commit_hash", GIT_COMMIT_HASH_SHORT.unwrap_or_default());
        _ = ph_event.insert_prop("cli_source_file", line);
        _ = ph_event.insert_prop("cli_source_line", file);
        _ = ph_event.insert_prop("cli_source_column", column);
        _ = ph_event.insert_prop("cli_source_module", module);

        // And the TelemetryEventData fields
        _ = ph_event.insert_prop("command", &command);
        _ = ph_event.insert_prop("message", &message);

        // And the rest of the event values
        for (key, value) in values {
            _ = ph_event.insert_prop(key, value);
        }

        // We need to go add the api key to the event, since posthog_rs doesn't expose it for us...
        let mut value = serde_json::to_value(ph_event).unwrap();
        value["timestamp"] = serde_json::Value::String(time.to_rfc3339());
        value["api_key"] = serde_json::Value::String(
            "phc_d2jQTZMqAWxSkzv3NQ8TlxCP49vtBZ5ZmlYMIZLFNNU".to_string(),
        );

        // If the event is an error, we need to add the error properties so posthog picks it up.
        // The stackframes should be transformed already.
        if let Some(error_type) = error_type {
            value["event"] = "$exception".into();
            value["properties"]["$session_id"] = session_id.to_string().into();
            value["properties"]["$exception_list"] = json!([{
                "type": error_type,
                "value": &message,
                "mechanism": {
                    "handled": error_handled, // panics are not handled
                    "synthetic": false, // all errors/panics and "real", not emitted by sentry or the sdk
                },
                "stacktrace": {
                    "type": "raw", // debug symbols are generally used. this might be wrong?
                    "frames": stack_frames
                }
            }]);
        }

        value
    }

    fn enroll_reporter() -> Result<Reporter> {
        #[derive(Debug, Deserialize, Serialize)]
        struct ReporterSession {
            reporter_id: Uuid,
            session_id: Uuid,
            created_at: SystemTime,
            last_used: SystemTime,
        }

        // If the user requests telemetry disabled, we don't enroll them
        if CliSettings::telemetry_disabled() {
            bail!("Telemetry is disabled");
        }

        // Create the sessions folder if it doesn't exist
        let stats_folder = Workspace::dioxus_data_dir().join("stats");
        let sessions_folder = stats_folder.join("sessions");
        if !sessions_folder.exists() {
            std::fs::create_dir_all(&sessions_folder)?;
        }

        // Create a reporter_id. If we find an invalid reporter_id, we use `nil` as the reporter ID.
        let distinct_id_file = stats_folder.join("reporter.json");
        let reporter_session = std::fs::read_to_string(&distinct_id_file)
            .map(|e| serde_json5::from_str::<ReporterSession>(&e));

        // If we have a valid reporter session, we use it, otherwise we create a new one.
        let mut reporter = match reporter_session {
            Ok(Ok(session)) => session,
            _ => ReporterSession {
                reporter_id: Uuid::new_v4(),
                session_id: Uuid::new_v7(uuid::Timestamp::now(
                    uuid::timestamp::context::ContextV7::new(),
                )),
                created_at: SystemTime::now(),
                last_used: SystemTime::now(),
            },
        };

        // Update the last used time to now, updating the session ID if it's older than 30 minutes.
        if reporter
            .last_used
            .duration_since(SystemTime::now())
            .map(|d| d.as_secs() > (30 * 60))
            .unwrap_or(true)
        {
            reporter.created_at = SystemTime::now();
            reporter.session_id = Uuid::new_v7(uuid::Timestamp::now(
                uuid::timestamp::context::ContextV7::new(),
            ));
        }
        reporter.last_used = SystemTime::now();

        // Write the reporter session back to the file
        std::fs::write(&distinct_id_file, serde_json5::to_string(&reporter)?)?;

        Ok(Reporter {
            distinct_id: reporter.reporter_id,
            session_id: reporter.session_id,
        })
    }

    async fn finish(
        &self,
        res: Result<Result<StructuredOutput>, Box<dyn Any + Send>>,
    ) -> StructuredOutput {
        // Drain the tracer as regular messages
        self.tui_active.store(false, Ordering::Relaxed);
        reset_cursor();

        // re-emit any remaining messages in case they're useful.
        while let Ok(Some(msg)) = self.tui_rx.lock().await.try_next() {
            let content = match msg.content {
                TraceContent::Text(text) => text,
                TraceContent::Cargo(msg) => msg.message.to_string(),
            };
            match msg.level {
                Level::ERROR => tracing::error!("{content}"),
                Level::WARN => tracing::warn!("{content}"),
                Level::INFO => tracing::info!("{content}"),
                Level::DEBUG => tracing::debug!("{content}"),
                Level::TRACE => tracing::trace!("{content}"),
            }
        }

        // If we have "safe" error, we need to log it
        if let Ok(Err(err)) = &res {
            self.record_backtrace(
                err,
                "Fatal error",
                std::thread::current().name(),
                None,
                Default::default(),
            );
        }

        // And then we can finally flush telemetry to disk
        _ = self.flush_telemetry_to_disk().await;

        // Create the final output based on the result of the CLI run.
        match res {
            // No printing needed for successful runs, we just return the output for JSON output.
            Ok(Ok(output)) => output,

            // If the CLI run failed, we print the error and return it.
            // Anyhow gives us a nice stack trace, so we can just print it.
            // Eventually we might want to format this better since a full stack trace is kinda ugly.
            Ok(Err(err)) => {
                use crate::styles::{ERROR_STYLE, GLOW_STYLE};
                let arg = std::env::args().nth(1).unwrap_or_else(|| "dx".to_string());
                let err_display = format!("{err:?}")
                    .lines()
                    .take_while(|line| !line.ends_with("___rust_try"))
                    .join("\n");
                let message = format!(
                    "{ERROR_STYLE}ERROR{ERROR_STYLE:#} {GLOW_STYLE}dx {}{GLOW_STYLE:#}: {}",
                    arg, err_display
                );
                eprintln!("\n{message}");
                StructuredOutput::Error { message }
            }

            // The top-level thread panicked entirely. The panic handler should print the error for us.
            // Just return the error for the structured output.
            Err(e) => StructuredOutput::Error {
                message: if let Some(s) = e.downcast_ref::<String>() {
                    s.to_string()
                } else if let Some(s) = e.downcast_ref::<&str>() {
                    s.to_string()
                } else {
                    "<unknown error>".to_string()
                },
            },
        }
    }

    /// Flush telemetry to disk.
    ///
    /// Maybe flush telemetry immediately if the command is a long-running command like `dx serve` or `dx run` and there's an error.
    /// Currently just flushes telemetry immediately if we're in CI.
    async fn flush_telemetry_to_disk(&self) -> Result<()> {
        use std::io::Write;

        let reporter = self.reporter.as_ref().context("No reporter initialized")?;

        // If we're in CI, we try to upload the telemetry immediately, with a short timeout (5 seconds or so)
        // Hopefully it doesn't fail! Not much we can do in CI.
        match CliSettings::is_ci() {
            true => {
                let mut msgs = self.telemetry_rx.lock().await;

                let request_body = std::iter::from_fn(|| msgs.try_next().ok().flatten())
                    .filter_map(|msg| serde_json::to_value(msg).ok())
                    .collect::<Vec<_>>();

                _ = self.upload_to_posthog(&request_body).await;
            }

            // Dump the logs to a the session file as jsonl
            false => {
                let mut msgs = self.telemetry_rx.lock().await;

                let msg_list =
                    std::iter::from_fn(|| msgs.try_next().ok().flatten()).collect::<Vec<_>>();

                if !msg_list.is_empty() {
                    let dest = Workspace::dioxus_data_dir()
                        .join("stats")
                        .join("sessions")
                        .join(format!("{}.jsonl", reporter.session_id));

                    let mut logfile = std::fs::OpenOptions::new()
                        .append(true)
                        .create(true)
                        .open(dest)?;

                    for msg in msg_list {
                        serde_json::to_writer(&mut logfile, &msg)?;
                        writeln!(logfile)?;
                    }
                }
            }
        }

        Ok(())
    }

    fn posthog_capture_endpoint() -> String {
        format!(
            "{}/capture/",
            Self::posthog_endpoint().trim_end_matches('/')
        )
    }

    fn posthog_endpoint() -> String {
        // In dev mode we can override the endpoint with an environment variable
        if cfg!(debug_assertions) {
            if let Ok(endpoint) = env::var("DX_REPORTER_ENDPOINT") {
                return endpoint;
            }
        }

        "https://dx-cli-stats.dioxus.dev/".to_string()
    }

    /// Collect the raw arguments passed to the CLI and convert them into a telemetry event.
    ///
    /// This gives some usage information about which commands are being used and how. Especially useful
    /// if a particular session is failing in some way.
    pub(crate) fn to_invoked_event(arg: &crate::Commands) -> TelemetryEventData {
        let (message, anonymized) = Self::command_anonymized(arg);
        let raw_arguments = std::env::args()
            .map(|s| dioxus_cli_telemetry::strip_paths(&s))
            .collect::<Vec<_>>();
        TelemetryEventData::new("cli_invoked", message)
            .with_value("raw_arguments", raw_arguments)
            .with_value("arguments", anonymized)
    }

    /// Returns arguments that we're curious about
    ///
    /// The first return is the message, basically the command name in sequence
    /// The second return is a JSON object with the anonymized arguments as a structured value.
    pub(crate) fn command_anonymized(arg: &crate::Commands) -> (String, serde_json::Value) {
        use crate::cli::config::{Config, Setting};
        use crate::{print::Print, BuildTools};
        use cargo_generate::Vcs;

        match arg {
            Commands::New(new) => (
                "new".to_string(),
                json!({
                    "subtemplate": new.subtemplate,
                    "option": new.option,
                    "yes": new.yes,
                    "vcs": new.vcs.map(|v| match v {
                        Vcs::Git => "git",
                        Vcs::None => "none",
                    }),
                }),
            ),
            Commands::Serve(serve) => ("serve".to_string(), serve.anonymized()),
            Commands::Bundle(bundle) => (
                "bundle".to_string(),
                json!({
                    "package_types": bundle.package_types,
                    "out_dir": bundle.out_dir.is_some(),
                }),
            ),
            Commands::Build(build) => ("build".to_string(), build.anonymized()),
            Commands::Run(run) => ("run".to_string(), run.args.anonymized()),
            Commands::Init(cmd) => (
                "new".to_string(),
                json!({
                    "subtemplate": cmd.subtemplate,
                    "option": cmd.option,
                    "yes": cmd.yes,
                    "vcs": cmd.vcs.map(|v| match v {
                        Vcs::Git => "git",
                        Vcs::None => "none",
                    }),
                }),
            ),
            Commands::Doctor(_cmd) => ("doctor".to_string(), json!({})),
            Commands::Translate(cmd) => (
                "translate".to_string(),
                json!({
                    "file": cmd.file.is_some(),
                    "raw": cmd.raw.is_some(),
                    "output": cmd.output.is_some(),
                    "component": cmd.component,
                }),
            ),
            Commands::Autoformat(cmd) => (
                "fmt".to_string(),
                json!({
                    "all_code": cmd.all_code,
                    "check": cmd.check,
                    "raw": cmd.raw.is_some(),
                    "file": cmd.file.is_some(),
                    "split_line_attributes": cmd.split_line_attributes,
                    "package": cmd.package.is_some(),
                }),
            ),
            Commands::Check(cmd) => (
                "check".to_string(),
                json!({
                    "file": cmd.file.is_some(),
                    "build_args": cmd.build_args.anonymized(),
                }),
            ),
            Commands::Config(config) => match config {
                Config::Init { force, .. } => (
                    "config init".to_string(),
                    json!({
                        "force": force,
                    }),
                ),
                Config::FormatPrint {} => ("config format-print".to_string(), json!({})),
                Config::CustomHtml {} => ("config custom-html".to_string(), json!({})),
                Config::Set(setting) => (
                    format!("config set {}", setting),
                    match setting {
                        Setting::AlwaysHotReload { value } => json!({ "value": value }),
                        Setting::AlwaysOpenBrowser { value } => json!({ "value": value }),
                        Setting::AlwaysOnTop { value } => json!({ "value": value }),
                        Setting::WSLFilePollInterval { value } => json!({ "value": value }),
                        Setting::DisableTelemetry { value } => json!({ "value": value }),
                        Setting::PreferredEditor { value } => json!({ "value": format!("{:?}", value) }),
                    },
                ),
            },
            Commands::SelfUpdate(cmd) => (
                "update".to_string(),
                json!({
                    "nightly": cmd.nightly,
                    "version": cmd.version,
                    "install": cmd.install,
                    "list": cmd.list,
                    "force": cmd.force,
                }),
            ),
            Commands::Tools(tool) => match tool {
                BuildTools::BuildAssets(_build_assets) => ("tools assets".to_string(), json!({})),
                BuildTools::HotpatchTip(_hotpatch_tip) => ("tools hotpatch".to_string(), json!({})),
            },
            Commands::Print(print) => match print {
                Print::ClientArgs(_args) => ("print client-args".to_string(), json!({})),
                Print::ServerArgs(_args) => ("print server-args".to_string(), json!({})),
            },
            Commands::Components(cmd) => match cmd {
                ComponentCommand::Add {
                    component,
                    registry,
                    force,
                } => (
                    "components add".to_string(),
                    json!({
                        "component": component,
                        "registry": registry,
                        "force": force,
                    }),
                ),
                ComponentCommand::Remove {
                    component,
                    registry,
                } => (
                    "components remove".to_string(),
                    json!({
                        "component": component,
                        "registry": registry,
                    }),
                ),
                ComponentCommand::Update { registry } => (
                    "components update".to_string(),
                    json!({
                        "registry": registry,
                    }),
                ),
                ComponentCommand::List { registry } => (
                    "components list".to_string(),
                    json!({
                        "registry": registry,
                    }),
                ),
                ComponentCommand::Clean => ("components clean".to_string(), json!({})),
                ComponentCommand::Schema => ("components schema".to_string(), json!({})),
            },
        }
    }

    fn record_backtrace(
        &self,
        error: &Error,
        error_type: &str,
        thread_name: Option<&str>,
        location: Option<&SavedLocation>,
        extra: serde_json::Map<String, serde_json::Value>,
    ) {
        let stacktrace = sentry_backtrace::parse_stacktrace(&format!("{:#}", error.backtrace()));
        let stack_frames: Vec<_> = stacktrace
            .unwrap_or_default()
            .frames
            .into_iter()
            .rev()
            .map(|frame| {
                // Convert the sentry type to the posthog type
                dioxus_cli_telemetry::StackFrame {
                    platform: "custom".to_string(),
                    raw_id: frame
                        .image_addr
                        .map(|addr| addr.to_string())
                        .unwrap_or_else(|| Uuid::new_v4().to_string()),
                    mangled_name: frame
                        .function
                        .as_ref()
                        .map(|f| f.to_string())
                        .unwrap_or_else(|| "<unknown>".to_string()),
                    resolved_name: frame
                        .function
                        .as_ref()
                        .map(|f| f.to_string())
                        .unwrap_or_else(|| "<unknown>".to_string()),
                    lang: "rust".to_string(),
                    resolved: true,
                    filename: frame.filename,
                    function: frame.function,
                    module: frame.module,
                    lineno: frame.lineno,
                    colno: frame.colno,
                    abs_path: frame.abs_path,
                    context_line: frame.context_line,
                    pre_context: frame.pre_context,
                    post_context: frame.post_context,
                    in_app: frame.in_app,
                    instruction_addr: frame.instruction_addr.map(|addr| addr.to_string()),
                    addr_mode: frame.addr_mode,
                    vars: frame.vars,
                    chunk_id: None,
                }
            })
            .collect();

        let (mut file, mut line, mut column) = location
            .as_ref()
            .map(|l| (l.file.to_string(), l.line, l.column))
            .unwrap_or_else(|| ("<unknown>".to_string(), 0, 0));

        // If the location is not provided, we try to extract it from the backtrace.
        // I eyeballed that 5 is enough to get to the actual panic location in most cases.
        if location.is_none() {
            if let Some(frame) = stack_frames.get(5) {
                if let Some(abs_path) = &frame.abs_path {
                    file = abs_path.to_string();
                }
                if let Some(lineno) = frame.lineno {
                    line = lineno as _;
                }
                if let Some(colno) = frame.colno {
                    column = colno as _;
                }
            }
        }

        let event = TelemetryEventData::new(error_type.to_string(), error.root_cause())
            .with_value("thread_name", thread_name)
            .with_error_type(error_type.to_string())
            .with_error_handled(false)
            .with_file(file)
            .with_line_column(line, column)
            .with_stack_frames(stack_frames)
            .with_values(extra);

        _ = self.telemetry_tx.unbounded_send(event);
    }
}

impl<S> Layer<S> for TraceController
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_event(
        &self,
        event: &tracing::Event<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        let mut visitor = CollectVisitor::default();
        event.record(&mut visitor);
        let meta = event.metadata();
        let level = meta.level();

        // Redirect to the TUI if it's active
        if self.tui_active.load(Ordering::Relaxed) {
            let final_msg = visitor.pretty();

            if visitor.source == TraceSrc::Unknown {
                visitor.source = TraceSrc::Dev;
            }

            _ = self
                .tui_tx
                .unbounded_send(TraceMsg::text(visitor.source, *level, final_msg));
        }

        // Handle telemetry events.
        // Only tracing events annotated with `telemetry = %json! { ... }` will be captured.
        // This applies to errors too - if you want an error to be captured, it must have a telemetry field!
        //
        // If the event is `error!()` it counts as a an "$exception" event in posthog
        // If it has a backtrace, then we upload the backtrace as well.
        if let Some(telemetry) = visitor.fields.get("telemetry") {
            let extra =
                serde_json::from_str::<serde_json::Map<String, serde_json::Value>>(telemetry)
                    .unwrap_or_default();

            let stage = match visitor.source {
                TraceSrc::Cargo | TraceSrc::App(_) => "",
                TraceSrc::Dev => "dev",
                TraceSrc::Build => "build",
                TraceSrc::Bundle => "bundle",
                TraceSrc::Unknown => "unknown",
            };

            // If the event has the captured panic attached, we record it through capture_backtrace directly.
            // Otherwise, we won't have a backtrace to work with, so we just log the event as a telemetry event.
            if let Some(error) = visitor.captured_panic {
                self.record_backtrace(
                    error.error.as_ref(),
                    &error.error_type,
                    error.thread_name.as_deref(),
                    error.location.as_ref(),
                    extra,
                );
            } else {
                let is_err = level == &Level::ERROR;
                let event_name = match visitor.fields.get("event") {
                    Some(event) => event.clone(),
                    None if is_err => "Runtime Error".into(),
                    None => "Telemetry Event".into(),
                };

                let mut event = TelemetryEventData::new(event_name, visitor.message.as_str())
                    .with_file(meta.file().unwrap_or("<unknown>").to_string())
                    .with_line_column(meta.line().unwrap_or_default(), 0)
                    .with_value("trace_stage", stage)
                    .with_value("trace_level", meta.level().to_string())
                    .with_value("trace_name", meta.name())
                    .with_value("trace_target", meta.target())
                    .with_value("trace_module_path", meta.module_path())
                    .with_values(extra)
                    .with_values(visitor.fields_to_json());

                if is_err {
                    event = event
                        .with_error_type("Runtime Error".to_string())
                        .with_error_handled(false);
                }

                _ = self.telemetry_tx.unbounded_send(event);
            }
        }

        // Write to a file if we need to.
        if let Some(open_file) = self.log_to_file.as_ref() {
            let new_line = if visitor.source == TraceSrc::Cargo {
                Cow::Borrowed(visitor.message.as_str())
            } else {
                let meta = event.metadata();
                let level = meta.level();

                let mut final_msg = String::new();
                _ = write!(
                    final_msg,
                    "[{level}] {}: {} ",
                    meta.module_path().unwrap_or("dx"),
                    visitor.message.as_str()
                );

                for (field, value) in visitor.fields.iter() {
                    _ = write!(final_msg, "{} ", format_field(field, value));
                }
                _ = writeln!(final_msg);
                Cow::Owned(final_msg)
            };

            let new_data = console::strip_ansi_codes(&new_line).to_string();
            if let Ok(mut file) = open_file.try_lock() {
                use std::io::Write;
                _ = writeln!(file, "{}", new_data);
            }
        }
    }
}

/// A record visitor that collects dx-specific info and user-provided fields for logging consumption.
#[derive(Default)]
struct CollectVisitor {
    message: String,
    source: TraceSrc,
    fields: HashMap<String, String>,
    captured_panic: Option<CapturedPanicError>,
}

impl Visit for CollectVisitor {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        let name = field.name();

        let mut value_string = String::new();
        write!(value_string, "{value:?}").unwrap();

        if name == "message" {
            self.message = value_string;
            return;
        }

        if name == DX_SRC_FLAG {
            self.source = TraceSrc::from(value_string);
            return;
        }

        if name == "json" || name == "backtrace" {
            return;
        }

        self.fields.insert(name.to_string(), value_string);
    }

    fn record_error(
        &mut self,
        field: &tracing::field::Field,
        value: &(dyn std::error::Error + 'static),
    ) {
        if let Some(captured) = value.downcast_ref::<CapturedPanicError>() {
            self.captured_panic = Some(captured.clone());
        } else {
            self.record_debug(field, &tracing::field::display(value));
        }
    }
}
impl CollectVisitor {
    fn pretty(&self) -> String {
        let mut final_msg = String::new();
        write!(final_msg, "{} ", self.message).unwrap();

        for (field, value) in self.fields.iter() {
            if field == "json" || field == "telemetry" || field == "backtrace" {
                continue;
            }

            write!(final_msg, "{} ", format_field(field, value)).unwrap();
        }
        final_msg
    }

    fn fields_to_json(&self) -> serde_json::Map<String, serde_json::Value> {
        self.fields
            .iter()
            .map(|(k, v)| (k.clone(), serde_json::Value::String(v.clone())))
            .collect()
    }
}

/// Formats a tracing field and value, removing any internal fields from the final output.
fn format_field(field_name: &str, value: &dyn Debug) -> String {
    match field_name {
        "message" => format!("{value:?}"),
        _ => format!("{field_name}={value:?}"),
    }
}

#[derive(Clone, PartialEq)]
pub struct TraceMsg {
    pub source: TraceSrc,
    pub level: Level,
    pub content: TraceContent,
    pub timestamp: chrono::DateTime<chrono::Local>,
}

#[derive(Clone, PartialEq)]
#[allow(clippy::large_enum_variant)]
pub enum TraceContent {
    Cargo(Diagnostic),
    Text(String),
}

impl TraceMsg {
    pub fn text(source: TraceSrc, level: Level, content: String) -> Self {
        Self {
            source,
            level,
            content: TraceContent::Text(content),
            timestamp: chrono::Local::now(),
        }
    }

    /// Create a new trace message from a cargo compiler message
    ///
    /// All `cargo` messages are logged at the `TRACE` level since they get *very* noisy during development
    pub fn cargo(content: Diagnostic) -> Self {
        Self {
            level: match content.level {
                DiagnosticLevel::Ice => Level::ERROR,
                DiagnosticLevel::Error => Level::ERROR,
                DiagnosticLevel::FailureNote => Level::ERROR,
                DiagnosticLevel::Warning => Level::TRACE,
                DiagnosticLevel::Note => Level::TRACE,
                DiagnosticLevel::Help => Level::TRACE,
                _ => Level::TRACE,
            },
            timestamp: chrono::Local::now(),
            source: TraceSrc::Cargo,
            content: TraceContent::Cargo(content),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Default)]
pub enum TraceSrc {
    App(BundleFormat),
    Dev,
    Build,
    Bundle,
    Cargo,

    #[default]
    Unknown,
}

impl std::fmt::Debug for TraceSrc {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        <Self as Display>::fmt(self, f)
    }
}

impl From<String> for TraceSrc {
    fn from(value: String) -> Self {
        match value.as_str() {
            "dev" => Self::Dev,
            "bld" => Self::Build,
            "cargo" => Self::Cargo,
            other => BundleFormat::from_str(other)
                .map(Self::App)
                .unwrap_or_else(|_| Self::Unknown),
        }
    }
}

impl Display for TraceSrc {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::App(bundle) => write!(f, "{bundle}"),
            Self::Dev => write!(f, "dev"),
            Self::Build => write!(f, "build"),
            Self::Cargo => write!(f, "cargo"),
            Self::Unknown => write!(f, "n/a"),
            Self::Bundle => write!(f, "bundle"),
        }
    }
}

/// Retrieve and print the relative elapsed wall-clock time since an epoch.
///
/// The `Default` implementation for `Uptime` makes the epoch the current time.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct PrettyUptime {
    epoch: Instant,
}

impl Default for PrettyUptime {
    fn default() -> Self {
        Self {
            epoch: Instant::now(),
        }
    }
}

impl FormatTime for PrettyUptime {
    fn format_time(&self, w: &mut Writer<'_>) -> std::fmt::Result {
        let e = self.epoch.elapsed();
        write!(w, "{:4}.{:2}s", e.as_secs(), e.subsec_millis())
    }
}
/// Run the provided future and wait for it to complete, handling Ctrl-C gracefully.
///
/// If ctrl-c is pressed twice, it exits immediately, skipping our telemetry flush.
/// Todo: maybe we want a side thread continuously flushing telemetry, esp if the user double ctrl-cs
async fn run_with_ctrl_c(
    f: impl Future<Output = Result<StructuredOutput>>,
) -> Result<StructuredOutput> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    let mut tx = Some(tx);

    _ = ctrlc::set_handler(move || {
        match tx.take() {
            // If we have a sender, we unset the following `select!` and continue from there
            Some(tx) => _ = tx.send(()),

            // If we get a second ctrl-c, we just exit immediately
            None => {
                reset_cursor();
                std::process::exit(1);
            }
        }
    });

    tokio::select! {
        _ = rx => Ok(StructuredOutput::Success),
        res = f => res,
    }
}

/// We only store non-pii information in telemetry to track issues and performance
/// across the CLI.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Reporter {
    pub session_id: Uuid,
    pub distinct_id: Uuid,
}
