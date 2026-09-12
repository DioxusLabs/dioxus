use super::test::TestArgs;
use crate::{AppBuilder, BuildId, BuildKind, BuildMode, BundleFormat, Result, WasmBindgen};
use anyhow::{Context, anyhow, bail};
use axum::{Router, extract::State, http::StatusCode, routing::post};
use serde_json::Value;
use std::{
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant},
};
use tempfile::TempDir;
use tokio::{
    process::Command,
    sync::{Mutex, mpsc},
};
use tower_http::services::ServeDir;

pub(crate) async fn run(
    args: TestArgs,
    requests: Vec<crate::BuildRequest>,
) -> Result<crate::StructuredOutput> {
    let mut selected = Vec::new();
    for req in requests {
        if req.bundle != BundleFormat::Web {
            tracing::warn!(
                "Skipping {} [{}]: dx test only supports host and web targets",
                req.package,
                req.triple
            );
            continue;
        }
        for target in select_web_targets(&args, &req)? {
            let mut request = req.clone();
            request.kind = BuildKind::Test;
            request.crate_target = target;
            selected.push(request);
        }
    }
    if selected.is_empty() {
        bail!("No web test targets selected (web tests need `[[test]] harness = false` targets)");
    }

    let browser = resolve_browser(args.browser.as_deref())?;
    let timeout = parse_duration(&args.timeout)?;
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
        binaries.push(WebBinary {
            request: request.clone(),
            dir,
        });
    }

    let mut listed = Vec::new();
    for binary in &binaries {
        let events = run_browser(binary, &browser, timeout, "list", None).await?;
        if let Some(Value::Array(tests)) = events
            .iter()
            .find_map(|event| {
                event
                    .get("event")
                    .filter(|e| e.as_str() == Some("list"))
                    .and_then(|_| event.get("tests"))
            })
            .cloned()
        {
            for test in tests {
                if let Some(name) = test.get("name").and_then(Value::as_str) {
                    if super::test::matches_filters(name, &args.filters, args.exact) {
                        listed.push((
                            binary.request.executable_name().to_string(),
                            name.to_string(),
                            test.get("should_panic")
                                .and_then(Value::as_bool)
                                .unwrap_or_default(),
                        ));
                    }
                }
            }
        }
    }
    if args.list {
        for (target, name, _) in listed {
            println!("{target}  {name}");
        }
        return Ok(crate::StructuredOutput::Success);
    }
    if args.no_run {
        for binary in binaries {
            tracing::info!("{}", binary.dir.display());
        }
        return Ok(crate::StructuredOutput::Success);
    }

    let selected = listed
        .into_iter()
        .filter_map(|(target, name, should_panic)| {
            binaries
                .iter()
                .position(|binary| binary.request.executable_name() == target)
                .map(|binary| WebCase {
                    binary,
                    name,
                    should_panic,
                })
        })
        .collect::<Vec<_>>();
    let threads = args
        .test_threads
        .or_else(|| std::thread::available_parallelism().ok().map(|n| n.get()))
        .unwrap_or(1)
        .max(1);
    let semaphore = Arc::new(tokio::sync::Semaphore::new(threads));
    let mut running = futures_util::stream::FuturesUnordered::new();
    let mut pending = selected.into_iter();
    let mut results = Vec::new();
    let started = Instant::now();
    let mut fail_fast = false;
    loop {
        tokio::select! {
            permit = semaphore.clone().acquire_owned(), if !fail_fast || args.no_fail_fast => {
                let permit = permit.context("web test semaphore closed")?;
                let Some(case) = pending.next() else { drop(permit); continue };
                let binary = binaries[case.binary].clone();
                let browser = browser.clone();
                running.push(async move { let _permit = permit; run_one(case, binary, browser, timeout).await });
            }
            Some(result) = futures_util::StreamExt::next(&mut running) => {
                let result = result?;
                if result.outcome == WebOutcome::Failed { fail_fast = true; }
                println!("{} [{:>8.3}s] {}", result.label(), result.elapsed.as_secs_f64(), result.name);
                if result.outcome == WebOutcome::Failed { println!("    {}", result.message.as_deref().unwrap_or("test failed")); }
                results.push(result);
            }
            else => break,
        }
    }
    let failed = results
        .iter()
        .filter(|result| result.outcome == WebOutcome::Failed)
        .count();
    let passed = results
        .iter()
        .filter(|result| result.outcome == WebOutcome::Passed)
        .count();
    let skipped = results
        .iter()
        .filter(|result| result.outcome == WebOutcome::Ignored)
        .count();
    println!(
        "Summary [{:>8.3}s] {} tests run: {passed} passed, {failed} failed, {skipped} skipped",
        started.elapsed().as_secs_f64(),
        results.len()
    );
    if failed > 0 {
        return Err(anyhow!("{failed} tests failed"));
    }
    Ok(crate::StructuredOutput::Success)
}

#[derive(Clone)]
struct WebBinary {
    request: crate::BuildRequest,
    dir: PathBuf,
}
struct WebCase {
    binary: usize,
    name: String,
    should_panic: bool,
}
#[derive(PartialEq, Eq)]
enum WebOutcome {
    Passed,
    Failed,
    Ignored,
}
struct WebResult {
    name: String,
    outcome: WebOutcome,
    message: Option<String>,
    elapsed: Duration,
}
impl WebResult {
    fn label(&self) -> console::StyledObject<&'static str> {
        match self.outcome {
            WebOutcome::Passed => console::style("PASS").green(),
            WebOutcome::Failed => console::style("FAIL").red(),
            WebOutcome::Ignored => console::style("SKIP").yellow(),
        }
    }
}

async fn run_one(
    case: WebCase,
    binary: WebBinary,
    browser: String,
    timeout: Duration,
) -> Result<WebResult> {
    let started = Instant::now();
    let events = run_browser(&binary, &browser, timeout, "test", Some(&case.name)).await?;
    let event = events.iter().find(|event| {
        event.get("event").and_then(Value::as_str) == Some("finished")
            && event.get("name").and_then(Value::as_str) == Some(case.name.as_str())
    });
    let message = event
        .and_then(|e| e.get("message"))
        .and_then(Value::as_str)
        .map(str::to_string);
    let (outcome, message) = match (
        event
            .and_then(|event| event.get("outcome"))
            .and_then(Value::as_str),
        case.should_panic,
    ) {
        (Some("ok"), true) => (WebOutcome::Failed, Some("test did not panic".to_string())),
        (Some("ok"), false) | (Some("failed"), true) => (WebOutcome::Passed, None),
        (Some("ignored"), _) => (WebOutcome::Ignored, None),
        _ => (WebOutcome::Failed, message),
    };
    Ok(WebResult {
        name: case.name,
        outcome,
        message,
        elapsed: started.elapsed(),
    })
}

async fn run_browser(
    binary: &WebBinary,
    browser: &str,
    timeout: Duration,
    mode: &str,
    test: Option<&str>,
) -> Result<Vec<Value>> {
    let dir = TempDir::new().context("failed to create Chrome profile")?;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let events = Arc::new(Mutex::new(Vec::new()));
    let (tx, mut rx) = mpsc::unbounded_channel::<Value>();
    let state = Arc::new(tx);
    let app = Router::new()
        .route("/__dx_test/event", post(receive_event))
        .fallback_service(ServeDir::new(binary.dir.clone()))
        .with_state(state);
    let server = tokio::spawn(async move { axum::serve(listener, app).await });
    let query = match (mode, test) {
        ("list", _) => "?list=1".to_string(),
        (_, Some(test)) => format!("?test={}", encode_query(test)),
        _ => String::new(),
    };
    let url = format!("http://{address}/{query}");
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
    let deadline = tokio::time::sleep(timeout);
    tokio::pin!(deadline);
    loop {
        tokio::select! {
            event = rx.recv() => if let Some(event) = event {
                let done = event.get("event").and_then(Value::as_str) == Some("done");
                events.lock().await.push(event);
                if done { break; }
            } else { break; },
            _ = &mut deadline => { child.kill().await.ok(); server.abort(); return Ok(vec![serde_json::json!({"event":"finished", "outcome":"failed", "name":test.unwrap_or("unknown"), "message":format!("timed out after {}s", timeout.as_secs())})]); }
        }
    }
    child.kill().await.ok();
    server.abort();
    Ok(events.lock().await.clone())
}

async fn receive_event(
    State(sender): State<Arc<mpsc::UnboundedSender<Value>>>,
    body: String,
) -> StatusCode {
    if let Ok(value) = serde_json::from_str::<Value>(&body) {
        let _ = sender.send(value);
    }
    StatusCode::NO_CONTENT
}

fn select_web_targets(
    args: &TestArgs,
    req: &crate::BuildRequest,
) -> Result<Vec<krates::cm::Target>> {
    let selected = args.select_targets(req)?;
    let mut output = Vec::new();
    for target in selected {
        if !is_harness_false(req.crate_dir().join("Cargo.toml"), &target.name)? {
            tracing::warn!(
                "Skipping {}: web tests need `harness = false` and dioxus-test-harness",
                target.name
            );
            continue;
        }
        output.push(target);
    }
    Ok(output)
}

fn is_harness_false(manifest: PathBuf, name: &str) -> Result<bool> {
    let text = std::fs::read_to_string(manifest)?;
    let value: toml::Value = toml::from_str(&text)?;
    Ok(value
        .get("test")
        .and_then(toml::Value::as_array)
        .is_some_and(|tests| {
            tests.iter().any(|test| {
                test.get("name").and_then(toml::Value::as_str) == Some(name)
                    && test.get("harness").and_then(toml::Value::as_bool) == Some(false)
            })
        }))
}

fn resolve_browser(explicit: Option<&str>) -> Result<String> {
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

pub(crate) fn parse_duration(value: &str) -> Result<Duration> {
    let (number, multiplier) = if let Some(n) = value.strip_suffix("ms") {
        (n, 1)
    } else if let Some(n) = value.strip_suffix('s') {
        (n, 1_000)
    } else if let Some(n) = value.strip_suffix('m') {
        (n, 60_000)
    } else {
        (value, 1_000)
    };
    let millis = number
        .parse::<u64>()
        .with_context(|| format!("invalid timeout `{value}`"))?
        .checked_mul(multiplier)
        .context("timeout overflow")?;
    Ok(Duration::from_millis(millis))
}

const INDEX_HTML: &str = r#"<!doctype html><html><body><script type="module">
import init from "./test.js";
const post = (o) => fetch("/__dx_test/event", {method:"POST", body: JSON.stringify(o), keepalive:true});
const failed = (reason) => post({event:"finished", name:new URL(location).searchParams.get("test") ?? "unknown", outcome:"failed", message:globalThis.__dx_panic_message ?? String(reason), exec_time:0}).then(() => post({event:"done"}));
window.addEventListener("error", (e) => failed(e.error ?? e.message));
window.addEventListener("unhandledrejection", (e) => failed(e.reason));
try { await init(); } catch (e) { await failed(e); }
</script></body></html>"#;
