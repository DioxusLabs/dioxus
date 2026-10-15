use super::*;
use serde_json::Value;
use std::cell::Cell;
use tracing_subscriber::{Registry, layer::SubscriberExt};
use wasm_bindgen_test::wasm_bindgen_test;

#[wasm_bindgen(inline_js = r#"
let restore;

export function start_capture() {
    if (restore) restore();
    const captured = { logs: [], marks: [], measures: [] };
    const methods = ["log", "debug", "info", "warn", "error", "trace"];
    const originals = methods.map(method => console[method]);
    const mark = performance.mark;
    const measure = performance.measure;
    for (const method of methods) {
        console[method] = (...args) => captured.logs.push([method, ...args]);
    }
    performance.mark = function(name) {
        captured.marks.push(name);
        return mark.call(performance, name);
    };
    performance.measure = function(name, start) {
        const entry = measure.call(performance, name, start);
        captured.measures.push({ name, start, duration: entry.duration });
        return entry;
    };
    restore = () => {
        methods.forEach((method, i) => console[method] = originals[i]);
        performance.mark = mark;
        performance.measure = measure;
        for (const name of captured.marks) performance.clearMarks(name);
        for (const entry of captured.measures) performance.clearMeasures(entry.name);
        restore = undefined;
        return JSON.stringify(captured);
    };
}

export function stop_capture() {
    return restore();
}
"#)]
extern "C" {
    fn start_capture();
    fn stop_capture() -> String;
}

fn capture(level: Level, f: impl FnOnce()) -> Value {
    start_capture();
    tracing::subscriber::with_default(Registry::default().with(WasmLayer::new(level)), f);
    serde_json::from_str(&stop_capture()).unwrap()
}

#[wasm_bindgen_test]
fn routes_and_filters_all_levels() {
    let methods = ["error", "warn", "info", "debug", "debug"];
    for (index, level) in [
        Level::ERROR,
        Level::WARN,
        Level::INFO,
        Level::DEBUG,
        Level::TRACE,
    ]
    .into_iter()
    .enumerate()
    {
        let captured = capture(level, || {
            tracing::error!("error");
            tracing::warn!("warn");
            tracing::info!("info");
            tracing::debug!("debug");
            tracing::trace!("trace");
        });
        let logs = captured["logs"].as_array().unwrap();
        assert_eq!(logs.len(), index + 1);
        assert_eq!(captured["marks"].as_array().unwrap().len(), index + 1);
        let measures = captured["measures"].as_array().unwrap();
        assert_eq!(measures.len(), index + 1);
        for (i, log) in logs.iter().enumerate() {
            assert_eq!(log[0], methods[i]);
            assert_eq!(log.as_array().unwrap().len(), 2);
            assert!(!log[1].as_str().unwrap().contains("%c"));
            assert_eq!(measures[i]["start"], format!("c{i:x}"));
            assert!(measures[i]["duration"].as_f64().unwrap() >= 0.0);
        }
    }
}

struct Counted<'a>(&'a Cell<usize>);

impl fmt::Debug for Counted<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.set(self.0.get() + 1);
        f.write_str("formatted-once")
    }
}

#[wasm_bindgen_test]
fn shares_formatted_fields_and_skips_disabled_values() {
    let calls = Cell::new(0);
    let value = Counted(&calls);
    let captured = capture(Level::INFO, || {
        tracing::debug!(?value, "disabled");
        tracing::debug_span!("disabled", ?value).in_scope(|| {});
        tracing::info!(
            ?value,
            user = "nico",
            count = 3,
            ok = true,
            "hello {}\n世界",
            7
        );
    });
    assert_eq!(calls.get(), 1);
    let logs = captured["logs"].as_array().unwrap();
    let measures = captured["measures"].as_array().unwrap();
    assert_eq!(logs.len(), 1);
    assert_eq!(measures.len(), 1);
    let fields = "hello 7\n世界 value=formatted-once user=\"nico\" count=3 ok=true";
    assert!(logs[0][1].as_str().unwrap().ends_with(fields));
    assert!(measures[0]["name"].as_str().unwrap().ends_with(fields));
    assert!(logs[0][1].as_str().unwrap().starts_with("INFO "));
    assert!(
        measures[0]["name"]
            .as_str()
            .unwrap()
            .starts_with(&format!("INFO {} ", module_path!()))
    );
}

#[wasm_bindgen_test]
fn formats_explicit_messages_and_message_less_events() {
    let captured = capture(Level::INFO, || {
        tracing::info!(count = 3, message = "explicit", empty = "");
        tracing::info!(r#type = "example", ratio = 1.5);
        tracing::info!(message = %"display", percent = "100%c");
    });
    for (i, suffix) in [
        " explicit count=3 empty=\"\"",
        " type=\"example\" ratio=1.5",
        " display percent=\"100%c\"",
    ]
    .into_iter()
    .enumerate()
    {
        assert!(captured["logs"][i][1].as_str().unwrap().ends_with(suffix));
        assert!(
            captured["measures"][i]["name"]
                .as_str()
                .unwrap()
                .ends_with(suffix)
        );
    }
}

#[wasm_bindgen_test]
fn retains_span_fields_updates_nesting_and_reentry() {
    let captured = capture(Level::INFO, || {
        let span = tracing::info_span!("outer", user = "nico", answer = tracing::field::Empty);
        span.record("answer", 42);
        span.in_scope(|| {
            tracing::info_span!("inner").in_scope(|| {});
        });
        span.in_scope(|| {});
    });
    assert!(captured["logs"].as_array().unwrap().is_empty());
    let marks = captured["marks"].as_array().unwrap();
    let measures = captured["measures"].as_array().unwrap();
    assert_eq!(marks.len(), 3);
    assert_eq!(measures.len(), 3);
    assert_eq!(marks[0], marks[2]);
    assert_ne!(marks[0], marks[1]);
    assert!(
        marks
            .iter()
            .all(|mark| mark.as_str().unwrap().starts_with('t'))
    );
    assert_eq!(measures[0]["start"], marks[1]);
    assert_eq!(measures[1]["start"], marks[0]);
    assert_eq!(measures[2]["start"], marks[2]);
    assert_eq!(measures[0]["name"], format!("\"inner\" {}", module_path!()));
    for entry in &measures[1..] {
        assert_eq!(
            entry["name"],
            format!("\"outer\" {} user=\"nico\" answer=42", module_path!())
        );
        assert!(entry["duration"].as_f64().unwrap() >= 0.0);
    }
}
