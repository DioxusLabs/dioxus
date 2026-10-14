use std::fmt::{self, Write};
use tracing::{
    Event, Level, Metadata, Subscriber,
    field::{Field, Visit},
};
use tracing_subscriber::layer::{Context, Layer};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console, js_name = debug)]
    fn console_debug(msg: &str);
    #[wasm_bindgen(js_namespace = console, js_name = info)]
    fn console_info(msg: &str);
    #[wasm_bindgen(js_namespace = console, js_name = warn)]
    fn console_warn(msg: &str);
    #[wasm_bindgen(js_namespace = console, js_name = error)]
    fn console_error(msg: &str);
}

/// Writes tracing events to the browser console using the console method that matches the event's
/// level, so the browser's level filters and `dx serve`'s log forwarding both see the real level.
///
/// `tracing-wasm` sends every event through `console.log`, which `dx` can't tell apart from INFO.
pub(crate) struct ConsoleLayer {
    max_level: Level,
}

impl ConsoleLayer {
    pub(crate) fn new(max_level: Level) -> Self {
        Self { max_level }
    }
}

impl<S: Subscriber> Layer<S> for ConsoleLayer {
    fn enabled(&self, metadata: &Metadata<'_>, _ctx: Context<'_, S>) -> bool {
        metadata.level() <= &self.max_level
    }

    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let meta = event.metadata();
        let mut recorder = Recorder::default();
        event.record(&mut recorder);

        let mut line = meta.level().to_string();
        if let (Some(file), Some(ln)) = (meta.file(), meta.line()) {
            _ = write!(line, " {file}:{ln}");
        }
        _ = write!(line, " {}{}", recorder.message, recorder.fields);

        // `console.trace` prints a stack trace, so TRACE events also go to `console.debug`.
        match *meta.level() {
            Level::ERROR => console_error(&line),
            Level::WARN => console_warn(&line),
            Level::INFO => console_info(&line),
            Level::DEBUG | Level::TRACE => console_debug(&line),
        }
    }
}

#[derive(Default)]
struct Recorder {
    message: String,
    fields: String,
}

impl Visit for Recorder {
    fn record_str(&mut self, field: &Field, value: &str) {
        if field.name() == "message" {
            self.message.push_str(value);
        } else {
            self.record_debug(field, &value);
        }
    }

    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        if field.name() == "message" {
            _ = write!(self.message, "{value:?}");
        } else {
            _ = write!(self.fields, " {}={value:?}", field.name());
        }
    }
}
