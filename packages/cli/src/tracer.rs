use crate::serve::output::{Message, MessageSource};
use futures_channel::mpsc::{unbounded, UnboundedReceiver, UnboundedSender};
use std::{
    collections::HashMap,
    env,
    fmt::Write,
    io,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
};
use tracing::{field::Visit, Subscriber};
use tracing_subscriber::{fmt::format, prelude::*, registry::LookupSpan, EnvFilter, Layer};

const LOG_ENV: &str = "DIOXUS_LOG";

/// Build tracing infrastructure.
pub fn build_tracing() -> CLILogControl {
    // TODO: clean these comments
    // 1. Set EnvFilter Layer
    // 2. Rolling log file layer
    // 3. Custom subscriber that filters any internal logs from continuing and sends user-facing logs to output.
    // 4. Tracing subscriber fmt layer for any user-facing messages

    // If the tui output is enabled we must send all logs to the TUI for handling
    // If the tui output is disabled we need to rely on the EnvFilter for filtering
    // and format the output and print it.

    // A subscriber that provides structured logs to the output if enabled
    // A writer that disables fmt subscriber from outputting if tui is enabled

    // A relaxed filter for consuming logs with the TUI.
    // The TUI will have it's own filtering mechanism built-in.
    let relaxed_filter = EnvFilter::new("info");

    // A hardened filter for non-interactive user-facing logs.
    // If {LOG_ENV} is set, default to env, otherwise filter to cli
    // and manganis warnings and errors from other crates
    let mut filter = EnvFilter::new("error,dx=info,dioxus-cli=info,manganis-cli-support=info");
    if env::var(LOG_ENV).is_ok() {
        filter = EnvFilter::from_env(LOG_ENV);
    }

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
    let formatter = format::debug_fn(|writer, field, value| match field.name() {
        "dx_src" => write!(writer, ""),
        "message" => write!(writer, "{:?}", value),
        _ => write!(writer, "{}={:?}", field, value),
    })
    .delimited(" ");

    let fmt_writer = Mutex::new(FmtLogWriter::new(output_enabled));

    let fmt_layer = tracing_subscriber::fmt::layer()
        .fmt_fields(formatter)
        .with_writer(fmt_writer)
        .without_time();

    let sub = tracing_subscriber::registry()
        .with(relaxed_filter)
        //.with(rolling_log_file)
        .with(cli_layer)
        .with(filter)
        .with(fmt_layer);

    #[cfg(feature = "tokio-console")]
    let sub = sub.with(console_subscriber::spawn());

    sub.init();

    writer_control
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
        if !self.internal_output_enabled.load(Ordering::SeqCst) {
            return;
        }

        let meta = event.metadata();
        let level = meta.level();

        let mut visitor = CollectVisitor::new();
        event.record(&mut visitor);

        let mut final_msg = String::new();
        write!(final_msg, "{}", visitor.message).unwrap();

        for (field, value) in visitor.fields.iter() {
            write!(final_msg, "{} = {}", field, value).unwrap();
        }

        if visitor.source == MessageSource::Unknown {
            visitor.source = MessageSource::Dev;
        }

        self.output_tx
            .unbounded_send(Message::new(visitor.source, level.clone(), final_msg))
            .unwrap();
    }

    // We don't want internal events to be user-facing so we disable the rest
    // of the stack if the user-facing opt-in isn't set.
    //
    // We could convert this to per-layer filtering instead of global if we wanted to
    // create a tab on the TUI for non-user-facing logs.
    fn event_enabled(
        &self,
        event: &tracing::Event<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) -> bool {
        let mut visitor = CollectVisitor::new();
        event.record(&mut visitor);

        visitor.dx_user_msg
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

        if name == "dx_src" {
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

impl io::Write for FmtLogWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
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
