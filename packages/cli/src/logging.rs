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

use crate::{serve::ServeUpdate, Cli, Commands, Platform as TargetPlatform, Verbosity};
use cargo_metadata::{diagnostic::DiagnosticLevel, CompilerMessage};
use clap::Parser;
use futures_channel::mpsc::{unbounded, UnboundedReceiver, UnboundedSender};
use once_cell::sync::OnceCell;
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

const LOG_ENV: &str = "DIOXUS_LOG";
const LOG_FILE_NAME: &str = "dx.log";
const DX_SRC_FLAG: &str = "dx_src";

static TUI_ACTIVE: AtomicBool = AtomicBool::new(false);
static TUI_TX: OnceCell<UnboundedSender<TraceMsg>> = OnceCell::new();
pub static VERBOSITY: OnceCell<Verbosity> = OnceCell::new();

pub(crate) struct TraceController {
    pub(crate) tui_rx: UnboundedReceiver<TraceMsg>,
}

impl TraceController {
    /// Initialize the CLI and set up the tracing infrastructure
    pub fn initialize() -> Cli {
        let args = Cli::parse();

        VERBOSITY
            .set(args.verbosity)
            .expect("verbosity should only be set once");

        // By default we capture ourselves at a higher tracing level when serving
        // This ensures we're tracing ourselves even if we end up tossing the logs
        let filter = if env::var(LOG_ENV).is_ok() {
            EnvFilter::from_env(LOG_ENV)
        } else if matches!(args.action, Commands::Serve(_)) {
            EnvFilter::new("error,dx=trace,dioxus-cli=trace,manganis-cli-support=trace")
        } else {
            EnvFilter::new(format!(
                "error,dx={our_level},dioxus-cli={our_level},manganis-cli-support={our_level}",
                our_level = if args.verbosity.verbose {
                    "debug"
                } else {
                    "info"
                }
            ))
        };

        let json_filter = tracing_subscriber::filter::filter_fn(move |meta| {
            if meta.fields().len() == 1 && meta.fields().iter().next().unwrap().name() == "json" {
                return args.verbosity.json_output;
            }
            true
        });

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

        let fmt_layer = if args.verbosity.json_output {
            fmt_layer.json().flatten_event(true).boxed()
        } else {
            fmt_layer.boxed()
        };

        // When running in interactive mode (of which serve is the only one), we don't want to log to console directly
        let print_fmts_filter =
            tracing_subscriber::filter::filter_fn(|_| !TUI_ACTIVE.load(Ordering::Relaxed));

        let sub = tracing_subscriber::registry()
            .with(filter)
            .with(json_filter)
            .with(FileAppendLayer::new())
            .with(CLILayer {})
            .with(fmt_layer.with_filter(print_fmts_filter));

        #[cfg(feature = "tokio-console")]
        let sub = sub.with(console_subscriber::spawn());

        sub.init();

        args
    }

    /// Get a handle to the trace controller.
    pub fn redirect() -> Self {
        let (tui_tx, tui_rx) = unbounded();
        TUI_ACTIVE.store(true, Ordering::Relaxed);
        TUI_TX.set(tui_tx.clone()).unwrap();

        Self { tui_rx }
    }

    /// Wait for the internal logger to send a message
    pub(crate) async fn wait(&mut self) -> ServeUpdate {
        use futures_util::StreamExt;
        let log = self.tui_rx.next().await.expect("tracer should never die");
        ServeUpdate::TracingLog { log }
    }
}

impl Drop for TraceController {
    fn drop(&mut self) {
        TUI_ACTIVE.store(false, Ordering::Relaxed);

        // re-emit any remaining messages
        while let Ok(Some(msg)) = self.tui_rx.try_next() {
            let contents = match msg.content {
                TraceContent::Text(text) => text,
                TraceContent::Cargo(msg) => msg.message.to_string(),
            };
            tracing::error!("{}", contents);
        }
    }
}

/// A logging layer that appends to a file.
///
/// This layer returns on any error allowing the cli to continue work
/// despite failing to log to a file. This helps in case of permission errors and similar.
pub(crate) struct FileAppendLayer {
    file_path: PathBuf,
    buffer: Mutex<String>,
}

impl FileAppendLayer {
    fn new() -> Self {
        let file_path = Self::log_path();

        if !file_path.exists() {
            _ = std::fs::write(&file_path, "");
        }

        Self {
            file_path,
            buffer: Mutex::new(String::new()),
        }
    }

    pub(crate) fn log_path() -> PathBuf {
        std::env::temp_dir().join(LOG_FILE_NAME)
    }
}

impl<S> Layer<S> for FileAppendLayer
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_event(
        &self,
        event: &tracing::Event<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        let mut visitor = CollectVisitor::new();
        event.record(&mut visitor);

        let new_line = if visitor.source == TraceSrc::Cargo {
            visitor.message
        } else {
            let meta = event.metadata();
            let level = meta.level();

            let mut final_msg = String::new();
            _ = write!(
                final_msg,
                "[{level}] {}: {} ",
                meta.module_path().unwrap_or("dx"),
                visitor.message
            );

            for (field, value) in visitor.fields.iter() {
                _ = write!(final_msg, "{} ", format_field(field, value));
            }
            _ = writeln!(final_msg);
            final_msg
        };

        // Append logs
        let new_data = console::strip_ansi_codes(&new_line).to_string();

        if let Ok(mut buf) = self.buffer.lock() {
            *buf += &new_data;
            // TODO: Make this efficient.
            _ = fs::write(&self.file_path, buf.as_bytes());
        }
    }
}

/// This is our "subscriber" (layer) that records structured data for the tui output.
struct CLILayer;

impl<S> Layer<S> for CLILayer
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    // Subscribe to user
    fn on_event(
        &self,
        event: &tracing::Event<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        if !TUI_ACTIVE.load(Ordering::Relaxed) {
            return;
        }

        let mut visitor = CollectVisitor::new();
        event.record(&mut visitor);

        let meta = event.metadata();
        let level = meta.level();

        let mut final_msg = String::new();
        write!(final_msg, "{} ", visitor.message).unwrap();

        for (field, value) in visitor.fields.iter() {
            write!(final_msg, "{} ", format_field(field, value)).unwrap();
        }

        if visitor.source == TraceSrc::Unknown {
            visitor.source = TraceSrc::Dev;
        }

        _ = TUI_TX
            .get()
            .unwrap()
            .unbounded_send(TraceMsg::text(visitor.source, *level, final_msg));
    }
}

/// A record visitor that collects dx-specific info and user-provided fields for logging consumption.
struct CollectVisitor {
    message: String,
    source: TraceSrc,
    fields: HashMap<String, String>,
}

impl CollectVisitor {
    pub fn new() -> Self {
        Self {
            message: String::new(),
            source: TraceSrc::Unknown,
            fields: HashMap::new(),
        }
    }
}

impl Visit for CollectVisitor {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        let name = field.name();

        let mut value_string = String::new();
        write!(value_string, "{:?}", value).unwrap();

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
    let mut out = String::new();
    match field_name {
        "message" => write!(out, "{:?}", value),
        _ => write!(out, "{}={:?}", field_name, value),
    }
    .unwrap();

    out
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
    Cargo(CompilerMessage),
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
    pub fn cargo(content: CompilerMessage) -> Self {
        Self {
            level: match content.message.level {
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

#[derive(Clone, PartialEq)]
pub enum TraceSrc {
    App(TargetPlatform),
    Dev,
    Build,
    Bundle,
    Cargo,
    Unknown,
}

impl std::fmt::Debug for TraceSrc {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let as_string = self.to_string();
        write!(f, "{as_string}")
    }
}

impl From<String> for TraceSrc {
    fn from(value: String) -> Self {
        match value.as_str() {
            "dev" => Self::Dev,
            "bld" => Self::Build,
            "cargo" => Self::Cargo,
            "app" => Self::App(TargetPlatform::Web),
            "windows" => Self::App(TargetPlatform::Windows),
            "macos" => Self::App(TargetPlatform::MacOS),
            "linux" => Self::App(TargetPlatform::Linux),
            "server" => Self::App(TargetPlatform::Server),
            _ => Self::Unknown,
        }
    }
}

impl Display for TraceSrc {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::App(platform) => match platform {
                TargetPlatform::Web => write!(f, "web"),
                TargetPlatform::MacOS => write!(f, "macos"),
                TargetPlatform::Windows => write!(f, "windows"),
                TargetPlatform::Linux => write!(f, "linux"),
                TargetPlatform::Server => write!(f, "server"),
                TargetPlatform::Ios => write!(f, "ios"),
                TargetPlatform::Android => write!(f, "android"),
                TargetPlatform::Liveview => write!(f, "liveview"),
            },
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
