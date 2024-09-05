use futures_channel::mpsc::{unbounded, UnboundedReceiver, UnboundedSender};
use futures_util::StreamExt;
use once_cell::sync::OnceCell;
use std::{
    env, io,
    sync::{
        atomic::{AtomicBool, Ordering},
        Mutex,
    },
};
use tracing_subscriber::{prelude::*, EnvFilter};
const LOG_ENV: &str = "DIOXUS_LOG";

use super::ServeUpdate;

static TUI_ENABLED: AtomicBool = AtomicBool::new(false);
static TUI_TX: OnceCell<UnboundedSender<String>> = OnceCell::new();

pub(crate) struct TraceController {
    pub(crate) tui_rx: UnboundedReceiver<String>,
}

impl TraceController {
    pub(crate) fn initialize() {
        // Start a tracing instance just for serving.
        // This ensures that any tracing we do while serving doesn't break the TUI itself, and instead is
        // redirected to the serve process.
        // If {LOG_ENV} is set, default to env, otherwise filter to cli
        // and manganis warnings and errors from other crates
        let mut filter = EnvFilter::new("error,dx=info,devdx=info,dioxus-cli=info");

        if env::var(LOG_ENV).is_ok() {
            filter = EnvFilter::from_env(LOG_ENV);
        }

        let cli_writer = Mutex::new(Writer {
            stdout: io::stdout(),
        });

        // Build tracing
        let fmt_layer = tracing_subscriber::fmt::layer()
            .with_writer(cli_writer)
            .with_filter(filter);

        let sub = tracing_subscriber::registry().with(fmt_layer);

        #[cfg(feature = "tokio-console")]
        let sub = sub.with(console_subscriber::spawn());

        sub.init();
    }

    pub(crate) fn start() -> Self {
        // Create writer controller and custom writer.
        let (tui_tx, tui_rx) = unbounded();
        TUI_TX.set(tui_tx.clone()).unwrap();
        TUI_ENABLED.store(true, Ordering::SeqCst);

        Self { tui_rx }
    }

    /// Wait for the internal logger to send a message
    pub(crate) async fn wait(&mut self) -> ServeUpdate {
        ServeUpdate::TracingLog {
            log: self.tui_rx.next().await.expect("tracer should never die"),
        }
    }

    pub(crate) fn shutdown(&self) {
        TUI_ENABLED.store(false, Ordering::SeqCst);
    }
}

/// Represents the CLI's custom tracing writer for conditionally writing logs between outputs.
struct Writer {
    stdout: io::Stdout,
}

// Implement a conditional writer so that logs are routed to the appropriate place.
impl io::Write for Writer {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if TUI_ENABLED.load(Ordering::SeqCst) {
            let len = buf.len();

            let as_string = String::from_utf8(buf.to_vec())
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

            TUI_TX
                .get()
                .unwrap()
                .unbounded_send(as_string)
                .map_err(|e| io::Error::new(io::ErrorKind::BrokenPipe, e))?;

            Ok(len)
        } else {
            self.stdout.write(buf)
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        if !TUI_ENABLED.load(Ordering::SeqCst) {
            self.stdout.flush()
        } else {
            Ok(())
        }
    }
}
