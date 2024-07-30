use std::{
    env, io,
    sync::{Arc, Mutex, OnceLock},
};
use tokio::sync::mpsc::UnboundedSender;
use tracing_subscriber::{fmt::{writer::OrElse, MakeWriter}, prelude::*, EnvFilter, Layer};

const LOG_ENV: &str = "DIOXUS_LOG";

pub fn build_tracing() -> Mutex<CLIWriter> {
    // If {LOG_ENV} is set, default to env, otherwise filter to cli
    // and manganis warnings and errors from other crates
    let mut filter = EnvFilter::new("error,dx=info,dioxus-cli=info,manganis-cli-support=info");
    if env::var(LOG_ENV).is_ok() {
        filter = EnvFilter::from_env(LOG_ENV);
    }

    let cli_writer = CLIWriter::new();
    let cli_writer_clone = cli_writer.clone();

    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_writer(cli_writer_clone)
        .with_filter(filter);
    let sub = tracing_subscriber::registry().with(fmt_layer);

    #[cfg(feature = "tokio-console")]
    let sub = sub.with(console_subscriber::spawn());

    sub.init();

    cli_writer
}

pub struct CLIWriter {
    stdout: io::Stdout,
    tui_writer: Option<TUIWriter>,
}

impl CLIWriter {
    pub fn new() -> Self {
        Self {
            stdout: io::stdout(),
            tui_writer: None,
        }
    }

    pub fn enable_tui(&mut self, tx: UnboundedSender<String>) {
        self.tui_writer = Some(Mutex::new(TUIWriter(tx)));
    }

    pub fn disable_tui(&mut self) {
        self.tui_writer = None;
    }
}

impl io::Write for CLIWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if let Some(tx) = &self.tx {
            let len = buf.len();

            let as_string = String::from_utf8(buf.to_vec())
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

            tx.send(as_string)
                .map_err(|e| io::Error::new(io::ErrorKind::BrokenPipe, e))?;
            Ok(len)
        } else {
            self.stdout.write(buf)
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl<'a> MakeWriter<'a> for CLIWriter {
    type Writer = OrElse<Mutex<TUIWriter>, io::Stdout>;

    fn make_writer(&'a self) -> Self::Writer {}
}

pub struct TUIWriter(UnboundedSender<String>);

impl io::Write for TUIWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let len = buf.len();

        let as_string = String::from_utf8(buf.to_vec())
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        self.0
            .send(as_string)
            .map_err(|e| io::Error::new(io::ErrorKind::BrokenPipe, e))?;
        Ok(len)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
