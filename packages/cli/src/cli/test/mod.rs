use super::*;
use crate::{Anonymized, BuildRequest, BundleFormat};
use anyhow::{Context, Result, anyhow, bail};
use futures_util::{StreamExt, stream::FuturesUnordered};
use krates::cm::TargetKind;
use std::{collections::VecDeque, path::PathBuf, sync::Arc, time::Instant};
use target_lexicon::Triple;
use tokio::sync::Semaphore;

mod config;
mod host;
mod report;
mod web;

pub(crate) use config::{ListFormat, MessageFormat, Resolved};
#[cfg(test)]
use report::TestId;
pub(crate) use report::{
    DiscoveredTest, HumanReporter, JsonReporter, JunitWriter, Outcome, Platform, Reporter, Summary,
    TestOutcome,
};

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

    /// Output format for `--list` (terse or json)
    #[clap(long, requires = "list")]
    pub(crate) format: Option<String>,

    /// Run all tests regardless of failure
    #[clap(long)]
    pub(crate) no_fail_fast: bool,

    /// Number of tests to run concurrently [default: number of cpus]
    #[clap(long, short = 'j')]
    pub(crate) test_threads: Option<usize>,

    /// Retry failing tests this many times
    #[clap(long)]
    pub(crate) retries: Option<u32>,

    /// Also run #[ignore]d tests
    #[clap(long)]
    pub(crate) include_ignored: bool,

    /// Run only #[ignore]d tests
    #[clap(long)]
    pub(crate) ignored: bool,

    /// Only run tests carrying this tag (repeatable, OR). Tests without tag
    /// metadata (eg libtest) are always excluded when --tag is given.
    #[clap(long)]
    pub(crate) tag: Vec<String>,

    /// Skip tests carrying this tag (repeatable)
    #[clap(long)]
    pub(crate) skip_tag: Vec<String>,

    /// Partition the discovered tests: `count:N/M` (sorted round-robin) or
    /// `hash:N/M` (stable hash of the test id); N is 1-based.
    #[clap(long)]
    pub(crate) partition: Option<String>,

    /// Test results output format
    #[clap(long, value_enum)]
    pub(crate) message_format: Option<MessageFormat>,

    /// Write a JUnit XML report to this path
    #[clap(long)]
    pub(crate) junit: Option<PathBuf>,

    /// Browser executable used for web tests.
    #[clap(long)]
    pub(crate) browser: Option<String>,

    /// Per-test timeout (e.g. `60s`, `250ms`). Defaults to the \[test\] timeout in
    /// Dioxus.toml, then `60s`.
    #[clap(long)]
    pub(crate) timeout: Option<String>,

    /// Directory failure artifacts (test output, captured DOM) are written to.
    /// Defaults to `target/dx/test-artifacts/<timestamp>`.
    #[clap(long)]
    pub(crate) artifacts_dir: Option<PathBuf>,

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
            "ignored": self.ignored,
            "tags": self.tag.len(),
            "skip_tags": self.skip_tag.len(),
            "partition": self.partition.is_some(),
            "message_format": self.message_format.is_some(),
            "junit": self.junit.is_some(),
            "browser": self.browser.is_some(),
            "timeout": self.timeout,
            "artifacts_dir": self.artifacts_dir.is_some(),
            "build_args": self.build_args.anonymized(),
        }}
    }
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

        let resolved = config::resolve(&self, requests.first().map(|req| &req.config.test))?;
        let artifacts_root = resolved.artifacts_dir.clone().or_else(|| {
            requests.first().map(|req| {
                req.target_dir.join("dx").join("test-artifacts").join(
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|dur| dur.as_secs().to_string())
                        .unwrap_or_default(),
                )
            })
        });

        // Split requests by what can run them: web bundles get the browser
        // runner, host/android/ios get the process runner, everything else warns.
        let host = Triple::host();
        let mut host_requests = vec![];
        let mut web_requests = vec![];
        for req in requests {
            if req.bundle == BundleFormat::Web {
                web_requests.push(req);
            } else if req.bundle == BundleFormat::Android
                || req.bundle == BundleFormat::Ios
                || req.triple == host
            {
                host_requests.push(req);
            } else {
                tracing::warn!(
                    "Skipping {} [{}]: dx test only supports host, web, android and ios targets",
                    req.package,
                    req.triple
                );
            }
        }
        if host_requests.is_empty() && web_requests.is_empty() {
            bail!("No testable targets: dx test supports host, web, android and ios targets");
        }

        // Build all suites first so `--no-run` never launches a browser.
        let host_suite = host::build(&self, &host_requests).await?;
        let web_suite = web::build(&self, &web_requests).await?;

        if self.no_run {
            for binary in &host_suite.binaries {
                tracing::info!("{}", binary.exe.display());
            }
            for binary in &web_suite.binaries {
                tracing::info!("{}", binary.dir.display());
            }
            return Ok(StructuredOutput::Success);
        }

        // Discover tests in every suite, then apply the global filters.
        let browser = match web_suite.binaries.is_empty() {
            true => None,
            false => Some(web::resolve_browser(resolved.browser.as_deref())?),
        };
        let mut discovered = host::discover(&host_suite).await?;
        if let Some(browser) = &browser {
            discovered.extend(web::discover(&web_suite, browser, &resolved).await?);
        }
        discovered.sort_by(|a, b| a.id.cmp(&b.id));

        let total_discovered = discovered.len();
        let selected = self.select_cases(discovered, &resolved);

        if self.list {
            self.print_list(&selected, &resolved);
            return Ok(StructuredOutput::Success);
        }

        // Reporters: human or json on stdout, junit writes a file at the end.
        let mut reporters: Vec<Box<dyn Reporter>> = vec![match resolved.message_format {
            MessageFormat::Json => Box::new(JsonReporter::new()),
            MessageFormat::Human => Box::new(HumanReporter::default()),
        }];
        if let Some(path) = &resolved.junit {
            reporters.push(Box::new(JunitWriter::new(path.clone())));
        }

        for reporter in reporters.iter_mut() {
            reporter.suite_started(&selected);
        }
        let started = Instant::now();

        let mut outcomes = vec![];
        let mut fail_fast = false;
        for platform in [
            Platform::Host,
            Platform::Android,
            Platform::Ios,
            Platform::Web,
        ] {
            let cases = selected
                .iter()
                .filter(|case| case.id.platform == platform)
                .collect::<Vec<_>>();
            if cases.is_empty() {
                continue;
            }
            // Devices can't run test binaries in parallel - one shell at a time.
            let threads = match platform {
                Platform::Android | Platform::Ios => {
                    if resolved.test_threads > 1 {
                        tracing::debug!("forcing --test-threads=1 for {} tests", platform.as_str());
                    }
                    1
                }
                _ => resolved.test_threads,
            };
            let no_fail_fast = resolved.no_fail_fast;
            let semaphore = Arc::new(Semaphore::new(threads));
            let mut pending: VecDeque<&DiscoveredTest> = cases.into_iter().collect();
            let mut running = FuturesUnordered::new();
            loop {
                tokio::select! {
                    permit = semaphore.clone().acquire_owned(), if !pending.is_empty() && (!fail_fast || no_fail_fast) => {
                        let _permit = permit.context("test scheduler semaphore closed")?;
                        let case = pending.pop_front().unwrap();
                        for reporter in reporters.iter_mut() {
                            reporter.test_started(&case.id);
                        }
                        let fut: std::pin::Pin<Box<dyn Future<Output = Result<Option<TestOutcome>>>>> =
                            match platform {
                                Platform::Web => Box::pin(web::run_case(&web_suite, case, browser.as_deref().unwrap_or(""), &resolved)),
                                _ => Box::pin(host::run_case(&host_suite, case, &resolved)),
                            };
                        running.push(async move { fut.await.map(|outcome| (case.id.clone(), outcome)) });
                    }
                    Some(res) = running.next() => {
                        let (_id, outcome) = res.context("test task failed")?;
                        let Some(mut result) = outcome else { continue };
                        if result.outcome.failed() {
                            fail_fast = true;
                            if let Some(root) = &artifacts_root {
                                if let Err(err) = write_artifacts(root, &mut result) {
                                    tracing::warn!("Failed to write test artifacts: {err}");
                                }
                            }
                        }
                        for reporter in reporters.iter_mut() {
                            reporter.test_finished(&result);
                        }
                        outcomes.push(result);
                    }
                    else => break,
                }
            }
        }

        let summary = Summary {
            total: outcomes.len(),
            passed: outcomes
                .iter()
                .filter(|o| o.outcome == Outcome::Passed)
                .count(),
            failed: outcomes.iter().filter(|o| o.outcome.failed()).count(),
            ignored: outcomes
                .iter()
                .filter(|o| o.outcome == Outcome::Ignored)
                .count(),
            filtered_out: total_discovered - selected.len(),
            elapsed: started.elapsed(),
            failing: outcomes
                .iter()
                .filter(|o| o.outcome.failed())
                .map(|o| o.id.clone())
                .collect(),
        };
        for reporter in reporters.iter_mut() {
            reporter.suite_finished(&summary);
        }

        if summary.failed > 0 {
            return Err(anyhow!("{} tests failed", summary.failed));
        }
        Ok(StructuredOutput::Success)
    }

    /// Apply name filters, tag filters, ignored selection and partitioning to
    /// the discovered tests.
    fn select_cases(
        &self,
        discovered: Vec<DiscoveredTest>,
        resolved: &Resolved,
    ) -> Vec<DiscoveredTest> {
        discovered
            .into_iter()
            .enumerate()
            .filter(|(_, case)| matches_filters(&case.id.name, &self.filters, self.exact))
            // --ignored runs only ignored tests; without --include-ignored the
            // ignored tests still appear in the list (they report as skipped).
            .filter(|(_, case)| !resolved.ignored || case.ignore)
            .filter(|(_, case)| {
                self.tag.is_empty() || case.tags.iter().any(|tag| self.tag.contains(tag))
            })
            .filter(|(_, case)| !case.tags.iter().any(|tag| self.skip_tag.contains(tag)))
            .filter(|(position, case)| match &resolved.partition {
                Some(partition) => partition.contains(&case.id, *position),
                None => true,
            })
            .map(|(_, case)| case)
            .collect()
    }

    fn print_list(&self, selected: &[DiscoveredTest], resolved: &Resolved) {
        for case in selected {
            match resolved.list_format {
                ListFormat::Terse => {
                    println!("{}::{}  {}", case.id.package, case.id.target, case.id.name)
                }
                ListFormat::Json => println!(
                    "{}",
                    serde_json::json!({
                        "name": case.id.label(),
                        "package": case.id.package,
                        "target": case.id.target,
                        "platform": case.id.platform.as_str(),
                        "file": case.file,
                        "line": case.line,
                        "ignore": case.ignore,
                        "should_panic": case.should_panic,
                        "tags": case.tags,
                    })
                ),
            }
        }
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

/// `cargo metadata` doesn't expose `harness`, so `harness = false` \[\[test\]\]
/// targets are detected by parsing the package manifest directly.
pub(crate) fn is_harness_false(manifest: PathBuf, name: &str) -> Result<bool> {
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

/// Write a failing test's output, message and captured artifacts into
/// `<root>/<package>__<target>__<platform>__<name (with :: -> __)>/`.
fn write_artifacts(root: &std::path::Path, outcome: &mut TestOutcome) -> Result<PathBuf> {
    let dir = root.join(format!(
        "{}__{}__{}__{}",
        outcome.id.package,
        outcome.id.target,
        outcome.id.platform.as_str(),
        outcome.id.name.replace("::", "__")
    ));
    std::fs::create_dir_all(&dir)
        .with_context(|| format!("Failed to create artifacts dir {}", dir.display()))?;
    std::fs::write(dir.join("output.txt"), &outcome.output)?;
    std::fs::write(
        dir.join("message.txt"),
        outcome.message.as_deref().unwrap_or_default(),
    )?;
    for (name, bytes) in &outcome.artifacts {
        std::fs::write(dir.join(name), bytes)?;
    }
    outcome.artifacts_dir = Some(dir.clone());
    Ok(dir)
}

/// Whether tag filters select this discovered test. Factored out for testing.
#[cfg(test)]
fn tag_selected(case: &DiscoveredTest, tags: &[String], skip: &[String]) -> bool {
    (tags.is_empty() || case.tags.iter().any(|tag| tags.contains(tag)))
        && !case.tags.iter().any(|tag| skip.contains(tag))
}

#[cfg(test)]
mod tests {
    use super::host::{parse_run_outcome, parse_test_list};
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
            Outcome::Passed
        );
        assert_eq!(
            parse_run_outcome(
                true,
                "running 1 test\ntest tests::later ... ignored\n\ntest result: ok. 0 passed; 0 failed; 1 ignored; 0 measured; 2 filtered out; finished in 0.00s\n"
            ),
            Outcome::Ignored
        );
        assert_eq!(
            parse_run_outcome(
                false,
                "running 1 test\ntest tests::broke ... FAILED\n\ntest result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s\n"
            ),
            Outcome::Failed
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

    #[test]
    fn failure_artifacts_written() {
        let root = std::env::temp_dir().join(format!("dx-artifacts-{}", std::process::id()));
        _ = std::fs::remove_dir_all(&root);
        let mut outcome = TestOutcome {
            id: TestId {
                package: "pkg".into(),
                target: "lib".into(),
                platform: Platform::Web,
                name: "tests::fails".into(),
            },
            outcome: Outcome::Failed,
            flaky: false,
            attempts: 1,
            message: Some("boom".into()),
            output: "stdout here".into(),
            artifacts: vec![("dom.html".to_string(), b"<html/>".to_vec())],
            artifacts_dir: None,
            elapsed: std::time::Duration::from_millis(1),
        };
        let dir = write_artifacts(&root, &mut outcome).unwrap();
        assert_eq!(dir, root.join("pkg__lib__web__tests__fails"));
        assert_eq!(
            std::fs::read_to_string(dir.join("output.txt")).unwrap(),
            "stdout here"
        );
        assert_eq!(
            std::fs::read_to_string(dir.join("message.txt")).unwrap(),
            "boom"
        );
        assert_eq!(std::fs::read(dir.join("dom.html")).unwrap(), b"<html/>");
        assert_eq!(outcome.artifacts_dir.as_deref(), Some(dir.as_path()));

        // The JSON reporter surfaces the artifacts dir on failures.
        let lines = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let mut reporter = JsonReporter::capturing(lines.clone());
        reporter.test_finished(&outcome);
        let event: serde_json::Value = serde_json::from_str(&lines.lock().unwrap()[0]).unwrap();
        assert_eq!(event["artifacts_dir"], dir.display().to_string());

        _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn tag_filters() {
        let case = |tags: &[&str]| DiscoveredTest {
            id: TestId {
                package: "p".into(),
                target: "t".into(),
                platform: Platform::Host,
                name: "n".into(),
            },
            binary: 0,
            file: None,
            line: None,
            ignore: false,
            should_panic: false,
            tags: tags.iter().map(|t| t.to_string()).collect(),
        };
        let ui = vec!["ui".to_string()];

        assert!(tag_selected(&case(&["ui"]), &ui, &[]));
        assert!(!tag_selected(&case(&["net"]), &ui, &[]));
        assert!(!tag_selected(&case(&[]), &ui, &[]));
        assert!(!tag_selected(&case(&["ui"]), &[], &ui));
        assert!(tag_selected(&case(&["net"]), &[], &ui));
        assert!(tag_selected(&case(&[]), &[], &ui));
    }
}
