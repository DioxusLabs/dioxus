use super::config::Resolved;
use super::report::{DiscoveredTest, Outcome, Platform, TestId, TestOutcome};
use super::{TestArgs, is_harness_false};
use crate::{AppBuilder, BuildId, BuildKind, BuildMode, BuildRequest, Result};
use anyhow::{Context, bail};
use krates::cm::TargetKind;
use std::{path::PathBuf, process::Stdio, time::Instant};

/// A built test binary and the request that produced it.
pub(crate) struct TestBinary {
    pub(crate) request: BuildRequest,
    pub(crate) exe: PathBuf,
    /// `harness = false` [[test]] targets speak our JSON discovery protocol.
    pub(crate) custom_harness: bool,
}

pub(crate) struct HostSuite {
    pub(crate) binaries: Vec<TestBinary>,
}

/// Enumerate testable targets and build them with `BuildKind::Test`.
pub(crate) async fn build(args: &TestArgs, requests: &[BuildRequest]) -> Result<HostSuite> {
    let mut selected = vec![];
    for req in requests {
        for target in args.select_targets(req)? {
            let custom_harness = target.kind.contains(&TargetKind::Test)
                && is_harness_false(req.crate_dir().join("Cargo.toml"), &target.name)?;
            let mut r = req.clone();
            r.kind = BuildKind::Test;
            r.crate_target = target;
            selected.push((r, custom_harness));
        }
    }

    // Build the test binaries sequentially - they share dep artifacts with `dx build` anyway.
    //
    // The recorded exe for a bin harness lives at the profile dir (`{profile}/{name}`), which a
    // later `cargo rustc` invocation in this same loop clobbers (e.g. `--test integ` builds the
    // bin as a normal dependency and copies it over that path). Hardlink each binary into a
    // private dir so subsequent builds can't overwrite the harness underneath us.
    let stable_dir = selected
        .first()
        .map(|(r, _)| r.target_dir.join("dx").join("test-binaries"));
    if let Some(dir) = &stable_dir {
        std::fs::create_dir_all(dir).context("Failed to create test binary dir")?;
    }
    let mut binaries = vec![];
    for (idx, (req, custom_harness)) in selected.iter().enumerate() {
        tracing::debug!("Building test target {} ...", req.executable_name());
        let artifacts = AppBuilder::started(req, BuildMode::Base, BuildId::PRIMARY)?
            .finish_build()
            .await?;
        let mut exe = artifacts.exe;
        if let Some(dir) = &stable_dir {
            let stable = dir.join(format!("{idx}-{}", req.executable_name()));
            _ = std::fs::remove_file(&stable);
            if std::fs::hard_link(&exe, &stable).is_ok() {
                exe = stable;
            }
        }
        tracing::debug!("Built test binary {}", exe.display());
        binaries.push(TestBinary {
            request: req.clone(),
            exe,
            custom_harness: *custom_harness,
        });
    }
    Ok(HostSuite { binaries })
}

/// Discover tests in every built binary. Custom harness binaries get a
/// `--list --format json` probe first, falling back to terse on parse failure.
pub(crate) async fn discover(suite: &HostSuite) -> Result<Vec<DiscoveredTest>> {
    let mut discovered = vec![];
    for (idx, binary) in suite.binaries.iter().enumerate() {
        let mut tests = match binary.custom_harness {
            true => list_json(binary).await?.unwrap_or_default(),
            false => vec![],
        };
        if tests.is_empty() {
            tests = list_terse(binary)
                .await?
                .into_iter()
                .map(|name| DiscoveredTest {
                    id: test_id(binary, name),
                    binary: idx,
                    file: None,
                    line: None,
                    ignore: false,
                    should_panic: false,
                    tags: vec![],
                })
                .collect();
        } else {
            for test in &mut tests {
                test.binary = idx;
            }
        }
        discovered.extend(tests);
    }
    Ok(discovered)
}

fn test_id(binary: &TestBinary, name: String) -> TestId {
    TestId {
        package: binary.request.package().name.clone(),
        target: binary.request.executable_name().to_string(),
        platform: Platform::Host,
        name,
    }
}

/// The environment `cargo test` sets for the test binaries it spawns.
fn test_env_vars(req: &BuildRequest) -> Vec<(&'static str, String)> {
    vec![
        ("CARGO_MANIFEST_DIR", req.crate_dir().display().to_string()),
        ("CARGO_PKG_NAME", req.package().name.clone()),
        ("CARGO_PKG_VERSION", req.crate_version()),
        ("CARGO_CRATE_NAME", req.executable_name().replace('-', "_")),
    ]
}

async fn list_terse(binary: &TestBinary) -> Result<Vec<String>> {
    let output = tokio::process::Command::new(&binary.exe)
        .args(["--list", "--format", "terse"])
        .current_dir(binary.request.crate_dir())
        .envs(test_env_vars(&binary.request))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .with_context(|| format!("Failed to run {}", binary.exe.display()))?;

    if !output.status.success() {
        bail!(
            "Test binary {} failed to list tests:\n{}",
            binary.exe.display(),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    Ok(parse_test_list(&String::from_utf8_lossy(&output.stdout)))
}

/// Parse `libtest --list --format terse` output: `name: test` lines (benchmarks are ignored).
pub(super) fn parse_test_list(stdout: &str) -> Vec<String> {
    stdout
        .lines()
        .filter_map(|line| line.trim_end().strip_suffix(": test").map(str::to_string))
        .collect()
}

/// `--list --format json` on a `harness = false` target: one
/// `{"name","file","line","ignore","should_panic","tags"}` object per line.
/// Returns `Ok(None)` when the output doesn't parse (not our harness).
async fn list_json(binary: &TestBinary) -> Result<Option<Vec<DiscoveredTest>>> {
    let output = tokio::process::Command::new(&binary.exe)
        .args(["--list", "--format", "json"])
        .current_dir(binary.request.crate_dir())
        .envs(test_env_vars(&binary.request))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .with_context(|| format!("Failed to run {}", binary.exe.display()))?;

    if !output.status.success() {
        return Ok(None);
    }

    let mut tests = vec![];
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        let Ok(value) = serde_json::from_str::<serde_json::Value>(line) else {
            return Ok(None);
        };
        let Some(name) = value.get("name").and_then(serde_json::Value::as_str) else {
            return Ok(None);
        };
        tests.push(DiscoveredTest {
            id: test_id(binary, name.to_string()),
            binary: 0,
            file: value
                .get("file")
                .and_then(serde_json::Value::as_str)
                .map(str::to_string),
            line: value
                .get("line")
                .and_then(serde_json::Value::as_u64)
                .map(|line| line as u32),
            ignore: value
                .get("ignore")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or_default(),
            should_panic: value
                .get("should_panic")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or_default(),
            tags: value
                .get("tags")
                .and_then(serde_json::Value::as_array)
                .map(|tags| {
                    tags.iter()
                        .filter_map(serde_json::Value::as_str)
                        .map(str::to_string)
                        .collect()
                })
                .unwrap_or_default(),
        });
    }
    Ok(Some(tests))
}

/// Run a single test in its own process, retrying up to `retries` times.
///
/// Returns `Ok(None)` when `--ignored` filtered the test out entirely (the
/// binary ran nothing) - the test simply isn't part of this run.
pub(crate) async fn run_case(
    suite: &HostSuite,
    case: &DiscoveredTest,
    resolved: &Resolved,
) -> Result<Option<TestOutcome>> {
    let binary = &suite.binaries[case.binary];
    let started = Instant::now();
    let mut attempts = 0;
    loop {
        let mut cmd = tokio::process::Command::new(&binary.exe);
        cmd.args(["--exact", &case.id.name, "--nocapture"])
            .current_dir(binary.request.crate_dir())
            .envs(test_env_vars(&binary.request))
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if resolved.include_ignored {
            cmd.arg("--include-ignored");
        }
        if resolved.ignored {
            cmd.arg("--ignored");
        }

        let child = cmd
            .kill_on_drop(true)
            .spawn()
            .with_context(|| format!("Failed to run {}", binary.exe.display()))?;
        let output = match tokio::time::timeout(resolved.timeout, child.wait_with_output()).await {
            Ok(output) => {
                output.with_context(|| format!("Failed to run {}", binary.exe.display()))?
            }
            Err(_) => {
                attempts += 1;
                if attempts > resolved.retries + 1 {
                    return Ok(Some(TestOutcome {
                        id: case.id.clone(),
                        outcome: Outcome::TimedOut,
                        flaky: false,
                        attempts,
                        message: Some(format!(
                            "test timed out after {}s",
                            resolved.timeout.as_secs_f64()
                        )),
                        output: String::new(),
                        elapsed: started.elapsed(),
                    }));
                }
                continue;
            }
        };
        let text = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );

        // With `--ignored`, non-ignored tests run zero tests - drop them from the report.
        if resolved.ignored && output.status.success() && ran_nothing(&text) {
            return Ok(None);
        }

        let outcome = parse_run_outcome(output.status.success(), &text);
        attempts += 1;
        if outcome != Outcome::Failed || attempts > resolved.retries + 1 {
            return Ok(Some(TestOutcome {
                id: case.id.clone(),
                outcome,
                flaky: attempts > 1,
                attempts,
                message: None,
                output: text,
                elapsed: started.elapsed(),
            }));
        }
    }
}

/// The binary ran but every test was filtered out.
pub(super) fn ran_nothing(output: &str) -> bool {
    output.lines().any(|line| {
        let Some(summary) = line.trim_start().strip_prefix("test result:") else {
            return false;
        };
        summary.contains("0 passed; 0 failed; 0 ignored") && summary.contains("filtered out")
    })
}

/// Derive a test outcome from the process exit status and the libtest summary line.
pub(super) fn parse_run_outcome(success: bool, output: &str) -> Outcome {
    if !success {
        return Outcome::Failed;
    }

    for line in output.lines() {
        let Some(summary) = line.trim_start().strip_prefix("test result:") else {
            continue;
        };
        let mut passed = 0;
        let mut ignored = 0;
        let mut failed = 0;
        for part in summary.split(';') {
            let tokens = part.split_whitespace().collect::<Vec<_>>();
            for pair in tokens.windows(2) {
                let Ok(n) = pair[0].parse::<usize>() else {
                    continue;
                };
                match pair[1].trim_end_matches(',') {
                    "passed" => passed = n,
                    "failed" => failed = n,
                    "ignored" => ignored = n,
                    _ => {}
                }
            }
        }

        if failed > 0 {
            return Outcome::Failed;
        }
        if passed == 0 && ignored > 0 {
            return Outcome::Ignored;
        }
        return Outcome::Passed;
    }

    // No summary line - trust the exit code.
    Outcome::Passed
}
