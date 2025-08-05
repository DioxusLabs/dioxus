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

use crate::BundleFormat;
use crate::{serve::ServeUpdate, Cli, Commands, Verbosity};
use crate::{
    telemetry::{self, flush_old_telemetry, flush_telemetry_to_file},
    Workspace,
};
use anyhow::Result;
use cargo_metadata::diagnostic::{Diagnostic, DiagnosticLevel};
use clap::Parser;
use dioxus_cli_telemetry::TelemetryEvent;
use dioxus_dx_wire_format::StructuredOutput;
use futures_channel::mpsc::{unbounded, UnboundedReceiver, UnboundedSender};
use futures_util::FutureExt;
use std::{any::Any, backtrace::Backtrace, str::FromStr, sync::Arc};
use std::{borrow::Cow, sync::OnceLock};
use std::{
    collections::HashMap,
    env,
    fmt::{Debug, Display, Write as _},
    fs,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Mutex,
    },
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
static TUI_ACTIVE: AtomicBool = AtomicBool::new(false);
static TUI_TX: OnceLock<UnboundedSender<TraceMsg>> = OnceLock::new();
pub static VERBOSITY: OnceLock<Verbosity> = OnceLock::new();

pub(crate) struct TraceController {
    pub(crate) tui_rx: UnboundedReceiver<TraceMsg>,
}

impl TraceController {
    /// Initialize the CLI and set up the tracing infrastructure
    ///
    /// This captures panics and flushes telemetry to a file after the CLI has run.
    pub async fn main<F>(run_app: impl FnOnce(Commands) -> F) -> Result<StructuredOutput>
    where
        F: Future<Output = Result<StructuredOutput>>,
    {
        let args = Cli::parse();

        VERBOSITY
            .set(args.verbosity.clone())
            .expect("verbosity should only be set once");

        // Set up a basic env-based filter for the logs
        let env_filter = if env::var(LOG_ENV).is_ok() {
            EnvFilter::from_env(LOG_ENV)
        } else if matches!(args.action, Commands::Serve(_)) {
            EnvFilter::new(
                "error,dx=trace,dioxus_cli=trace,manganis_cli_support=trace,wasm_split_cli=trace,subsecond_cli_support=trace",
            )
        } else {
            EnvFilter::new(format!(
                "error,dx={our_level},dioxus_cli={our_level},manganis_cli_support={our_level},wasm_split_cli={our_level},subsecond_cli_support={our_level}",
                our_level = if args.verbosity.verbose {
                    "debug"
                } else {
                    "info"
                }
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
        .with_filter(tracing_subscriber::filter::filter_fn(|_| {
            !TUI_ACTIVE.load(Ordering::Relaxed)
        }));

        // Set up the tokio console subscriber if enabled
        #[cfg(feature = "tokio-console")]
        let console_layer = console_subscriber::spawn();
        #[cfg(not(feature = "tokio-console"))]
        let console_layer = tracing_subscriber::layer::Identity::new();

        // Construct our custom layer that handles the TUI and file logging
        let dx_layer = DxLayer::new(&args);

        // Construct the tracing subscriber
        tracing_subscriber::registry()
            .with(env_filter)
            .with(json_filter)
            .with(dx_layer.clone())
            .with(console_layer)
            .with(fmt_layer)
            .init();

        // Construct a ctrl-c handler that attempts to exit the CLI gracefully

        // Run the app, catching panics and errors
        //
        // *All* panics make it into the telemetry collector
        // Only some get printed to the console.
        let app_res = AssertUnwindSafe(run_app(args.action)).catch_unwind().await;

        // Write any in-flight logs to the file / telemetry queue
        dx_layer.finish(&app_res);

        // Flush the telemetry to a file.
        // If CI=1, we wait for the telemetry to be flushed before exiting.
        let output = match app_res {
            Ok(Ok(res)) => Ok(res),
            Ok(Err(e)) => Err(e),
            Err(panic_err) => {
                // And then print the panic itself.
                let as_str = if let Some(p) = panic_err.downcast_ref::<String>() {
                    p.as_ref()
                } else if let Some(p) = panic_err.downcast_ref::<&str>() {
                    p
                } else {
                    "<unknown panic>"
                };

                todo!()

                // Err(anyhow::anyhow!(message))
            }
        };

        output
    }

    /// Get a handle to the trace controller.
    pub fn redirect(interactive: bool) -> Self {
        let (tui_tx, tui_rx) = unbounded();

        if interactive {
            TUI_ACTIVE.store(true, Ordering::Relaxed);
            TUI_TX.set(tui_tx.clone()).unwrap();
        }

        Self { tui_rx }
    }

    /// Wait for the internal logger to send a message
    pub(crate) async fn wait(&mut self) -> ServeUpdate {
        use futures_util::StreamExt;

        let Some(log) = self.tui_rx.next().await else {
            return std::future::pending().await;
        };

        ServeUpdate::TracingLog { log }
    }

    pub(crate) fn shutdown_panic(&mut self) {
        TUI_ACTIVE.store(false, Ordering::Relaxed);

        // re-emit any remaining messages
        while let Ok(Some(msg)) = self.tui_rx.try_next() {
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
    }
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

/// A custom layer that wraps our special interception logic based on the mode of the CLI and its verbosity.
///
/// Redirects TUI logs, writes to files, and queues telemetry events.
#[derive(Clone)]
pub struct DxLayer {
    session_id: Uuid,
    telemetry_tx: UnboundedSender<TelemetryEvent>,
    telemetry_rx: Arc<Mutex<UnboundedReceiver<TelemetryEvent>>>,
    log_to_file: Option<PathBuf>,
    log_tile_file_buffer: Arc<Mutex<String>>,
}

impl DxLayer {
    fn new(args: &crate::cli::Cli) -> Self {
        // Initialize the log file if we use it
        let log_to_file = match args.verbosity.log_to_file.as_deref() {
            Some(file_path) => {
                if !file_path.exists() {
                    _ = std::fs::write(file_path, "");
                }
                Some(file_path.to_path_buf())
            }
            None => None,
        };

        // Create a new session ID for this invocation of the CLI
        let session_id = Uuid::new_v4();

        // Create a new telemetry channel
        // Note that we only drain the channel at the end of the CLI run, so it's not really being used as a channel - more of a vecdeque
        let (telemetry_tx, telemetry_rx) = futures_channel::mpsc::unbounded();

        // Create an owned version of ourselves
        let s = Self {
            session_id,
            telemetry_tx,
            log_to_file,
            telemetry_rx: Arc::new(Mutex::new(telemetry_rx)),
            log_tile_file_buffer: Arc::new(Mutex::new(String::new())),
        };

        // Send out the heartbeat event and then try to start uploading any existing telemetry logs
        let (cmd, value) = args.action.command_anonymized();
        let telemetry_event = TelemetryEvent::new(name, module, message, stage);

        tokio::spawn(async move {
            use reqwest::{header::CONTENT_TYPE, Client as HttpClient};

            // Try to initialize the telemetry session
            let reporter_id = Self::enroll_reporter().ok();

            // Wait a little bit to prevent abuse (spam loops)
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;

            // Create the telemetry client
            let client = HttpClient::builder()
                .timeout(std::time::Duration::from_secs(5))
                .build()
                .unwrap();

            // Send off a heartbeat request

            // Wait a few seconds to see if we can end up in `dx serve` or a long-running task
            // If we're in CI though, we do want to flush telemetry immediately
            if std::env::var("CI").is_err() {
                tokio::time::sleep(std::time::Duration::from_millis(5000)).await;
            }

            // Now start loading telemetry files, locking them, and then uploading them.
        });

        s
    }

    fn enroll_reporter() -> Result<Uuid> {
        // If the user requests telemetry disabled, we don't enroll them

        // Create the sessions folder if it doesn't exist
        let sessions_folder = Workspace::dioxus_home_dir().join("stats");
        if !sessions_folder.exists() {
            std::fs::create_dir_all(&sessions_folder);
        }

        // Create a reporter_id
        let stable_session_file = sessions_folder.join("reporter_id.json");
        let reporter_id = if stable_session_file.exists() {
            let contents = std::fs::read_to_string(stable_session_file)?;
            serde_json::from_str::<Uuid>(&contents)?
        } else {
            let new_id = Uuid::new_v4();
            std::fs::write(stable_session_file, serde_json::to_string(&new_id)?)?;
            new_id
        };

        Ok(reporter_id)
    }

    fn start_telemetry_thread() {
        todo!()
    }

    /// When building the CLI in CI, we include a key used to sign telemetry events.
    /// This lets us verify that the telemetry events are coming from a mostly trusted source.
    ///
    /// Note that the key here is not the same key for the backend - our proxy fixes up the key to
    /// pass along. This way we can cycle keys on the backend without breaking the CLI.
    fn untrusted_api_key() -> &'static str {
        option_env!("DX_RELEASE_ANALYTICS_KEY").unwrap_or_else(|| "untrusted")
    }

    /// Flush pending logs to the telemetry file.
    fn finish(&self, res: &Result<Result<StructuredOutput>, Box<dyn Any + Send>>) -> Result<()> {
        use std::io::Write;

        // Add the fatal error to the telemetry
        if let Ok(Err(err)) = res.as_ref() {
            _ = self.telemetry_tx.unbounded_send(fatal_error(&err));
        }

        // If there's a panic, we also want to capture that
        if let Err(panic_err) = res {
            let backtrace = BACKTRACE.get();
            // // Attempt to emulate the default panic hook
            // let message = match BACKTRACE.get() {
            //     Some((back, location)) => {
            //         let location_display = location
            //             .as_ref()
            //             .map(|l| format!("{}:{}:{}", l.file, l.line, l.column))
            //             .unwrap_or_else(|| "<unknown>".to_string());

            //         let backtrace_display = clean_backtrace(back);

            //         // Add the fatal error to the telemetry
            //         send_telemetry_event(TelemetryEvent::new(
            //             "panic",
            //             Some(location_display.clone()),
            //             &backtrace_display,
            //             "error",
            //         ));

            //         format!("dx serve panicked at {location_display}\n{as_str}\n{backtrace_display} ___rust_try")
            //     }
            //     None => {
            //         // Add the fatal error to the telemetry
            //         send_telemetry_event(TelemetryEvent::new("panic", None, as_str, "error"));
            //         format!("dx serve panicked: {as_str}")
            //     }
            // };
        }

        // Create a new file to dump our telemetry to
        let path = self.session_logs_file();
        std::fs::create_dir_all(path.parent().unwrap())?;
        let mut out = std::fs::File::options()
            .create(true)
            .append(true)
            .open(path)?;

        // Write the built-up telemetry events to the file
        let mut messages = self.telemetry_rx.lock().unwrap();
        while let Ok(Some(msg)) = messages.try_next() {
            writeln!(out, "{}", serde_json::to_string(&msg).unwrap())?;
        }

        Ok(())
    }

    fn session_logs_file(&self) -> PathBuf {
        Workspace::dioxus_home_dir()
            .join("stats")
            .join("sessions")
            .join(format!("stats-{}.jsonl", self.session_id))
    }
}

impl<S> Layer<S> for DxLayer
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
        if TUI_ACTIVE.load(Ordering::Relaxed) {
            let mut final_msg = String::new();
            write!(final_msg, "{} ", visitor.message).unwrap();

            for (field, value) in visitor.fields.iter() {
                write!(final_msg, "{} ", format_field(field, value)).unwrap();
            }

            if visitor.source == TraceSrc::Unknown {
                visitor.source = TraceSrc::Dev;
            }

            _ = TUI_TX.get().unwrap().unbounded_send(TraceMsg::text(
                visitor.source,
                *level,
                final_msg,
            ));
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
                    serde_json::from_str::<TelemetryEvent>(raw_event)
                        .unwrap()
                        .with_value("location", event_location()),
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

                let mut event = TelemetryEvent::new(
                    "tracing error",
                    meta.module_path().map(ToString::to_string),
                    visitor.message.as_str(),
                    stage,
                )
                .with_value("location", event_location());

                for (field, value) in visitor.fields.iter() {
                    event = event.with_value(field, value);
                }

                _ = self.telemetry_tx.unbounded_send(event);
            }

            // Otherwise, ignore it
            _ => {}
        }

        // Write to a file if we need to.
        if let Some(file_path) = self.log_to_file.as_ref() {
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
            if let Ok(mut buf) = self.log_tile_file_buffer.lock() {
                *buf += &new_data;
                _ = fs::write(file_path, buf.as_bytes());
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

pub fn fatal_error(error: &crate::Error) -> TelemetryEvent {
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
