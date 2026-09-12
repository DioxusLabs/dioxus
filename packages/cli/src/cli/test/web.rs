use super::config::Resolved;
use super::report::{DiscoveredTest, Outcome, Platform, TestId, TestOutcome};
use super::{TestArgs, is_harness_false};
use crate::{AppBuilder, BuildId, BuildKind, BuildMode, BuildRequest, Result, WasmBindgen};
use anyhow::{Context, bail};
use axum::{Router, extract::State, http::StatusCode, routing::post};
use serde_json::Value;
use std::{collections::HashMap, path::PathBuf, sync::Arc, time::Instant};
use tempfile::TempDir;
use tokio::{process::Command, sync::Mutex, sync::mpsc};
use tower_http::services::ServeDir;

/// A channel key used for the `?list=1` discovery page.
const LIST_KEY: &str = "__list";

pub(crate) struct WebSuite {
    pub(crate) binaries: Vec<WebBinary>,
}

pub(crate) struct WebBinary {
    pub(crate) request: BuildRequest,
    pub(crate) dir: PathBuf,
    server: WebServer,
}

struct WebServer {
    address: std::net::SocketAddr,
    routes: Arc<Routes>,
    task: tokio::task::JoinHandle<()>,
}

/// Routes browser events to the waiting test. `started`/`finished` carry `name`;
/// `done` and `list` go to the `LIST_KEY` channel used by the discovery page.
#[derive(Default)]
struct Routes {
    senders: Mutex<HashMap<String, mpsc::UnboundedSender<Value>>>,
}

/// Select `harness = false` [[test]] targets, build them for wasm32, run
/// wasm-bindgen, and start one event server per binary.
pub(crate) async fn build(args: &TestArgs, requests: &[BuildRequest]) -> Result<WebSuite> {
    let mut selected = Vec::new();
    for req in requests {
        for target in args.select_targets(req)? {
            if !is_harness_false(req.crate_dir().join("Cargo.toml"), &target.name)? {
                tracing::warn!(
                    "Skipping {}: web tests need `harness = false` and dioxus-test-harness",
                    target.name
                );
                continue;
            }
            let mut request = req.clone();
            request.kind = BuildKind::Test;
            request.crate_target = target;
            selected.push(request);
        }
    }

    let mut binaries = Vec::new();
    for request in &selected {
        let artifacts = AppBuilder::started(request, BuildMode::Base, BuildId::PRIMARY)?
            .finish_build()
            .await?;
        let dir = request
            .target_dir
            .join("dx")
            .join(&request.main_target)
            .join("test")
            .join("web")
            .join(request.executable_name());
        std::fs::create_dir_all(&dir)?;
        let version = request
            .workspace
            .wasm_bindgen_version()
            .context("failed to detect wasm-bindgen version")?;
        WasmBindgen::verify_install(&version).await?;
        WasmBindgen::new(&version)
            .input_path(&artifacts.exe)
            .target("web")
            .out_dir(&dir)
            .out_name("test")
            .debug(true)
            .keep_debug(true)
            .run()
            .await?;
        std::fs::write(dir.join("index.html"), INDEX_HTML)?;
        let server = serve(&dir).await?;
        binaries.push(WebBinary {
            request: request.clone(),
            dir,
            server,
        });
    }
    Ok(WebSuite { binaries })
}

/// Start the static file + event server for one binary.
async fn serve(dir: &std::path::Path) -> Result<WebServer> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let routes = Arc::new(Routes::default());
    let app = Router::new()
        .route("/__dx_test/event", post(receive_event))
        .fallback_service(ServeDir::new(dir))
        .with_state(routes.clone());
    let task = tokio::spawn(async move {
        _ = axum::serve(listener, app).await;
    });
    Ok(WebServer {
        address,
        routes,
        task,
    })
}

async fn receive_event(State(routes): State<Arc<Routes>>, body: String) -> StatusCode {
    let Ok(event) = serde_json::from_str::<Value>(&body) else {
        return StatusCode::NO_CONTENT;
    };
    let kind = event.get("event").and_then(Value::as_str).unwrap_or("");
    let key = match kind {
        "list" | "done" => Some(LIST_KEY.to_string()),
        _ => event
            .get("name")
            .and_then(Value::as_str)
            .map(str::to_string),
    };
    let Some(key) = key else {
        return StatusCode::NO_CONTENT;
    };
    let sender = routes.senders.lock().await.get(&key).cloned();
    if let Some(sender) = sender {
        _ = sender.send(event);
    }
    StatusCode::NO_CONTENT
}

impl Drop for WebServer {
    fn drop(&mut self) {
        self.task.abort();
    }
}

/// Launch headless chrome against `query` and collect events until `done` or timeout.
async fn browse(
    binary: &WebBinary,
    browser: &str,
    resolved: &Resolved,
    key: &str,
    query: String,
) -> Vec<Value> {
    let (tx, mut rx) = mpsc::unbounded_channel::<Value>();
    binary
        .server
        .routes
        .senders
        .lock()
        .await
        .insert(key.to_string(), tx);

    let events = async {
        let mut events = Vec::new();
        let url = format!("http://{}/{query}", binary.server.address);
        let dir = TempDir::new().context("failed to create Chrome profile")?;
        let mut child = Command::new(browser)
            .args([
                "--headless=new",
                "--disable-gpu",
                "--no-sandbox",
                "--no-first-run",
                "--disable-extensions",
            ])
            .arg(format!("--user-data-dir={}", dir.path().display()))
            .arg(url)
            .kill_on_drop(true)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()?;
        let deadline = tokio::time::sleep(resolved.timeout);
        tokio::pin!(deadline);
        loop {
            tokio::select! {
                event = rx.recv() => match event {
                    Some(event) => {
                        let kind = event.get("event").and_then(Value::as_str);
                        // `done` ends the discovery page; `finished` for our
                        // key ends a single-test page (`done` isn't routable
                        // per-test when pages run concurrently on one server).
                        let done = kind == Some("done")
                            || (kind == Some("finished")
                                && event.get("name").and_then(Value::as_str) == Some(key));
                        events.push(event);
                        if done { break; }
                    }
                    None => break,
                },
                _ = &mut deadline => break,
            }
        }
        child.kill().await.ok();
        Ok::<_, anyhow::Error>(events)
    }
    .await;

    binary.server.routes.senders.lock().await.remove(key);
    events.unwrap_or_default()
}

/// `?list=1` the binary and collect `DiscoveredTest`s from the `list` event.
pub(crate) async fn discover(
    suite: &WebSuite,
    browser: &str,
    resolved: &Resolved,
) -> Result<Vec<DiscoveredTest>> {
    let mut discovered = vec![];
    for (idx, binary) in suite.binaries.iter().enumerate() {
        let events = browse(binary, browser, resolved, LIST_KEY, "?list=1".to_string()).await;
        let Some(tests) = events.iter().find_map(|event| {
            (event.get("event").and_then(Value::as_str) == Some("list"))
                .then(|| event.get("tests").and_then(Value::as_array))
                .flatten()
        }) else {
            continue;
        };
        for test in tests {
            let Some(name) = test.get("name").and_then(Value::as_str) else {
                continue;
            };
            discovered.push(DiscoveredTest {
                id: TestId {
                    package: binary.request.package().name.clone(),
                    target: binary.request.executable_name().to_string(),
                    platform: Platform::Web,
                    name: name.to_string(),
                },
                binary: idx,
                file: test.get("file").and_then(Value::as_str).map(str::to_string),
                line: test.get("line").and_then(Value::as_u64).map(|l| l as u32),
                ignore: test
                    .get("ignore")
                    .and_then(Value::as_bool)
                    .unwrap_or_default(),
                should_panic: test
                    .get("should_panic")
                    .and_then(Value::as_bool)
                    .unwrap_or_default(),
                tags: test
                    .get("tags")
                    .and_then(Value::as_array)
                    .map(|tags| {
                        tags.iter()
                            .filter_map(Value::as_str)
                            .map(str::to_string)
                            .collect()
                    })
                    .unwrap_or_default(),
            });
        }
    }
    Ok(discovered)
}

/// Run one test in a fresh chrome page (`?test=<name>`) and translate its events.
pub(crate) async fn run_case(
    suite: &WebSuite,
    case: &DiscoveredTest,
    browser: &str,
    resolved: &Resolved,
) -> Result<Option<TestOutcome>> {
    let binary = &suite.binaries[case.binary];
    let started = Instant::now();
    let mut query = format!("?test={}", encode_query(&case.id.name));
    if resolved.include_ignored || resolved.ignored {
        query += "&include_ignored=1";
    }
    let events = browse(binary, browser, resolved, &case.id.name, query).await;

    let finished = events.iter().find(|event| {
        event.get("event").and_then(Value::as_str) == Some("finished")
            && event.get("name").and_then(Value::as_str) == Some(case.id.name.as_str())
    });

    let (outcome, message) = match finished {
        None => (
            Outcome::TimedOut,
            Some(format!(
                "timed out after {}s",
                resolved.timeout.as_secs_f64()
            )),
        ),
        Some(event) => {
            let message = event
                .get("message")
                .and_then(Value::as_str)
                .map(str::to_string);
            match (
                event.get("outcome").and_then(Value::as_str),
                case.should_panic,
            ) {
                (Some("ok"), true) => (Outcome::Failed, Some("test did not panic".to_string())),
                (Some("ok"), false) | (Some("failed"), true) => (Outcome::Passed, None),
                (Some("ignored"), _) => (Outcome::Ignored, None),
                _ => (Outcome::Failed, message),
            }
        }
    };

    Ok(Some(TestOutcome {
        id: case.id.clone(),
        outcome,
        flaky: false,
        attempts: 1,
        output: message.clone().unwrap_or_default(),
        message,
        elapsed: started.elapsed(),
    }))
}

pub(crate) fn resolve_browser(explicit: Option<&str>) -> Result<String> {
    if let Some(path) = explicit {
        return Ok(path.to_string());
    }
    if let Ok(path) = std::env::var("DX_TEST_BROWSER") {
        return Ok(path);
    }
    for candidate in [
        "google-chrome",
        "google-chrome-stable",
        "chromium",
        "chromium-browser",
        "chrome",
        "msedge",
    ] {
        if let Ok(path) = which::which(candidate) {
            return Ok(path.display().to_string());
        }
    }
    bail!("No browser found; pass --browser <path> or set DX_TEST_BROWSER")
}

fn encode_query(value: &str) -> String {
    value
        .bytes()
        .map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (byte as char).to_string()
            }
            byte => format!("%{byte:02X}"),
        })
        .collect()
}

const INDEX_HTML: &str = r#"<!doctype html><html><body><script type="module">
import init from "./test.js";
const post = (o) => fetch("/__dx_test/event", {method:"POST", body: JSON.stringify(o), keepalive:true});
const failed = (reason) => post({event:"finished", name:new URL(location).searchParams.get("test") ?? "unknown", outcome:"failed", message:globalThis.__dx_panic_message ?? String(reason), exec_time:0}).then(() => post({event:"done"}));
window.addEventListener("error", (e) => failed(e.error ?? e.message));
window.addEventListener("unhandledrejection", (e) => failed(e.reason));
try { await init(); } catch (e) { await failed(e); }
</script></body></html>"#;
