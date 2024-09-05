use crate::cli::serve::ServeArgs;
use futures_channel::mpsc::{unbounded, UnboundedReceiver, UnboundedSender};
use futures_util::StreamExt;
use std::{
    env, io,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
};
use tracing_subscriber::{prelude::*, EnvFilter};
const LOG_ENV: &str = "DIOXUS_LOG";

use super::ServeUpdate;

pub struct TraceController {
    pub tui_rx: UnboundedReceiver<String>,
    pub tui_enabled: Arc<AtomicBool>,
}

impl TraceController {
    pub fn start(cfg: &ServeArgs) -> Self {
        // Start a tracing instance just for serving.
        // This ensures that any tracing we do while serving doesn't break the TUI itself, and instead is
        // redirected to the serve process.
        // If {LOG_ENV} is set, default to env, otherwise filter to cli
        // and manganis warnings and errors from other crates
        let mut filter = EnvFilter::new("error,dx=info,dioxus-cli=info");
        if env::var(LOG_ENV).is_ok() {
            filter = EnvFilter::from_env(LOG_ENV);
        }

        // Create writer controller and custom writer.
        let (tui_tx, tui_rx) = unbounded();
        let tui_enabled = Arc::new(AtomicBool::new(true));

        let writer_control = Self {
            tui_rx,
            tui_enabled: tui_enabled.clone(),
        };

        let cli_writer = Mutex::new(Writer {
            stdout: io::stdout(),
            tui_tx,
            tui_enabled,
        });

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

    /// Wait for the internal logger to send a message
    pub async fn wait(&mut self) -> ServeUpdate {
        ServeUpdate::TracingLog {
            log: self.tui_rx.next().await.expect("tracer should never die"),
        }
    }

    pub fn shutdown(&self) {
        self.tui_enabled.store(false, Ordering::SeqCst);
    }
}

/// Represents the CLI's custom tracing writer for conditionally writing logs between outputs.
struct Writer {
    stdout: io::Stdout,
    tui_tx: UnboundedSender<String>,
    tui_enabled: Arc<AtomicBool>,
}

// Implement a conditional writer so that logs are routed to the appropriate place.
impl io::Write for Writer {
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
