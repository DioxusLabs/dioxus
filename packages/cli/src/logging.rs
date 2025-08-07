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

use crate::{dx_build_info::GIT_COMMIT_HASH_SHORT, serve::ServeUpdate, Cli, Commands, Verbosity};
use crate::{BundleFormat, CliSettings, Workspace};
use anyhow::{bail, Context, Result};
use cargo_metadata::diagnostic::{Diagnostic, DiagnosticLevel};
use clap::Parser;
use dioxus_cli_telemetry::{Reporter, TelemetryEventData};
use dioxus_dx_wire_format::StructuredOutput;
use futures_channel::mpsc::{UnboundedReceiver, UnboundedSender};
use futures_util::{FutureExt, TryFutureExt};
use serde_json::json;
use std::{any::Any, backtrace::Backtrace, io::Read, str::FromStr, sync::Arc};
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

static BACKTRACE: OnceLock<CapturedBacktrace> = OnceLock::new();
pub static VERBOSITY: OnceLock<Verbosity> = OnceLock::new();

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

struct CapturedBacktrace {
    backtrace: Backtrace,
    location: Option<SavedLocation>,
}

struct SavedLocation {
    file: String,
    line: u32,
    column: u32,
}

impl TraceController {
    pub const EVENT_HEARTBEAT: &str = "cli_heartbeat";

    /// Initialize the CLI and set up the tracing infrastructure
    ///
    /// This captures panics and flushes telemetry to a file after the CLI has run.
    ///
    /// We pass the TraceController around the CLI in a few places, namely the serve command so the TUI
    /// can access things like the logs.
    pub async fn main<F>(run_app: impl FnOnce(Commands, Self) -> F) -> Result<StructuredOutput>
    where
        F: Future<Output = Result<StructuredOutput>>,
    {
        let args = Cli::parse();
        let tui_active = Arc::new(AtomicBool::new(false));

        VERBOSITY
            .set(args.verbosity.clone())
            .expect("verbosity should only be set once");

        // Set up a basic env-based filter for the logs
        let env_filter = match env::var(LOG_ENV) {
            Ok(_) => EnvFilter::from_env(LOG_ENV),
            _ if matches!(args.action, Commands::Serve(_)) => EnvFilter::new("error,dx=trace,dioxus_cli=trace,manganis_cli_support=trace,wasm_split_cli=trace,subsecond_cli_support=trace"),
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
            move |_| !tui_active.load(Ordering::Relaxed)
        }));

        // Set up the tokio console subscriber if enabled
        #[cfg(feature = "tokio-console")]
        let console_layer = console_subscriber::spawn();
        #[cfg(not(feature = "tokio-console"))]
        let console_layer = tracing_subscriber::layer::Identity::new();

        // Construct our custom layer that handles the TUI and file logging
        // Initialize the log file if we use it
        let log_to_file = args
            .verbosity
            .log_to_file
            .as_deref()
            .map(|file_path| {
                if !file_path.exists() {
                    _ = std::fs::write(file_path, "");
                }
                std::fs::OpenOptions::new()
                    .append(true)
                    .create(true)
                    .open(file_path)
                    .map(|file| Arc::new(tokio::sync::Mutex::new(file)))
            })
            .transpose()?;

        // Create a new session ID for this invocation of the CLI
        let reporter = Self::enroll_reporter().ok().map(|distinct_id| Reporter {
            distinct_id,
            session_id: Uuid::new_v4(),
        });

        // Create a new telemetry uploader
        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .inspect_err(|err| tracing::debug!("Failed to create tracing HTTP client: {}", err))
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
                .upload_telemetry_files(Self::to_heartbeat_event(&args.action))
                .map_err(|err| tracing::error!("Failed to upload telemetry files: {}", err)),
        );

        // Construct the tracing subscriber
        tracing_subscriber::registry()
            .with(env_filter)
            .with(json_filter)
            .with(tracer.clone())
            .with(console_layer)
            .with(fmt_layer)
            .init();

        // Set the panic handler to capture backtraces but not print them. We will print them later.
        std::panic::set_hook(Box::new(move |panic_info| {
            _ = BACKTRACE.set(CapturedBacktrace {
                backtrace: Backtrace::capture(),
                location: panic_info.location().map(|l| SavedLocation {
                    file: l.file().to_string(),
                    line: l.line(),
                    column: l.column(),
                }),
            });
        }));

        // Run the app, catching panics and errors, early flushing if `ctrl_c` is pressed.
        let app_res = AssertUnwindSafe(run_with_ctrl_c(run_app(args.action, tracer.clone())))
            .catch_unwind()
            .await;

        // Write any in-flight logs to the file / telemetry queue
        if let Err(e) = tracer.flush_logs(&app_res).await {
            tracing::trace!("Failed to finish logging: {}", e);
        }

        // Do any final logging cleanup
        tracer.cleanup_ui(app_res).await
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
    async fn upload_telemetry_files(self, heartbeat: TelemetryEventData) -> Result<()> {
        use fs2::FileExt;

        // Wait a little bit to prevent abuse (spam loops) and not do extra work if it's a simple `--help` call
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        // Send off a heartbeat request. If this fails, we skip anything else.
        self.send_heartbeat(heartbeat).await?;

        // Wait a few seconds to see if we can end up in `dx serve` or a long-running task
        // If we're in CI though, we do want to flush telemetry immediately
        if !CliSettings::is_ci() {
            tokio::time::sleep(std::time::Duration::from_millis(3000)).await;
        }

        tracing::info!("Uploading telemetry files...");

        // Now start loading telemetry files, locking them, and then uploading them.
        let stats_dir = Workspace::dioxus_data_dir().join("stats").join("sessions");
        for entry in stats_dir.read_dir()?.flatten() {
            tracing::info!("Processing telemetry file: {}", entry.path().display());

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
            let reporter = self.reporter.as_ref().context("no reporter")?;
            let session_id = entry
                .path()
                .file_stem()
                .and_then(|s| Uuid::from_str(s.to_str()?).ok());
            let request_body = jsonl_file
                .lines()
                .map(serde_json::from_str::<TelemetryEventData>)
                .filter_map(Result::ok)
                .map(|event| Self::telemetry_to_posthog(reporter, event, session_id))
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

    async fn send_heartbeat(&self, heartbeat: TelemetryEventData) -> Result<()> {
        let reporter = self.reporter.as_ref().context("No reporter initialized")?;
        let body = Self::telemetry_to_posthog(reporter, heartbeat, None);
        self.upload_to_posthog(&vec![body]).await
    }

    /// Convert the dioxus-cli-telemetry event into a posthog event.
    ///
    /// We try to maintain the same structure for each telemetry event to do advanced filtering on the backend.
    fn telemetry_to_posthog(
        reporter: &Reporter,
        event: TelemetryEventData,
        custom_sesion: Option<Uuid>,
    ) -> serde_json::Value {
        let Reporter {
            session_id,
            distinct_id,
        } = reporter;

        let TelemetryEventData {
            action,
            module,
            message,
            time,
            values,
            command,
        } = event;

        let mut ph_event = posthog_rs::Event::new(action, distinct_id.to_string());

        // The reporter's fields
        _ = ph_event.insert_prop("is_ci", CliSettings::is_ci());
        _ = ph_event.insert_prop("session_id", custom_sesion.unwrap_or(*session_id));
        _ = ph_event.insert_prop("distinct_id", distinct_id.to_string());
        _ = ph_event.insert_prop("host_os", target_lexicon::HOST.operating_system.to_string());
        _ = ph_event.insert_prop("host_arch", target_lexicon::HOST.architecture.to_string());
        _ = ph_event.insert_prop("host_triple", target_lexicon::Triple::host().to_string());
        _ = ph_event.insert_prop("cli_version_major", crate::dx_build_info::PKG_VERSION_MAJOR);
        _ = ph_event.insert_prop("cli_version_minor", crate::dx_build_info::PKG_VERSION_MINOR);
        _ = ph_event.insert_prop("cli_version_patch", crate::dx_build_info::PKG_VERSION_PATCH);
        _ = ph_event.insert_prop("cli_version_pre", crate::dx_build_info::PKG_VERSION_PRE);
        _ = ph_event.insert_prop("cli_commit_hash", GIT_COMMIT_HASH_SHORT.unwrap_or_default());

        // And the TelemetryEventData fields
        _ = ph_event.insert_prop("command", command);
        _ = ph_event.insert_prop("message", message);
        _ = ph_event.insert_prop("module", module);

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

        value
    }

    fn enroll_reporter() -> Result<Uuid> {
        // If the user requests telemetry disabled, we don't enroll them
        if CliSettings::telemetry_disabled() {
            bail!("Telemetry is disabled");
        }

        // Create the sessions folder if it doesn't exist\
        let stats_folder = Workspace::dioxus_data_dir().join("stats");
        let sessions_folder = stats_folder.join("sessions");
        if !sessions_folder.exists() {
            std::fs::create_dir_all(&sessions_folder)?;
        }

        // Create a reporter_id. If we find an invalid reporter_id, we use `nil` as the reporter ID.
        // If users want to enroll in telemetry but don't want a reporter ID, they can replace the
        // contents of the file with anything that is not a valid UUID.
        let stable_session_file = stats_folder.join("reporter_id.json");
        let reporter_id = if stable_session_file.exists() {
            let contents = std::fs::read_to_string(stable_session_file)?;
            serde_json::from_str::<Uuid>(&contents).unwrap_or(Uuid::from_u128(1))
        } else {
            let new_id = Uuid::new_v4();
            std::fs::write(stable_session_file, serde_json::to_string(&new_id)?)?;
            new_id
        };

        Ok(reporter_id)
    }

    async fn cleanup_ui(
        &self,
        app_res: Result<Result<StructuredOutput>, Box<dyn Any + Send>>,
    ) -> Result<StructuredOutput> {
        // Kill the screen so we don't ruin the terminal
        _ = crate::serve::Output::remote_shutdown(true);

        // And drain the tracer as regular messages. All messages will be logged (including traces)
        // and then we can print the panic message
        self.tui_active.store(false, Ordering::Relaxed);

        // re-emit any remaining messages
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

        // Return the right UI and error
        match app_res {
            Ok(Ok(res)) => Ok(res),
            Ok(Err(e)) => {
                println!();
                Err(e)
            }
            Err(panic_err) => {
                println!();

                // And then print the panic itself.
                let as_str = if let Some(p) = panic_err.downcast_ref::<String>() {
                    p.as_ref()
                } else if let Some(p) = panic_err.downcast_ref::<&str>() {
                    p
                } else {
                    "<unknown panic>"
                };

                // Pretty-print the backtrace if we have it
                let message = match BACKTRACE.get() {
                    Some(loc) => {
                        let location_display = loc
                            .location
                            .as_ref()
                            .map(|l| format!("{}:{}:{}", l.file, l.line, l.column))
                            .unwrap_or_else(|| "<unknown>".to_string());

                        let clean = clean_backtrace(&loc.backtrace);

                        format!("dx panicked at {location_display}\n{as_str}\n\n{clean}")
                    }
                    None => format!("dx panicked: {as_str}"),
                };

                Err(anyhow::anyhow!(message))
            }
        }
    }

    /// Flush pending logs to the telemetry file.
    async fn flush_logs(
        &self,
        res: &Result<Result<StructuredOutput>, Box<dyn Any + Send>>,
    ) -> Result<()> {
        use std::io::Write;

        // Only flush telemetry if we have a reporter
        let Some(reporter) = self.reporter.as_ref() else {
            return Ok(());
        };

        // Add the fatal error to the telemetry
        if let Ok(Err(err)) = res.as_ref() {
            _ = self.telemetry_tx.unbounded_send(fatal_error(err));
        }

        // If there's a panic, we also want to capture that
        if let Err(panic_err) = res {
            // if let Some(bt) = bt {
            //     println!("{:?}", bt);
            // }
            // println!("dx panicked: {}", panic_err);
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
                Some(loc) => {
                    let location_display = loc
                        .location
                        .as_ref()
                        .map(|l| format!("{}:{}:{}", l.file, l.line, l.column))
                        .unwrap_or_else(|| "<unknown>".to_string());

                    // let backtrace_display = clean_backtrace(back);

                    // // Add the fatal error to the telemetry
                    // send_telemetry_event(TelemetryEvent::new(
                    //     "panic",
                    //     Some(location_display.clone()),
                    //     &backtrace_display,
                    //     "error",
                    // ));

                    format!("dx serve panicked at {location_display}\n{as_str}\n{location_display} ___rust_try")
                }
                None => {
                    // Add the fatal error to the telemetry
                    // send_telemetry_event(TelemetryEvent::new("panic", None, as_str, "error"));
                    format!("dx serve panicked: {as_str}")
                }
            };
        }

        // If we're in CI, we try to upload the telemetry immediately, with a short timeout (5 seconds or so)
        // Hopefully it doesn't fail! Not much we can do in CI.
        match CliSettings::is_ci() {
            true => {
                let mut msgs = self.telemetry_rx.lock().await;

                let request_body = std::iter::from_fn(|| msgs.try_next().ok().flatten())
                    .filter_map(|msg| serde_json::to_value(msg).ok())
                    .collect::<Vec<_>>();

                self.upload_to_posthog(&request_body).await?;
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

    pub(crate) fn to_heartbeat_event(arg: &crate::Commands) -> TelemetryEventData {
        let (name, anonymized) = Self::command_anonymized(arg);
        TelemetryEventData::new("heartbeat", name, module_path!())
            .with_value(
                "raw_arguments",
                std::env::args()
                    .skip(1)
                    .map(|s| dioxus_cli_telemetry::strip_paths(&s))
                    .collect::<Vec<_>>(),
            )
            .with_value("arguments", anonymized)
    }

    /// Returns arguments that we're curious about
    pub(crate) fn command_anonymized(arg: &crate::Commands) -> (String, serde_json::Value) {
        use crate::cli::config::{Config, Setting};
        use crate::BuildTools;
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
            Commands::Run(run) => (
                "run".to_string(),
                json! {{
                    "args": run.args.anonymized()
                }},
            ),
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
            },
        }
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
            let mut final_msg = String::new();
            write!(final_msg, "{} ", visitor.message).unwrap();

            for (field, value) in visitor.fields.iter() {
                write!(final_msg, "{} ", format_field(field, value)).unwrap();
            }

            if visitor.source == TraceSrc::Unknown {
                visitor.source = TraceSrc::Dev;
            }

            _ = self
                .tui_tx
                .unbounded_send(TraceMsg::text(visitor.source, *level, final_msg));
        }

        // Send a telemetry event if we have a telemetry channel
        //
        // the goal here is to enabled `dx::trace!(telemetry = %json! { a: 123, b: 123  });
        let event_location = || {
            meta.file()
                .and_then(|f| Some((f, meta.line()?)))
                .map(|(file, line)| format!("{file}:{line}"))
                .unwrap_or_else(|| "<unknown>".to_string())
        };
        match visitor.fields.get("telemetry") {
            // If the event has a telemetry field, we pass it through directly, but we also add the location
            // Otherwise, we try to create it from the event metadata
            Some(raw_event) => {
                _ = self.telemetry_tx.unbounded_send(
                    serde_json::from_str::<TelemetryEventData>(raw_event)
                        .unwrap()
                        .with_value("event_location", event_location()),
                );
            }

            // Only capture errors or explicit telemetry events
            // We ignore Cargo and User messages
            None if (*level == Level::ERROR
                && matches!(
                    visitor.source,
                    TraceSrc::Dev | TraceSrc::Build | TraceSrc::Bundle | TraceSrc::Unknown
                )) =>
            {
                let stage = match visitor.source {
                    TraceSrc::Cargo | TraceSrc::App(_) => "",
                    TraceSrc::Dev => "dev",
                    TraceSrc::Build => "build",
                    TraceSrc::Bundle => "bundle",
                    TraceSrc::Unknown => "unknown",
                };

                let mut event = TelemetryEventData::new(
                    "tracing error",
                    visitor.message.as_str(),
                    meta.module_path()
                        .map(ToString::to_string)
                        .unwrap_or_else(|| "<unknown>".to_string()),
                )
                .with_value("event_location", event_location())
                .with_value("stage", stage);

                for (field, value) in visitor.fields.iter() {
                    event = event.with_value(field, value);
                }

                _ = self.telemetry_tx.unbounded_send(event);
            }

            // Otherwise, ignore it
            _ => {}
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

        self.fields.insert(name.to_string(), value_string);
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

pub fn fatal_error(error: &crate::Error) -> TelemetryEventData {
    let mut telemetry_event =
        TelemetryEventData::new("fatal_error", error.to_string(), module_path!());
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
                _ = console::Term::stdout().show_cursor();
                std::process::exit(1);
            }
        }
    });

    tokio::select! {
        _ = rx => Ok(StructuredOutput::ExitRequested),
        res = f => res,
    }
}

// curl -X POST "https://us.i.posthog.com/i/v0/e/" \
//      -H "Content-Type: application/json" \
//      -d '{
//          "api_key": "phc_d2jQTZMqAWxSkzv3NQ8TlxCP49vtBZ5ZmlYMIZLFNNU",
//          "event": "$exception",
//          "properties": {
//              "distinct_id": "distinct_id_of_your_user",
//              "$exception_list": [{
//                  "type": "RangeError",
//                  "value": "Maximum call stack size exceeded",
//                  "mechanism": {
//                      "handled": true,
//                      "synthetic": false
//                  },
//                  "stacktrace": {
//                      "type": "resolved",
//                      "frames": [
//                          {
//                              "raw_id": "89f3907385ddb1bdc2feebd77625387d685e3da2f9a68bcf2634e3b3b39122e51f895e5c880def374525c558899b2e90eed214900ff9c8f949243e4b79eb32a9",
//                              "mangled_name": "Array.forEach",
//                              "in_app": false,
//                              "resolved_name": "Array.forEach",
//                              "lang": "javascript",
//                              "resolved": true
//                          },
//                          /* Additional frames omitted for brevity */
//                      ]
//                  }
//              }],
//              "$exception_fingerprint": "209842d96784e19321e3a36b068d53fff7a01ebcb1da9e98df35c4c49db0b4f3b62aea7ee25a714470e61f8d36b4716f227f241c153477e5fa9adfda64ce9f71"
//          },
//      }'
