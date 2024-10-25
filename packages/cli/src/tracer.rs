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

use crate::{serve::ServeUpdate, Platform as TargetPlatform};
use cargo_metadata::{diagnostic::DiagnosticLevel, CompilerMessage};
use futures_channel::mpsc::{unbounded, UnboundedReceiver, UnboundedSender};
use once_cell::sync::OnceCell;
use std::{
    collections::HashMap,
    env,
    fmt::{Debug, Display, Write as _},
    fs,
    io::{self, Write},
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Mutex,
    },
};
use tracing::{field::Visit, Level, Subscriber};
use tracing_subscriber::{fmt::format, prelude::*, registry::LookupSpan, EnvFilter, Layer};

const LOG_ENV: &str = "DIOXUS_LOG";
const LOG_FILE_NAME: &str = "dx.log";
const DX_SRC_FLAG: &str = "dx_src";

pub fn log_path() -> PathBuf {
    let tmp_dir = std::env::temp_dir();
    tmp_dir.join(LOG_FILE_NAME)
}

static TUI_ENABLED: AtomicBool = AtomicBool::new(false);
static TUI_TX: OnceCell<UnboundedSender<TraceMsg>> = OnceCell::new();

pub(crate) struct TraceController {
    pub(crate) tui_rx: UnboundedReceiver<TraceMsg>,
}

impl TraceController {
    /// Get a handle to the trace controller.
    pub fn redirect() -> Self {
        let (tui_tx, tui_rx) = unbounded();
        TUI_ENABLED.store(true, Ordering::SeqCst);
        TUI_TX.set(tui_tx.clone()).unwrap();
        Self { tui_rx }
    }

    /// Wait for the internal logger to send a message
    pub(crate) async fn wait(&mut self) -> ServeUpdate {
        use futures_util::StreamExt;
        let log = self.tui_rx.next().await.expect("tracer should never die");
        ServeUpdate::TracingLog { log }
    }

    pub(crate) fn shutdown(&self) {
        TUI_ENABLED.store(false, Ordering::SeqCst);
    }

    /// Build tracing infrastructure.
    pub fn initialize() {
        let mut filter =
            EnvFilter::new("error,dx=trace,dioxus-cli=debug,manganis-cli-support=debug");

        if env::var(LOG_ENV).is_ok() {
            filter = EnvFilter::from_env(LOG_ENV);
        }

        // Log file
        let log_path = log_path();
        _ = std::fs::write(&log_path, "");
        let file_append_layer = match FileAppendLayer::new(log_path) {
            Ok(f) => Some(f),
            Err(e) => {
                tracing::error!(dx_src = ?TraceSrc::Dev, err = ?e, "failed to init log file");
                None
            }
        };

        // Build CLI layer
        let cli_layer = CLILayer;

        // Build fmt layer
        let fmt_layer = tracing_subscriber::fmt::layer()
            .fmt_fields(
                format::debug_fn(|writer, field, value| {
                    write!(writer, "{}", format_field(field.name(), value))
                })
                .delimited(" "),
            )
            .with_writer(Mutex::new(FmtLogWriter {}))
            .with_timer(tracing_subscriber::fmt::time::time());

        let sub = tracing_subscriber::registry()
            .with(filter)
            .with(file_append_layer)
            .with(cli_layer)
            .with(fmt_layer);

        #[cfg(feature = "tokio-console")]
        let sub = sub.with(console_subscriber::spawn());

        sub.init();
    }
}

/// A logging layer that appends to a file.
///
/// This layer returns on any error allowing the cli to continue work
/// despite failing to log to a file. This helps in case of permission errors and similar.
struct FileAppendLayer {
    file_path: PathBuf,
    buffer: Mutex<String>,
}

impl FileAppendLayer {
    pub fn new(file_path: PathBuf) -> io::Result<Self> {
        Ok(Self {
            file_path,
            buffer: Mutex::new(String::new()),
        })
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
        let mut visitor = CollectVisitor::new();
        event.record(&mut visitor);

        // If the TUI output is disabled we let fmt subscriber handle the logs
        // EXCEPT for cargo logs which we just print.
        if !TUI_ENABLED.load(Ordering::SeqCst) {
            if visitor.source == TraceSrc::Cargo {
                println!("{}", visitor.message);
            }
            return;
        }

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

        TUI_TX
            .get()
            .unwrap()
            .unbounded_send(TraceMsg::text(visitor.source, *level, final_msg))
            .unwrap();
    }

    // TODO: support spans? structured tui log display?
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

struct FmtLogWriter {}

impl Write for FmtLogWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
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
