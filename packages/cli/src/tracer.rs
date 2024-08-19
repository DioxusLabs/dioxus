//! CLI Tracing
//! 
//! Tracing has internal and user-facing logs. User-facing logs are directly router to the user in some form.
//! Internal logs are stored in a log file for consumption in bug reports and debugging.
//! We use tracing fields to determine whether a log is interal or external and additionally if the log should be
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

use crate::serve::output::{Message, MessageSource};
use console::strip_ansi_codes;
use futures_channel::mpsc::{unbounded, UnboundedReceiver, UnboundedSender};
use std::{
    collections::HashMap,
    env,
    fmt::{Debug, Write as _},
    fs,
    io::{self, Write},
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
};
use tracing::{field::Visit, Subscriber};
use tracing_subscriber::{
    filter::filter_fn, fmt::format, prelude::*, registry::LookupSpan, EnvFilter, Layer,
};

const LOG_ENV: &str = "DIOXUS_LOG";
const LOG_FILE_NAME: &str = "dx.log";
const DX_SRC_FLAG: &str = "dx_src";
const DX_NO_FMT_FLAG: &str = "dx_no_fmt";

/// Build tracing infrastructure.
pub fn build_tracing() -> CLILogControl {
    let mut filter = EnvFilter::new("error,dx=info,dioxus-cli=info,manganis-cli-support=info");
    if env::var(LOG_ENV).is_ok() {
        filter = EnvFilter::from_env(LOG_ENV);
    }

    // Log file
    let log_path = std::env::temp_dir().join(LOG_FILE_NAME);
    _ = std::fs::write(&log_path, "");
    let file_append_layer = match FileAppendLayer::new(log_path) {
        Ok(f) => Some(f),
        Err(e) => {
            tracing::error!(dx_src = ?MessageSource::Dev, err = ?e, "failed to init log file");
            None
        }
    };

    // Create writer controller and custom writer.
    let (output_tx, output_rx) = unbounded();
    let output_enabled = Arc::new(AtomicBool::new(false));
    let writer_control = CLILogControl {
        output_rx,
        output_enabled: output_enabled.clone(),
    };

    // Build CLI layer
    let cli_layer = CLILayer::new(output_enabled.clone(), output_tx);

    // Build fmt layer
    let formatter = format::debug_fn(|writer, field, value| {
        write!(writer, "{}", format_field(field.name(), value))
    })
    .delimited(" ");

    // Format subscriber
    let fmt_writer = Mutex::new(FmtLogWriter::new(output_enabled));
    let fmt_layer = tracing_subscriber::fmt::layer()
        .fmt_fields(formatter)
        .with_writer(fmt_writer)
        .without_time()
        .with_filter(filter_fn(|meta| {
            // Filter any logs with "dx_no_fmt" or is not user facing (no dx_src)
            let mut fields = meta.fields().iter();
            let has_src_flag = fields.any(|f| f.name() == DX_SRC_FLAG);

            if !has_src_flag {
                return false;
            }

            let has_fmt_flag = fields.any(|f| f.name() == DX_NO_FMT_FLAG);
            if has_fmt_flag {
                return false;
            }

            true
        }));

    let sub = tracing_subscriber::registry()
        .with(filter)
        .with(file_append_layer)
        .with(cli_layer)
        .with(fmt_layer);

    #[cfg(feature = "tokio-console")]
    let sub = sub.with(console_subscriber::spawn());

    sub.init();

    writer_control
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

        let new_line = if visitor.source == MessageSource::Cargo
            || event.fields().any(|f| f.name() == DX_NO_FMT_FLAG)
        {
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
        let new_data = strip_ansi_codes(&new_line).to_string();

        if let Ok(mut buf) = self.buffer.lock() {
            *buf += &new_data;
            // TODO: Make this efficient.
            _ = fs::write(&self.file_path, buf.as_bytes());
        }
    }
}

/// This is our "subscriber" (layer) that records structured data for the tui output.
struct CLILayer {
    internal_output_enabled: Arc<AtomicBool>,
    output_tx: UnboundedSender<Message>,
}

impl CLILayer {
    pub fn new(
        internal_output_enabled: Arc<AtomicBool>,
        output_tx: UnboundedSender<Message>,
    ) -> Self {
        Self {
            internal_output_enabled,
            output_tx,
        }
    }
}

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
        // We only care about user-facing logs.
        let has_src_flag = event.fields().any(|f| f.name() == DX_SRC_FLAG);
        if !has_src_flag {
            return;
        }

        let mut visitor = CollectVisitor::new();
        event.record(&mut visitor);

        // If the TUI output is disabled we let fmt subscriber handle the logs
        // EXCEPT for cargo logs which we just print.
        if !self.internal_output_enabled.load(Ordering::SeqCst) {
            if visitor.source == MessageSource::Cargo
                || event.fields().any(|f| f.name() == DX_NO_FMT_FLAG)
            {
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

        if visitor.source == MessageSource::Unknown {
            visitor.source = MessageSource::Dev;
        }

        self.output_tx
            .unbounded_send(Message::new(visitor.source, *level, final_msg))
            .unwrap();
    }

    // TODO: support spans? structured tui log display?
}

/// A record visitor that collects dx-specific info and user-provided fields for logging consumption.
struct CollectVisitor {
    pub message: String,
    pub source: MessageSource,
    pub dx_user_msg: bool,
    pub fields: HashMap<String, String>,
}

impl CollectVisitor {
    pub fn new() -> Self {
        Self {
            message: String::new(),
            source: MessageSource::Unknown,
            dx_user_msg: false,
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
            self.source = MessageSource::from(value_string);
            self.dx_user_msg = true;
            return;
        }

        self.fields.insert(name.to_string(), value_string);
    }
}

// Contains the sync primitives to control the CLIWriter.
pub struct CLILogControl {
    pub output_rx: UnboundedReceiver<Message>,
    pub output_enabled: Arc<AtomicBool>,
}

struct FmtLogWriter {
    stdout: io::Stdout,
    output_enabled: Arc<AtomicBool>,
}

impl FmtLogWriter {
    pub fn new(output_enabled: Arc<AtomicBool>) -> Self {
        Self {
            stdout: io::stdout(),
            output_enabled,
        }
    }
}

impl Write for FmtLogWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        // Handle selection between TUI or Terminal output.
        if !self.output_enabled.load(Ordering::SeqCst) {
            self.stdout.write(buf)
        } else {
            Ok(buf.len())
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        if !self.output_enabled.load(Ordering::SeqCst) {
            self.stdout.flush()
        } else {
            Ok(())
        }
    }
}

/// Formats a tracing field and value, removing any internal fields from the final output.
fn format_field(field_name: &str, value: &dyn Debug) -> String {
    let mut out = String::new();
    match field_name {
        DX_SRC_FLAG => write!(out, ""),
        DX_NO_FMT_FLAG => write!(out, ""),
        "message" => write!(out, "{:?}", value),
        _ => write!(out, "{}={:?}", field_name, value),
    }
    .unwrap();

    out
}
