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
        const names = new Set([...captured.marks, ...captured.measures.map(entry => entry.name)]);
        captured.retained = performance.getEntries().filter(entry => names.has(entry.name)).length;
        restore = undefined;
        return JSON.stringify(captured);
    };
}

export function stop_capture() {
    return restore();
}

export function reenter_on_performance(method, callback) {
    const original = performance[method];
    performance[method] = function(...args) {
        performance[method] = original;
        callback();
        return original.apply(performance, args);
    };
}
"#)]
extern "C" {
    fn start_capture();
    fn stop_capture() -> String;
    fn reenter_on_performance(method: &str, callback: &JsValue);
}

fn capture(level: Level, f: impl FnOnce()) -> Value {
    start_capture();
    tracing::subscriber::with_default(Registry::default().with(WasmLayer::new(level)), f);
    let captured: Value = serde_json::from_str(&stop_capture()).unwrap();
    assert_eq!(captured["retained"], 0);
    captured
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
        tracing::info!("literal %c %s %d %o %O %f %%");
    });
    for (i, suffix) in [
        " explicit count=3 empty=\"\"",
        " type=\"example\" ratio=1.5",
        " display percent=\"100%c\"",
        " literal %c %s %d %o %O %f %%",
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

#[wasm_bindgen_test]
fn measures_each_level_of_a_recursively_entered_span() {
    let captured = capture(Level::INFO, || {
        let span = tracing::info_span!("recursive");
        span.in_scope(|| span.in_scope(|| {}));
    });
    let marks = captured["marks"].as_array().unwrap();
    let measures = captured["measures"].as_array().unwrap();
    assert_eq!(marks.len(), 2);
    assert_eq!(measures.len(), 2);
    assert_ne!(marks[0], marks[1]);
    assert_eq!(measures[0]["start"], marks[1]);
    assert_eq!(measures[1]["start"], marks[0]);
    assert!(measures[1]["duration"].as_f64().unwrap() >= measures[0]["duration"].as_f64().unwrap());
}

#[wasm_bindgen_test]
fn reuses_span_buffer_for_marks_and_labels() {
    let buffer = String::with_capacity(1024);
    let pointer = buffer.as_ptr();
    let capacity = buffer.capacity();
    SPAN_BUFFER.set(buffer);
    let captured = capture(Level::INFO, || {
        let long = tracing::info_span!("再利用", user = "世界");
        let short = tracing::info_span!("short");
        for span in [&long, &short, &long] {
            span.in_scope(|| {});
            let buffer = SPAN_BUFFER.take();
            assert_eq!(buffer.as_ptr(), pointer);
            assert_eq!(buffer.capacity(), capacity);
            SPAN_BUFFER.set(buffer);
        }
    });
    let measures = captured["measures"].as_array().unwrap();
    assert_eq!(measures.len(), 3);
    assert_eq!(
        measures[0]["name"],
        format!("\"再利用\" {} user=\"世界\"", module_path!())
    );
    assert_eq!(measures[1]["name"], format!("\"short\" {}", module_path!()));
    assert_eq!(measures[2]["name"], measures[0]["name"]);
}

#[wasm_bindgen_test]
fn preserves_span_buffer_during_synchronous_js_reentry() {
    for method in ["mark", "measure", "clearMarks", "clearMeasures"] {
        let captured = capture(Level::INFO, || {
            let outer = tracing::info_span!("outer", user = "世界");
            let inner = tracing::info_span!("inner");
            let callback = Closure::<dyn FnMut()>::new(move || inner.in_scope(|| {}));
            reenter_on_performance(method, callback.as_ref());
            outer.in_scope(|| {});
        });
        let marks = captured["marks"].as_array().unwrap();
        let measures = captured["measures"].as_array().unwrap();
        assert_eq!(marks.len(), 2);
        assert_eq!(measures.len(), 2);
        assert_ne!(marks[0], marks[1]);
        for entry in measures {
            assert!(marks.contains(&entry["start"]));
        }
        for name in [
            format!("\"outer\" {} user=\"世界\"", module_path!()),
            format!("\"inner\" {}", module_path!()),
        ] {
            assert!(measures.iter().any(|entry| entry["name"] == name));
        }
    }
}
