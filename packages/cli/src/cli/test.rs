use super::*;
use crate::{Anonymized, AppBuilder, BuildId, BuildKind, BuildMode, BuildRequest, BundleFormat};
use anyhow::{Context, anyhow, bail};
use futures_util::{StreamExt, stream::FuturesUnordered};
use krates::cm::TargetKind;
use std::{collections::VecDeque, process::Stdio, sync::Arc, time::Instant};
use target_lexicon::Triple;
use tokio::sync::Semaphore;

/// Build and run tests through the dx build pipeline.
#[derive(Clone, Debug, Parser)]
pub(crate) struct TestArgs {
    /// Only run tests whose name contains this string (repeatable)
    pub(crate) filters: Vec<String>,

    /// Match test names exactly
    #[clap(long)]
    pub(crate) exact: bool,

    /// Test only the library target
    #[clap(long)]
    pub(crate) lib: bool,

    /// Test all binary targets
    #[clap(long)]
    pub(crate) bins: bool,

    /// Test all integration test targets
    #[clap(long)]
    pub(crate) tests: bool,

    /// Test only the specified integration test (repeatable)
    #[clap(long)]
    pub(crate) test: Vec<String>,

    /// Compile but do not run tests
    #[clap(long)]
    pub(crate) no_run: bool,

    /// List discovered tests instead of running them
    #[clap(long)]
    pub(crate) list: bool,

    /// Run all tests regardless of failure
    #[clap(long)]
    pub(crate) no_fail_fast: bool,

    /// Number of tests to run concurrently [default: number of cpus]
    #[clap(long, short = 'j')]
    pub(crate) test_threads: Option<usize>,

    /// Retry failing tests this many times
    #[clap(long, default_value = "0")]
    pub(crate) retries: u32,

    /// Also run #[ignore]d tests
    #[clap(long)]
    pub(crate) include_ignored: bool,

    /// Browser executable used for web tests.
    #[clap(long)]
    pub(crate) browser: Option<String>,

    /// Per-test timeout (e.g. `60s`, `250ms`).
    #[clap(long, default_value = "60s")]
    pub(crate) timeout: String,

    /// Information about the target to test
    #[clap(flatten)]
    pub(crate) build_args: CommandWithPlatformOverrides<BuildArgs>,
}

impl Anonymized for TestArgs {
    fn anonymized(&self) -> Value {
        json! {{
            "filters": self.filters.len(),
            "exact": self.exact,
            "lib": self.lib,
            "bins": self.bins,
            "tests": self.tests,
            "no_run": self.no_run,
            "list": self.list,
            "no_fail_fast": self.no_fail_fast,
            "test_threads": self.test_threads,
            "retries": self.retries,
            "include_ignored": self.include_ignored,
            "browser": self.browser.is_some(),
            "timeout": self.timeout,
            "build_args": self.build_args.anonymized(),
        }}
    }
}

/// A single named test in a built test binary.
struct TestCase {
    name: String,
    binary: usize,
}

/// A built test binary and the request that produced it.
struct TestBinary {
    request: BuildRequest,
    exe: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RunOutcome {
    Passed,
    Ignored,
    Failed,
}

struct TestResult {
    binary: usize,
    name: String,
    outcome: RunOutcome,
    /// Set when a retry eventually passed.
    flaky: bool,
    output: String,
    elapsed: std::time::Duration,
}

impl TestArgs {
    pub(crate) async fn test(self) -> Result<StructuredOutput> {
        let BuildTargets { client, server } = self.build_args.clone().into_targets().await?;

        let mut requests = vec![client];
        if let Some(server) = server {
            let client = &requests[0];
            if server.package != client.package || server.features != client.features {
                requests.push(server);
            }
        }

        let web_requests = requests
            .iter()
            .filter(|req| req.bundle == BundleFormat::Web)
            .cloned()
            .collect::<Vec<_>>();
        if !web_requests.is_empty() {
            return crate::cli::test_web::run(self, web_requests).await;
        }

        // Only host targets can run natively for now.
        let host = Triple::host();
        let mut host_requests = vec![];
        for req in requests {
            if req.triple == host {
                host_requests.push(req);
            } else {
                tracing::warn!(
                    "Skipping {} [{}]: dx test currently only runs host targets",
                    req.package,
                    req.triple
                );
            }
        }
        if host_requests.is_empty() {
            bail!("No host targets to test - dx test currently only runs host targets");
        }

        // Enumerate the targets to test for each request.
        let mut selected = vec![];
        for req in &host_requests {
            for target in self.select_targets(req)? {
                let mut r = req.clone();
                r.kind = BuildKind::Test;
                r.crate_target = target;
                selected.push(r);
            }
        }
        if selected.is_empty() {
            bail!("No test targets selected");
        }

        // Build the test binaries sequentially - they share dep artifacts with `dx build` anyway.
        //
        // The recorded exe for a bin harness lives at the profile dir (`{profile}/{name}`), which a
        // later `cargo rustc` invocation in this same loop clobbers (e.g. `--test integ` builds the
        // bin as a normal dependency and copies it over that path). Hardlink each binary into a
        // private dir so subsequent builds can't overwrite the harness underneath us.
        let stable_dir = selected
            .first()
            .map(|r| r.target_dir.join("dx").join("test-binaries"));
        if let Some(dir) = &stable_dir {
            std::fs::create_dir_all(dir).context("Failed to create test binary dir")?;
        }
        let mut binaries = vec![];
        for (idx, req) in selected.iter().enumerate() {
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
            });
        }

        if self.no_run {
            for binary in &binaries {
                tracing::info!("{}", binary.exe.display());
            }
            return Ok(StructuredOutput::Success);
        }

        // Discover the tests in each binary.
        let mut cases = vec![];
        for (idx, binary) in binaries.iter().enumerate() {
            let names = list_tests(binary).await?;
            for name in names {
                if matches_filters(&name, &self.filters, self.exact) {
                    cases.push(TestCase { name, binary: idx });
                }
            }
        }

        if self.list {
            for case in &cases {
                let binary = &binaries[case.binary];
                println!(
                    "{}::{}  {}",
                    binary.request.package,
                    binary.request.executable_name(),
                    case.name
                );
            }
            return Ok(StructuredOutput::Success);
        }

        if cases.is_empty() {
            tracing::info!("No tests matched.");
            return Ok(StructuredOutput::Success);
        }

        let started = Instant::now();
        let threads = self
            .test_threads
            .or_else(|| std::thread::available_parallelism().ok().map(|n| n.get()))
            .unwrap_or(1)
            .max(1);

        // Run one process per test, nextest style, bounded by a semaphore.
        let semaphore = Arc::new(Semaphore::new(threads));
        let mut pending: VecDeque<TestCase> = cases.into_iter().collect();
        let total_run = pending.len();
        let mut running = FuturesUnordered::new();
        let timeout = super::test_web::parse_duration(&self.timeout)?;
        let mut results = vec![];
        let mut failed = vec![];
        let mut fail_fast = false;

        loop {
            tokio::select! {
                permit = semaphore.clone().acquire_owned(), if !pending.is_empty() && (!fail_fast || self.no_fail_fast) => {
                    let _permit = permit.context("test scheduler semaphore closed")?;
                    let case = pending.pop_front().unwrap();
                    let ctx = TestTaskContext::new(&binaries[case.binary], case);
                    let include_ignored = self.include_ignored;
                    let retries = self.retries;
                    running.push(async move { run_test(ctx, include_ignored, retries, timeout).await });
                }
                Some(res) = running.next() => {
                    let res = res.context("test task failed")?;
                    if res.outcome == RunOutcome::Failed {
                        fail_fast = true;
                        failed.push(res.name.clone());
                    }
                    print_test_result(&binaries[res.binary], &res);
                    results.push(res);
                }
                else => break,
            }
        }

        let elapsed = started.elapsed().as_secs_f64();
        let passed = results
            .iter()
            .filter(|r| matches!(r.outcome, RunOutcome::Passed))
            .count();
        let skipped = results
            .iter()
            .filter(|r| matches!(r.outcome, RunOutcome::Ignored))
            .count();

        println!(
            "{} [{elapsed:>8.3}s] {} tests run: {} passed, {} failed, {} skipped",
            console::style("Summary").bold(),
            total_run,
            passed,
            failed.len(),
            skipped,
        );

        if !failed.is_empty() {
            println!("Failing tests:");
            for name in &failed {
                println!("    {name}");
            }
            return Err(anyhow!("{} tests failed", failed.len()));
        }

        Ok(StructuredOutput::Success)
    }

    /// Enumerate the testable targets of the request's package, narrowed by the selector flags.
    pub(crate) fn select_targets(&self, req: &BuildRequest) -> Result<Vec<krates::cm::Target>> {
        let targets = &req.package().targets;
        let bin_filter = self
            .build_args
            .shared
            .build_arguments
            .bin
            .iter()
            .cloned()
            .collect::<Vec<_>>();

        let any_selector =
            self.lib || self.bins || self.tests || !self.test.is_empty() || !bin_filter.is_empty();

        let has_kind = |target: &krates::cm::Target, kind: TargetKind| target.kind.contains(&kind);
        let named = |kind: TargetKind, names: &[String]| -> Result<Vec<krates::cm::Target>> {
            names
                .iter()
                .map(|name| {
                    targets
                        .iter()
                        .find(|t| has_kind(t, kind.clone()) && t.name.as_str() == name)
                        .cloned()
                        .with_context(|| {
                            let available = targets
                                .iter()
                                .filter(|t| has_kind(t, kind.clone()))
                                .map(|t| t.name.as_str())
                                .collect::<Vec<_>>()
                                .join(", ");
                            match kind {
                                TargetKind::Bin => format!(
                                    "Failed to find binary {name}. \nAvailable binaries are:\n{available}"
                                ),
                                _ => format!(
                                    "Failed to find test {name}. \nAvailable tests are:\n{available}"
                                ),
                            }
                        })
                })
                .collect::<Result<Vec<_>>>()
        };

        let mut selected = vec![];
        if !any_selector {
            // Default: lib (if any), every bin, every test target.
            for target in targets {
                if target
                    .kind
                    .iter()
                    .any(|k| matches!(k, TargetKind::Lib | TargetKind::Bin | TargetKind::Test))
                {
                    selected.push(target.clone());
                }
            }
            return Ok(selected);
        }

        if self.lib {
            selected.extend(
                targets
                    .iter()
                    .filter(|t| has_kind(t, TargetKind::Lib))
                    .cloned(),
            );
        }
        if self.bins {
            selected.extend(
                targets
                    .iter()
                    .filter(|t| has_kind(t, TargetKind::Bin))
                    .cloned(),
            );
        }
        if self.tests {
            selected.extend(
                targets
                    .iter()
                    .filter(|t| has_kind(t, TargetKind::Test))
                    .cloned(),
            );
        }
        selected.extend(named(TargetKind::Bin, &bin_filter)?);
        selected.extend(named(TargetKind::Test, &self.test)?);

        // A crate can't have two targets with the same name and kind, but the selector flags
        // can overlap (`--bins --bin foo`) - dedup by name+kind.
        let mut seen = std::collections::HashSet::new();
        selected.retain(|t| seen.insert((t.name.clone(), t.kind.clone())));

        Ok(selected)
    }
}

/// What a spawned test task needs - cloned out of the binary list so the futures can run
/// concurrently without borrowing.
struct TestTaskContext {
    exe: PathBuf,
    crate_dir: PathBuf,
    package: String,
    version: String,
    crate_name: String,
    binary: usize,
    name: String,
}

impl TestTaskContext {
    fn new(binary: &TestBinary, case: TestCase) -> Self {
        Self {
            exe: binary.exe.clone(),
            crate_dir: binary.request.crate_dir(),
            package: binary.request.package().name.clone(),
            version: binary.request.crate_version(),
            crate_name: binary.request.executable_name().replace('-', "_"),
            binary: case.binary,
            name: case.name,
        }
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

/// Run `<exe> --list --format terse` and return the discovered test names.
async fn list_tests(binary: &TestBinary) -> Result<Vec<String>> {
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
fn parse_test_list(stdout: &str) -> Vec<String> {
    stdout
        .lines()
        .filter_map(|line| line.trim_end().strip_suffix(": test").map(str::to_string))
        .collect()
}

pub(crate) fn matches_filters(name: &str, filters: &[String], exact: bool) -> bool {
    if filters.is_empty() {
        return true;
    }
    filters.iter().any(|f| {
        if exact {
            name == f
        } else {
            name.contains(f.as_str())
        }
    })
}

/// Run a single test in its own process, retrying up to `retries` times.
async fn run_test(
    ctx: TestTaskContext,
    include_ignored: bool,
    retries: u32,
    timeout: std::time::Duration,
) -> Result<TestResult> {
    let started = Instant::now();
    let mut attempts = 0;
    loop {
        let mut cmd = tokio::process::Command::new(&ctx.exe);
        cmd.args(["--exact", &ctx.name, "--nocapture"])
            .current_dir(&ctx.crate_dir)
            .env("CARGO_MANIFEST_DIR", &ctx.crate_dir)
            .env("CARGO_PKG_NAME", &ctx.package)
            .env("CARGO_PKG_VERSION", &ctx.version)
            .env("CARGO_CRATE_NAME", &ctx.crate_name)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if include_ignored {
            cmd.arg("--include-ignored");
        }

        let child = cmd
            .kill_on_drop(true)
            .spawn()
            .with_context(|| format!("Failed to run {}", ctx.exe.display()))?;
        let output = match tokio::time::timeout(timeout, child.wait_with_output()).await {
            Ok(output) => output.with_context(|| format!("Failed to run {}", ctx.exe.display()))?,
            Err(_) => {
                let text = format!("test timed out after {}s", timeout.as_secs_f64());
                attempts += 1;
                if attempts > retries + 1 {
                    return Ok(TestResult {
                        binary: ctx.binary,
                        name: ctx.name.clone(),
                        outcome: RunOutcome::Failed,
                        flaky: attempts > 1,
                        output: text,
                        elapsed: started.elapsed(),
                    });
                }
                continue;
            }
        };
        let text = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );

        let outcome = parse_run_outcome(output.status.success(), &text);
        attempts += 1;
        if outcome != RunOutcome::Failed || attempts > retries + 1 {
            return Ok(TestResult {
                binary: ctx.binary,
                name: ctx.name.clone(),
                outcome,
                flaky: attempts > 1,
                output: text,
                elapsed: started.elapsed(),
            });
        }
    }
}

/// Derive a test outcome from the process exit status and the libtest summary line.
fn parse_run_outcome(success: bool, output: &str) -> RunOutcome {
    if !success {
        return RunOutcome::Failed;
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
            return RunOutcome::Failed;
        }
        if passed == 0 && ignored > 0 {
            return RunOutcome::Ignored;
        }
        return RunOutcome::Passed;
    }

    // No summary line - trust the exit code.
    RunOutcome::Passed
}

fn print_test_result(binary: &TestBinary, res: &TestResult) {
    let secs = res.elapsed.as_secs_f64();
    let label = match res.outcome {
        RunOutcome::Passed if res.flaky => console::style("FLAKY").magenta(),
        RunOutcome::Passed => console::style("PASS").green(),
        RunOutcome::Ignored => console::style("SKIP").yellow(),
        RunOutcome::Failed => console::style("FAIL").red(),
    };
    println!(
        "{label} [{secs:>8.3}s] {}::{} {}",
        binary.request.package,
        binary.request.executable_name(),
        res.name
    );
    if res.outcome == RunOutcome::Failed {
        for line in res.output.lines() {
            println!("    {line}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_terse_list_output() {
        let output = indoc::indoc! {"
            tests::adds: test
            tests::nested::does_stuff: test
            benches::big_loop: benchmark
            tests::ignored_case: test
        "};
        assert_eq!(
            parse_test_list(output),
            vec![
                "tests::adds".to_string(),
                "tests::nested::does_stuff".to_string(),
                "tests::ignored_case".to_string(),
            ]
        );
    }

    #[test]
    fn parses_run_outcomes() {
        assert_eq!(
            parse_run_outcome(
                true,
                "running 1 test\ntest tests::adds ... ok\n\ntest result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 3 filtered out; finished in 0.00s\n"
            ),
            RunOutcome::Passed
        );
        assert_eq!(
            parse_run_outcome(
                true,
                "running 1 test\ntest tests::later ... ignored\n\ntest result: ok. 0 passed; 0 failed; 1 ignored; 0 measured; 2 filtered out; finished in 0.00s\n"
            ),
            RunOutcome::Ignored
        );
        assert_eq!(
            parse_run_outcome(
                false,
                "running 1 test\ntest tests::broke ... FAILED\n\ntest result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s\n"
            ),
            RunOutcome::Failed
        );
    }

    #[test]
    fn filters_match_substring_and_exact() {
        let filters = vec!["tests::adds".to_string()];
        assert!(matches_filters("tests::adds", &filters, false));
        assert!(matches_filters("my_tests::adds_more", &filters, false));
        assert!(!matches_filters("tests::other", &filters, false));

        assert!(matches_filters("tests::adds", &filters, true));
        assert!(!matches_filters("my_tests::adds_more", &filters, true));

        assert!(matches_filters("anything", &[], false));
    }
}
