use super::TestCase;
use serde::Serialize;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::spawn_local;
use web_sys::{Request, RequestInit, window};

#[derive(Serialize)]
struct TestInfo {
    name: String,
    file: &'static str,
    line: u32,
    ignore: bool,
    should_panic: bool,
    tags: &'static [&'static str],
    platforms: &'static [&'static str],
    runnable: bool,
}

pub fn run() {
    install_panic_hook();
    spawn_local(async move {
        let query = window()
            .and_then(|window| window.location().search().ok())
            .unwrap_or_default();
        let query = query.trim_start_matches('?');
        let params = web_sys::UrlSearchParams::new_with_str(query).ok();
        let list = params.as_ref().is_some_and(|p| p.get("list").is_some());
        let requested = params.as_ref().and_then(|p| p.get("test"));
        let include_ignored = params
            .as_ref()
            .is_some_and(|p| p.get("include_ignored").is_some());
        let cases = inventory::iter::<TestCase>.into_iter().collect::<Vec<_>>();

        if list {
            let tests = cases
                .iter()
                .map(|case| TestInfo {
                    name: display_name(case.name).to_string(),
                    file: case.file,
                    line: case.line,
                    ignore: case.ignore,
                    should_panic: case.should_panic,
                    tags: case.tags,
                    platforms: case.platforms,
                    runnable: case.run.is_some(),
                })
                .collect::<Vec<_>>();
            post(Event::List { tests }).await;
            post(Event::Done).await;
            return;
        }

        for case in cases {
            let name = display_name(case.name).to_string();
            if requested
                .as_deref()
                .is_some_and(|requested| requested != name)
            {
                continue;
            }
            if case.run.is_none() {
                post(Event::Finished {
                    name,
                    outcome: "ignored",
                    message: Some("not runnable on web".to_string()),
                    exec_time: 0.0,
                })
                .await;
                continue;
            }
            if case.ignore && !include_ignored {
                post(Event::Finished {
                    name,
                    outcome: "ignored",
                    message: None,
                    exec_time: 0.0,
                })
                .await;
                continue;
            }
            post(Event::Started { name: name.clone() }).await;
            let start = performance_now();
            // The async test closure is polled by a local task so both sync and async tests work.
            let future = (case.run.expect("not runnable on web"))();
            future.await;
            post(Event::Finished {
                name,
                outcome: "ok",
                message: None,
                exec_time: (performance_now() - start) / 1000.0,
            })
            .await;
        }
        post(Event::Done).await;
    });
}

#[derive(Serialize)]
#[serde(tag = "event")]
enum Event {
    #[serde(rename = "list")]
    List { tests: Vec<TestInfo> },
    #[serde(rename = "started")]
    Started { name: String },
    #[serde(rename = "finished")]
    Finished {
        name: String,
        outcome: &'static str,
        message: Option<String>,
        exec_time: f64,
    },
    #[serde(rename = "done")]
    Done,
}

async fn post(event: Event) {
    let Ok(body) = serde_json::to_string(&event) else {
        return;
    };
    let Some(window) = window() else { return };
    let init = RequestInit::new();
    init.set_method("POST");
    init.set_body(&JsValue::from_str(&body));
    let Ok(request) = Request::new_with_str_and_init("/__dx_test/event", &init) else {
        return;
    };
    let _ = request.headers().set("Content-Type", "application/json");
    let promise = window.fetch_with_request(&request);
    let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
}

/// Wasm panics trap and reject the `spawn_local` promise, which the JS shim
/// reports via its `unhandledrejection`/`error` listeners. This hook stashes the
/// real panic message on `globalThis.__dx_panic_message` for the shim to read.
fn install_panic_hook() {
    std::panic::set_hook(Box::new(|info| {
        let message = info
            .payload()
            .downcast_ref::<&str>()
            .map(|s| s.to_string())
            .or_else(|| info.payload().downcast_ref::<String>().cloned())
            .unwrap_or_else(|| "test panicked".to_string());
        let global = js_sys::global();
        let _ = js_sys::Reflect::set(
            &global,
            &JsValue::from_str("__dx_panic_message"),
            &JsValue::from_str(&message),
        );
        web_sys::console::error_1(&JsValue::from_str(&message));
    }));
}

fn performance_now() -> f64 {
    window()
        .map(|window| window.performance().map(|p| p.now()).unwrap_or_default())
        .unwrap_or_default()
}

fn display_name(name: &str) -> &str {
    name.split_once("::").map(|(_, rest)| rest).unwrap_or(name)
}
