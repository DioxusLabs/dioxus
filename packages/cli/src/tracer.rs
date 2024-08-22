use futures_channel::mpsc::{unbounded, UnboundedReceiver, UnboundedSender};
use std::{
    env, io,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
};
use tracing_subscriber::{prelude::*, EnvFilter};

const LOG_ENV: &str = "DIOXUS_LOG";

/// Build tracing infrastructure.
pub fn build_tracing() -> CLILogControl {
    // If {LOG_ENV} is set, default to env, otherwise filter to cli
    // and manganis warnings and errors from other crates
    let mut filter = EnvFilter::new("error,dx=info,dioxus-cli=info");
    if env::var(LOG_ENV).is_ok() {
        filter = EnvFilter::from_env(LOG_ENV);
    }

    // Create writer controller and custom writer.
    let (tui_tx, tui_rx) = unbounded();
    let tui_enabled = Arc::new(AtomicBool::new(false));

    let writer_control = CLILogControl {
        tui_rx,
        tui_enabled: tui_enabled.clone(),
    };
    let cli_writer = Mutex::new(CLIWriter::new(tui_enabled, tui_tx));

    // Build tracing
    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_writer(cli_writer)
        .with_filter(filter);
    let sub = tracing_subscriber::registry().with(fmt_layer);

    #[cfg(feature = "tokio-console")]
    let sub = sub.with(console_subscriber::spawn());

    sub.init();

    writer_control
}

/// Contains the sync primitives to control the CLIWriter.
pub struct CLILogControl {
    pub tui_rx: UnboundedReceiver<String>,
    pub tui_enabled: Arc<AtomicBool>,
}

/// Represents the CLI's custom tracing writer for conditionally writing logs between outputs.
pub struct CLIWriter {
    stdout: io::Stdout,
    tui_tx: UnboundedSender<String>,
    tui_enabled: Arc<AtomicBool>,
}

impl CLIWriter {
    /// Create a new CLIWriter with required sync primitives for conditionally routing logs.
    pub fn new(tui_enabled: Arc<AtomicBool>, tui_tx: UnboundedSender<String>) -> Self {
        Self {
            stdout: io::stdout(),
            tui_tx,
            tui_enabled,
        }
    }
}

// Implement a conditional writer so that logs are routed to the appropriate place.
impl io::Write for CLIWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if self.tui_enabled.load(Ordering::SeqCst) {
            let len = buf.len();

            let as_string = String::from_utf8(buf.to_vec())
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

            self.tui_tx
                .unbounded_send(as_string)
                .map_err(|e| io::Error::new(io::ErrorKind::BrokenPipe, e))?;

            Ok(len)
        } else {
            self.stdout.write(buf)
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        if !self.tui_enabled.load(Ordering::SeqCst) {
            self.stdout.flush()
        } else {
            Ok(())
        }
    }
}
