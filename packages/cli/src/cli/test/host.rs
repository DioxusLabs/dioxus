use super::config::Resolved;
use super::report::{DiscoveredTest, Outcome, Platform, TestId, TestOutcome};
use super::{TestArgs, is_harness_false};
use crate::{AppBuilder, BuildId, BuildKind, BuildMode, BuildRequest, BundleFormat, Result};
use anyhow::{Context, bail};
use krates::cm::TargetKind;
use std::{
    path::{Path, PathBuf},
    process::Stdio,
    time::Instant,
};
use tokio::process::Command;

/// How a built test binary is executed: directly on the host, pushed to an
/// Android device over adb, or spawned inside an iOS simulator.
#[derive(Clone)]
pub(crate) enum Executor {
    Local,
    Adb {
        adb: PathBuf,
        /// adb `-t` transport id pinning the device (from `--device`).
        transport_id: Option<String>,
        /// Remote dir binaries are pushed to.
        remote_dir: String,
    },
    /// `xcrun simctl spawn <device>` - `booted` or a simulator uuid.
    Simctl {
        device: String,
    },
}

impl Executor {
    pub(crate) fn platform(&self) -> Platform {
        match self {
            Executor::Local => Platform::Host,
            Executor::Adb { .. } => Platform::Android,
            Executor::Simctl { .. } => Platform::Ios,
        }
    }

    /// Push the binary to the device if needed and return the path the
    /// executor should run. Host-visible executors return the local path.
    pub(crate) async fn stage(&self, exe: &Path, package: &str) -> Result<String> {
        let Executor::Adb { remote_dir, .. } = self else {
            return Ok(exe.display().to_string());
        };

        let file = exe
            .file_name()
            .context("test binary has no file name")?
            .to_string_lossy();
        let remote_dir = format!("{remote_dir}/{package}");
        let remote = format!("{remote_dir}/{file}");

        self.adb(&["shell", "mkdir", "-p", &remote_dir])
            .output()
            .await
            .context("Failed to create remote test dir")?;
        let status = self
            .adb(&["push"])
            .arg(exe)
            .arg(&remote)
            .output()
            .await
            .context("Failed to push test binary to device")?;
        if !status.status.success() {
            bail!(
                "adb push {} failed: {}",
                exe.display(),
                String::from_utf8_lossy(&status.stderr)
            );
        }
        self.adb(&["shell", "chmod", "755", &remote])
            .output()
            .await
            .context("Failed to chmod remote test binary")?;
        Ok(remote)
    }

    /// Build the command that runs `staged` with `args` and `env`.
    pub(crate) fn command(
        &self,
        staged: &str,
        args: &[String],
        env: &[(String, String)],
    ) -> Command {
        match self {
            Executor::Local => {
                let mut cmd = Command::new(staged);
                cmd.args(args).envs(env.iter().cloned());
                cmd
            }
            Executor::Adb { .. } => {
                let path = Path::new(staged);
                let dir = path
                    .parent()
                    .map(|dir| dir.display().to_string())
                    .unwrap_or_default();
                let file = path
                    .file_name()
                    .map(|file| file.to_string_lossy().to_string())
                    .unwrap_or_else(|| staged.to_string());
                let env_prefix = match env.is_empty() {
                    true => String::new(),
                    false => format!(
                        "env {} ",
                        env.iter()
                            .map(|(key, value)| format!("{key}={}", shell_quote(value)))
                            .collect::<Vec<_>>()
                            .join(" ")
                    ),
                };
                let args = args
                    .iter()
                    .map(|arg| shell_quote(arg))
                    .collect::<Vec<_>>()
                    .join(" ");
                let mut cmd = self.adb(&["shell"]);
                cmd.arg(format!("cd {dir} && {env_prefix}./{file} {args}"));
                cmd
            }
            Executor::Simctl { device } => {
                let mut cmd = Command::new("xcrun");
                cmd.args(["simctl", "spawn", device]).arg(staged).args(args);
                for (key, value) in env {
                    cmd.env(format!("SIMCTL_CHILD_{key}"), value);
                }
                cmd
            }
        }
    }

    fn adb(&self, args: &[&str]) -> Command {
        let Executor::Adb {
            adb, transport_id, ..
        } = self
        else {
            unreachable!()
        };
        let mut cmd = Command::new(adb);
        if let Some(id) = transport_id {
            cmd.args(["-t", id]);
        }
        cmd.args(args);
        cmd
    }

    /// Best-effort kill of a test binary still running on the device after a
    /// timeout. The adb client is already dead at this point.
    pub(crate) async fn kill_remote(&self, staged: &str) {
        if let Executor::Adb { .. } = self {
            let file = Path::new(staged)
                .file_name()
                .map(|file| file.to_string_lossy().to_string())
                .unwrap_or_else(|| staged.to_string());
            _ = self.adb(&["shell", "pkill", "-f", &file]).output().await;
        }
    }
}

/// Quote a string for `adb shell`: single-quote, escaping embedded quotes.
pub(crate) fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

/// A built test binary and the request that produced it.
pub(crate) struct TestBinary {
    pub(crate) request: BuildRequest,
    pub(crate) exe: PathBuf,
    /// `harness = false` \[\[test\]\] targets speak our JSON discovery protocol.
    pub(crate) custom_harness: bool,
    pub(crate) executor: Executor,
    /// Path the executor runs: the local exe, or the on-device path for adb.
    pub(crate) staged: String,
}

impl TestBinary {
    fn platform(&self) -> Platform {
        self.executor.platform()
    }
}

pub(crate) struct HostSuite {
    pub(crate) binaries: Vec<TestBinary>,
}

/// Pick the executor for a request: adb push for Android, `simctl spawn` for
/// the iOS simulator, direct execution otherwise.
async fn executor_for(req: &BuildRequest) -> Result<Executor> {
    match req.bundle {
        BundleFormat::Android => {
            let adb = req
                .workspace
                .android_tools()
                .context("Failed to get android tools")?
                .adb
                .clone();
            let transport =
                AppBuilder::get_android_device_transport_id(&adb, req.device_name.as_deref()).await;
            let transport_id = match transport.as_slice() {
                [_, id] => Some(id.clone()),
                _ => None,
            };
            Ok(Executor::Adb {
                adb,
                transport_id,
                remote_dir: "/data/local/tmp/dx/tests".to_string(),
            })
        }
        BundleFormat::Ios => {
            // `--device` selects a physical iOS device which needs a signed app.
            if req.device_name.is_some() {
                bail!(
                    "dx test on physical iOS devices requires a signed app bundle; run on the simulator (`--ios`) instead"
                );
            }
            Ok(Executor::Simctl {
                device: "booted".to_string(),
            })
        }
        _ => Ok(Executor::Local),
    }
}

/// Enumerate testable targets and build them with `BuildKind::Test`.
pub(crate) async fn build(args: &TestArgs, requests: &[BuildRequest]) -> Result<HostSuite> {
    let mut selected = vec![];
    let mut executors = vec![];
    for req in requests {
        let mut executor = None;
        for target in args.select_targets(req)? {
            let custom_harness = target.kind.contains(&TargetKind::Test)
                && is_harness_false(req.crate_dir().join("Cargo.toml"), &target.name)?;
            let mut r = req.clone();
            r.kind = BuildKind::Test;
            r.crate_target = target;
            // One executor per request; probed only when a target is selected.
            let executor_idx = match executor {
                Some(idx) => idx,
                None => {
                    executors.push(executor_for(req).await?);
                    executor = Some(executors.len() - 1);
                    executors.len() - 1
                }
            };
            selected.push((r, custom_harness, executor_idx));
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
        .map(|(r, ..)| r.target_dir.join("dx").join("test-binaries"));
    if let Some(dir) = &stable_dir {
        std::fs::create_dir_all(dir).context("Failed to create test binary dir")?;
    }
    let mut binaries = vec![];
    for (idx, (req, custom_harness, executor_idx)) in selected.iter().enumerate() {
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
        let executor = executors
            .get(*executor_idx)
            .context("missing executor")?
            .clone();
        let staged = executor
            .stage(&exe, &req.package().name)
            .await
            .context("Failed to stage test binary")?;
        tracing::debug!("Built test binary {}", exe.display());
        binaries.push(TestBinary {
            request: req.clone(),
            exe,
            custom_harness: *custom_harness,
            executor,
            staged,
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
                    platforms: vec![],
                    runnable: true,
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
        platform: binary.platform(),
        name,
    }
}

/// The environment `cargo test` sets for the test binaries it spawns.
fn test_env_vars(req: &BuildRequest) -> Vec<(String, String)> {
    [
        ("CARGO_MANIFEST_DIR", req.crate_dir().display().to_string()),
        ("CARGO_PKG_NAME", req.package().name.clone()),
        ("CARGO_PKG_VERSION", req.crate_version()),
        ("CARGO_CRATE_NAME", req.executable_name().replace('-', "_")),
    ]
    .into_iter()
    .map(|(key, value)| (key.to_string(), value))
    .collect()
}

async fn list_terse(binary: &TestBinary) -> Result<Vec<String>> {
    let args = ["--list", "--format", "terse"].map(str::to_string);
    let output = binary
        .executor
        .command(&binary.staged, &args, &test_env_vars(&binary.request))
        .current_dir(binary.request.crate_dir())
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
    let args = ["--list", "--format", "json"].map(str::to_string);
    let output = binary
        .executor
        .command(&binary.staged, &args, &test_env_vars(&binary.request))
        .current_dir(binary.request.crate_dir())
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
        let id = test_id(binary, name.to_string());
        let Some(test) = parse_discovered_json(id, &value) else {
            return Ok(None);
        };
        tests.push(test);
    }
    Ok(Some(tests))
}

/// One `{"name","file","line","ignore","should_panic","tags","platforms",
/// "runnable"}` line from `--list --format json`.
pub(super) fn parse_discovered_json(
    id: TestId,
    value: &serde_json::Value,
) -> Option<DiscoveredTest> {
    if value
        .get("name")
        .and_then(serde_json::Value::as_str)
        .is_none()
    {
        return None;
    }
    Some(DiscoveredTest {
        id,
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
        platforms: value
            .get("platforms")
            .and_then(serde_json::Value::as_array)
            .map(|platforms| {
                platforms
                    .iter()
                    .filter_map(serde_json::Value::as_str)
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default(),
        runnable: value
            .get("runnable")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(true),
    })
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
        let mut args = vec![
            "--exact".to_string(),
            case.id.name.clone(),
            "--nocapture".to_string(),
        ];
        if resolved.include_ignored {
            args.push("--include-ignored".to_string());
        }
        if resolved.ignored {
            args.push("--ignored".to_string());
        }
        let mut cmd =
            binary
                .executor
                .command(&binary.staged, &args, &test_env_vars(&binary.request));
        cmd.current_dir(binary.request.crate_dir())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let child = cmd
            .kill_on_drop(true)
            .spawn()
            .with_context(|| format!("Failed to run {}", binary.exe.display()))?;
        let output = match tokio::time::timeout(resolved.timeout, child.wait_with_output()).await {
            Ok(output) => {
                output.with_context(|| format!("Failed to run {}", binary.exe.display()))?
            }
            Err(_) => {
                // The local client process is dead; the device-side binary
                // may still be running - best-effort kill it.
                binary.executor.kill_remote(&binary.staged).await;
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
                        artifacts: vec![],
                        artifacts_dir: None,
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
                artifacts: vec![],
                artifacts_dir: None,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_list_json_with_platform_fields() {
        let id = TestId {
            package: "p".into(),
            target: "t".into(),
            platform: Platform::Host,
            name: "tests::web_only".into(),
        };
        let value = serde_json::json!({
            "name": "tests::web_only", "file": "tests/web.rs", "line": 12,
            "ignore": false, "should_panic": true, "tags": ["ui"],
            "platforms": ["web"], "runnable": false
        });
        let test = parse_discovered_json(id.clone(), &value).unwrap();
        assert_eq!(test.id.name, "tests::web_only");
        assert_eq!(test.platforms, vec!["web"]);
        assert!(!test.runnable);
        assert!(test.should_panic);

        // Missing fields default to runnable-on-everything (libtest shape).
        let sparse = serde_json::json!({"name": "tests::plain"});
        let test = parse_discovered_json(id.clone(), &sparse).unwrap();
        assert!(test.platforms.is_empty());
        assert!(test.runnable);

        assert!(parse_discovered_json(id.clone(), &serde_json::json!({})).is_none());
    }

    fn args(cmd: &Command) -> Vec<String> {
        cmd.as_std()
            .get_args()
            .map(|arg| arg.to_string_lossy().to_string())
            .collect()
    }

    #[test]
    fn shell_quote_escapes() {
        assert_eq!(shell_quote("a b"), "'a b'");
        assert_eq!(shell_quote("it's"), "'it'\\''s'");
        assert_eq!(shell_quote(""), "''");
    }

    #[test]
    fn adb_command_renders_remote_shell() {
        let executor = Executor::Adb {
            adb: PathBuf::from("/sdk/adb"),
            transport_id: Some("3".to_string()),
            remote_dir: "/data/local/tmp/dx/tests".to_string(),
        };
        let cmd = executor.command(
            "/data/local/tmp/dx/tests/pkg/0-mytest",
            &["--exact".to_string(), "it's a test".to_string()],
            &[("A".to_string(), "1".to_string())],
        );
        let args = args(&cmd);
        assert_eq!(cmd.as_std().get_program(), Path::new("/sdk/adb"));
        assert_eq!(args[0..3], ["-t", "3", "shell"]);
        assert_eq!(
            args[3],
            "cd /data/local/tmp/dx/tests/pkg && env A='1' ./0-mytest '--exact' 'it'\\''s a test'"
        );
    }

    #[test]
    fn simctl_command_spawns_with_child_env() {
        let executor = Executor::Simctl {
            device: "booted".to_string(),
        };
        let cmd = executor.command(
            "/bin/mytest",
            &["--list".to_string()],
            &[("A".to_string(), "1".to_string())],
        );
        assert_eq!(cmd.as_std().get_program(), Path::new("xcrun"));
        assert_eq!(
            args(&cmd),
            ["simctl", "spawn", "booted", "/bin/mytest", "--list"]
        );
        assert!(cmd.as_std().get_envs().any(|(key, value)| {
            key == "SIMCTL_CHILD_A" && value.map(|v| v == "1").unwrap_or_default()
        }));
    }

    /// The adb executor talks to a fake adb script; verifies the staging
    /// sequence (mkdir/push/chmod) and the run command round-trips.
    #[cfg(unix)]
    #[tokio::test]
    async fn adb_executor_against_fake_adb() {
        let dir = std::env::temp_dir().join(format!("dx-fake-adb-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let log = dir.join("calls.log");
        let adb = dir.join("adb");
        std::fs::write(
            &adb,
            indoc::formatdoc! {"
                #!/bin/sh
                echo \"$@\" >> {log}
                exit 0
            ", log = log.display()},
        )
        .unwrap();
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&adb, std::fs::Permissions::from_mode(0o755)).unwrap();

        let exe = dir.join("mytest");
        std::fs::write(&exe, "binary").unwrap();

        let executor = Executor::Adb {
            adb: adb.clone(),
            transport_id: None,
            remote_dir: "/data/local/tmp/dx/tests".to_string(),
        };
        let staged = executor.stage(&exe, "mypkg").await.unwrap();
        assert_eq!(staged, "/data/local/tmp/dx/tests/mypkg/mytest");

        executor
            .command(&staged, &["--exact".to_string()], &[])
            .output()
            .await
            .unwrap();
        executor.kill_remote(&staged).await;

        let calls = std::fs::read_to_string(&log).unwrap();
        let calls = calls.lines().collect::<Vec<_>>();
        assert_eq!(
            calls,
            [
                "shell mkdir -p /data/local/tmp/dx/tests/mypkg",
                &format!(
                    "push {} /data/local/tmp/dx/tests/mypkg/mytest",
                    exe.display()
                ),
                "shell chmod 755 /data/local/tmp/dx/tests/mypkg/mytest",
                "shell cd /data/local/tmp/dx/tests/mypkg && ./mytest '--exact'",
                "shell pkill -f mytest",
            ]
        );

        _ = std::fs::remove_dir_all(&dir);
    }
}
