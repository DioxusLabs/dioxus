// Adapted from tracing-wasm 0.2.1 (https://github.com/storyai/tracing-wasm).
// Copyright (c) 2020 Story.ai. Used under the MIT license in ../LICENSE-tracing-wasm.

use std::cell::{Cell, RefCell};
use std::fmt::{self, Write};
use std::sync::atomic::{AtomicUsize, Ordering};
use tracing::{
    Event, Id, Level, Metadata, Subscriber,
    field::{Field, Visit},
    span::{Attributes, Record},
};
use tracing_subscriber::{
    layer::{Context, Layer},
    registry::LookupSpan,
};
use wasm_bindgen::prelude::*;

#[cfg(test)]
mod tests;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = performance)]
    fn mark(name: &str);
    #[wasm_bindgen(catch, js_namespace = performance)]
    fn measure(name: &str, start: &str) -> Result<(), JsValue>;
    #[wasm_bindgen(js_namespace = performance, js_name = clearMarks)]
    fn clear_marks(name: &str);
    #[wasm_bindgen(js_namespace = performance, js_name = clearMeasures)]
    fn clear_measures(name: &str);
    #[wasm_bindgen(js_namespace = console, js_name = debug)]
    fn console_debug(msg: &str);
    #[wasm_bindgen(js_namespace = console, js_name = info)]
    fn console_info(msg: &str);
    #[wasm_bindgen(js_namespace = console, js_name = warn)]
    fn console_warn(msg: &str);
    #[wasm_bindgen(js_namespace = console, js_name = error)]
    fn console_error(msg: &str);
}

thread_local! {
    /// Ids of the spans entered on this thread, with one entry per enter so that
    /// each level of a recursively entered span gets its own mark.
    static ENTERED: RefCell<Vec<u64>> = const { RefCell::new(Vec::new()) };
    // Taken out while calling JS so synchronous re-entry uses a separate buffer.
    static SPAN_BUFFER: Cell<String> = const { Cell::new(String::new()) };
}

/// Writes events to level-matched console methods and records spans and events in Performance timings.
/// Event fields are formatted once and shared between both outputs.
/// Timing entries are cleared once recorded: profiler recordings and `PerformanceObserver`s see them,
/// but they are not retained for `performance.getEntries()`.
pub(crate) struct WasmLayer {
    max_level: Level,
    last_event_id: AtomicUsize,
}

impl WasmLayer {
    pub(crate) fn new(max_level: Level) -> Self {
        Self {
            max_level,
            last_event_id: AtomicUsize::new(0),
        }
    }
}

impl<S: Subscriber + for<'a> LookupSpan<'a>> Layer<S> for WasmLayer {
    fn enabled(&self, metadata: &Metadata<'_>, _ctx: Context<'_, S>) -> bool {
        metadata.level() <= &self.max_level
    }

    fn on_new_span(&self, attrs: &Attributes<'_>, id: &Id, ctx: Context<'_, S>) {
        let mut recorder = Recorder::default();
        attrs.record(&mut recorder);
        if let Some(span) = ctx.span(id) {
            span.extensions_mut().insert(recorder);
        }
    }

    fn on_record(&self, id: &Id, values: &Record<'_>, ctx: Context<'_, S>) {
        if let Some(span) = ctx.span(id)
            && let Some(recorder) = span.extensions_mut().get_mut::<Recorder>()
        {
            values.record(recorder);
        }
    }

    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let meta = event.metadata();
        let mut recorder = Recorder::default();
        event.record(&mut recorder);

        let mut line = meta.level().to_string();
        if let (Some(file), Some(ln)) = (meta.file(), meta.line()) {
            _ = write!(line, " {file}:{ln}");
        }
        _ = write!(line, "{recorder}");

        // `console.trace` prints a stack trace, so TRACE events also go to `console.debug`.
        match *meta.level() {
            Level::ERROR => console_error(&line),
            Level::WARN => console_warn(&line),
            Level::INFO => console_info(&line),
            Level::DEBUG | Level::TRACE => console_debug(&line),
        }

        let mark_name = format!("c{:x}", self.last_event_id.fetch_add(1, Ordering::Relaxed));
        mark(&mark_name);
        line.clear();
        _ = write!(
            line,
            "{} {}{recorder}",
            meta.level(),
            meta.module_path().unwrap_or("...")
        );
        _ = measure(&line, &mark_name);
        clear_marks(&mark_name);
        clear_measures(&line);
    }

    fn on_enter(&self, id: &Id, _ctx: Context<'_, S>) {
        let id = id.into_u64();
        let depth = ENTERED.with_borrow_mut(|entered| {
            let depth = entered.iter().filter(|span| **span == id).count();
            entered.push(id);
            depth
        });
        let mut buffer = SPAN_BUFFER.take();
        buffer.clear();
        _ = write!(buffer, "t{id:x}.{depth}");
        mark(&buffer);
        SPAN_BUFFER.set(buffer);
    }

    fn on_exit(&self, id: &Id, ctx: Context<'_, S>) {
        let span_id = id.into_u64();
        let depth = ENTERED.with_borrow_mut(|entered| {
            if let Some(index) = entered.iter().rposition(|span| *span == span_id) {
                entered.remove(index);
            }
            entered.iter().filter(|span| **span == span_id).count()
        });
        if let Some(span) = ctx.span(id) {
            let mut buffer = SPAN_BUFFER.take();
            buffer.clear();
            _ = write!(buffer, "t{span_id:x}.{depth}");
            let mark_name_len = buffer.len();
            let meta = span.metadata();
            _ = write!(
                buffer,
                "\"{}\" {}",
                meta.name(),
                meta.module_path().unwrap_or("...")
            );
            if let Some(recorder) = span.extensions().get::<Recorder>() {
                _ = write!(buffer, "{recorder}");
            }
            let (mark_name, name) = buffer.split_at(mark_name_len);
            _ = measure(name, mark_name);
            clear_marks(mark_name);
            clear_measures(name);
            SPAN_BUFFER.set(buffer);
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

impl fmt::Display for Recorder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if !self.message.is_empty() {
            write!(f, " {}", self.message)?;
        }
        f.write_str(&self.fields)
    }
}
