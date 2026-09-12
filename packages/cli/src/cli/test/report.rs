use anyhow::{Context, Result};
use std::{collections::BTreeMap, path::PathBuf, time::Duration};

/// The platform a test runs on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) enum Platform {
    Host,
    Web,
}

impl Platform {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Platform::Host => "host",
            Platform::Web => "web",
        }
    }
}

/// Identifies one test case across all suites.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) struct TestId {
    pub(crate) package: String,
    pub(crate) target: String,
    pub(crate) platform: Platform,
    pub(crate) name: String,
}

impl TestId {
    /// `{target}::{name}` - the libtest-style name used in JSON events.
    pub(crate) fn label(&self) -> String {
        format!("{}::{}", self.target, self.name)
    }

    /// Full identity used for hash partitioning.
    pub(crate) fn full(&self) -> String {
        format!(
            "{}::{}::{}::{}",
            self.package,
            self.target,
            self.platform.as_str(),
            self.name
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Outcome {
    Passed,
    Failed,
    Ignored,
    TimedOut,
}

impl Outcome {
    pub(crate) fn event(&self) -> &'static str {
        match self {
            Outcome::Passed => "ok",
            Outcome::Failed => "failed",
            Outcome::Ignored => "ignored",
            Outcome::TimedOut => "timeout",
        }
    }

    pub(crate) fn failed(&self) -> bool {
        matches!(self, Outcome::Failed | Outcome::TimedOut)
    }
}

pub(crate) struct TestOutcome {
    pub(crate) id: TestId,
    pub(crate) outcome: Outcome,
    /// Set when a retry eventually passed.
    pub(crate) flaky: bool,
    pub(crate) attempts: u32,
    pub(crate) message: Option<String>,
    pub(crate) output: String,
    pub(crate) elapsed: Duration,
}

pub(crate) struct DiscoveredTest {
    pub(crate) id: TestId,
    /// Index into the runner's binary list.
    pub(crate) binary: usize,
    pub(crate) file: Option<String>,
    pub(crate) line: Option<u32>,
    pub(crate) ignore: bool,
    pub(crate) should_panic: bool,
    pub(crate) tags: Vec<String>,
}

#[derive(Default)]
pub(crate) struct Summary {
    pub(crate) total: usize,
    pub(crate) passed: usize,
    pub(crate) failed: usize,
    pub(crate) ignored: usize,
    pub(crate) filtered_out: usize,
    pub(crate) elapsed: Duration,
    pub(crate) failing: Vec<TestId>,
}

pub(crate) trait Reporter {
    fn suite_started(&mut self, tests: &[DiscoveredTest]);
    fn test_started(&mut self, id: &TestId);
    fn test_finished(&mut self, result: &TestOutcome);
    fn suite_finished(&mut self, summary: &Summary);
}

/// The interactive reporter: PASS/FAIL/SKIP/FLAKY/TIMEOUT lines plus a summary.
pub(crate) struct HumanReporter;

impl Reporter for HumanReporter {
    fn suite_started(&mut self, _tests: &[DiscoveredTest]) {}

    fn test_started(&mut self, _id: &TestId) {}

    fn test_finished(&mut self, result: &TestOutcome) {
        let label = match result.outcome {
            Outcome::Passed if result.flaky => console::style("FLAKY").magenta(),
            Outcome::Passed => console::style("PASS").green(),
            Outcome::Ignored => console::style("SKIP").yellow(),
            Outcome::Failed => console::style("FAIL").red(),
            Outcome::TimedOut => console::style("TIMEOUT").red(),
        };
        println!(
            "{label} [{:>8.3}s] {}::{}  {}",
            result.elapsed.as_secs_f64(),
            result.id.package,
            result.id.target,
            result.id.name
        );
        if result.outcome.failed() {
            let output = match result.output.is_empty() {
                false => result.output.as_str(),
                true => result.message.as_deref().unwrap_or("test failed"),
            };
            for line in output.lines() {
                println!("    {line}");
            }
        }
    }

    fn suite_finished(&mut self, summary: &Summary) {
        println!(
            "{} [{:>8.3}s] {} tests run: {} passed, {} failed, {} skipped",
            console::style("Summary").bold(),
            summary.elapsed.as_secs_f64(),
            summary.total,
            summary.passed,
            summary.failed,
            summary.ignored,
        );
        if !summary.failing.is_empty() {
            println!("Failing tests:");
            for id in &summary.failing {
                println!("    {}::{}", id.target, id.name);
            }
        }
    }
}

/// libtest-json compatible reporter: one JSON event per line on stdout.
pub(crate) struct JsonReporter {
    emit: Box<dyn FnMut(String)>,
}

impl JsonReporter {
    pub(crate) fn new() -> Self {
        Self {
            emit: Box::new(|line| println!("{line}")),
        }
    }

    #[cfg(test)]
    fn capturing(lines: std::sync::Arc<std::sync::Mutex<Vec<String>>>) -> Self {
        Self {
            emit: Box::new(move |line| lines.lock().unwrap().push(line)),
        }
    }

    fn event(&mut self, event: serde_json::Value) {
        (self.emit)(event.to_string());
    }
}

impl Reporter for JsonReporter {
    fn suite_started(&mut self, tests: &[DiscoveredTest]) {
        self.event(
            serde_json::json!({"type": "suite", "event": "started", "test_count": tests.len()}),
        );
    }

    fn test_started(&mut self, id: &TestId) {
        self.event(serde_json::json!({
            "type": "test", "event": "started", "name": id.label(),
            "package": id.package, "target": id.target, "platform": id.platform.as_str(),
        }));
    }

    fn test_finished(&mut self, result: &TestOutcome) {
        let mut event = serde_json::json!({
            "type": "test",
            "event": result.outcome.event(),
            "name": result.id.label(),
            "package": result.id.package,
            "target": result.id.target,
            "platform": result.id.platform.as_str(),
            "exec_time": result.elapsed.as_secs_f64(),
            "attempts": result.attempts,
            "flaky": result.flaky,
        });
        if result.outcome.failed() {
            event["stdout"] = match result.output.is_empty() {
                false => serde_json::json!(result.output),
                true => serde_json::json!(result.message.as_deref().unwrap_or("")),
            };
        }
        self.event(event);
    }

    fn suite_finished(&mut self, summary: &Summary) {
        self.event(serde_json::json!({
            "type": "suite",
            "event": if summary.failed == 0 { "ok" } else { "failed" },
            "passed": summary.passed,
            "failed": summary.failed,
            "ignored": summary.ignored,
            "measured": 0,
            "filtered_out": summary.filtered_out,
            "exec_time": summary.elapsed.as_secs_f64(),
        }));
    }
}

/// Collects outcomes and writes JUnit XML on `suite_finished`.
pub(crate) struct JunitWriter {
    path: PathBuf,
    outcomes: Vec<TestOutcome>,
}

impl JunitWriter {
    pub(crate) fn new(path: PathBuf) -> Self {
        Self {
            path,
            outcomes: vec![],
        }
    }

    fn write(&self) -> Result<()> {
        // One <testsuite> per (package, target, platform) binary, in stable order.
        let mut suites: BTreeMap<(String, String, Platform), Vec<&TestOutcome>> = BTreeMap::new();
        for outcome in &self.outcomes {
            suites
                .entry((
                    outcome.id.package.clone(),
                    outcome.id.target.clone(),
                    outcome.id.platform,
                ))
                .or_default()
                .push(outcome);
        }

        let mut xml = String::from("<testsuites>\n");
        for ((package, target, platform), outcomes) in suites {
            let tests = outcomes.len();
            let failures = outcomes
                .iter()
                .filter(|o| o.outcome == Outcome::Failed || o.outcome == Outcome::TimedOut)
                .count();
            let skipped = outcomes
                .iter()
                .filter(|o| o.outcome == Outcome::Ignored)
                .count();
            let time: f64 = outcomes.iter().map(|o| o.elapsed.as_secs_f64()).sum();
            xml.push_str(&format!(
                "  <testsuite name=\"{}\" tests=\"{tests}\" failures=\"{failures}\" skipped=\"{skipped}\" time=\"{time:.3}\">\n",
                escape_xml(&format!("{package}::{target} ({})", platform.as_str())),
            ));
            for outcome in outcomes {
                xml.push_str(&format!(
                    "    <testcase classname=\"{}\" name=\"{}\" time=\"{:.3}\">\n",
                    escape_xml(&format!("{package}::{target}")),
                    escape_xml(&outcome.id.name),
                    outcome.elapsed.as_secs_f64(),
                ));
                if outcome.outcome.failed() {
                    let message = outcome.message.as_deref().unwrap_or("test failed");
                    xml.push_str(&format!(
                        "      <failure message=\"{}\"><![CDATA[{}]]></failure>\n",
                        escape_xml(message),
                        cdata(&outcome.output),
                    ));
                }
                if outcome.outcome == Outcome::Ignored {
                    xml.push_str("      <skipped/>\n");
                }
                if outcome.flaky {
                    xml.push_str(&format!(
                        "      <system-out>flaky: passed after {} attempts</system-out>\n",
                        outcome.attempts
                    ));
                }
                xml.push_str("    </testcase>\n");
            }
            xml.push_str("  </testsuite>\n");
        }
        xml.push_str("</testsuites>\n");

        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&self.path, xml)
            .with_context(|| format!("Failed to write junit report to {}", self.path.display()))
    }
}

impl Reporter for JunitWriter {
    fn suite_started(&mut self, _tests: &[DiscoveredTest]) {}

    fn test_started(&mut self, _id: &TestId) {}

    fn test_finished(&mut self, result: &TestOutcome) {
        self.outcomes.push(TestOutcome {
            id: result.id.clone(),
            outcome: result.outcome,
            flaky: result.flaky,
            attempts: result.attempts,
            message: result.message.clone(),
            output: result.output.clone(),
            elapsed: result.elapsed,
        });
    }

    fn suite_finished(&mut self, _summary: &Summary) {
        if let Err(err) = self.write() {
            tracing::error!("Failed to write junit report: {err}");
        }
    }
}

fn escape_xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn cdata(value: &str) -> String {
    value.replace("]]>", "]]]]><![CDATA[>")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(name: &str) -> TestId {
        TestId {
            package: "pkg".to_string(),
            target: "lib".to_string(),
            platform: Platform::Host,
            name: name.to_string(),
        }
    }

    fn outcome(name: &str, outcome: Outcome, message: Option<&str>) -> TestOutcome {
        TestOutcome {
            id: id(name),
            outcome,
            flaky: false,
            attempts: 1,
            message: message.map(str::to_string),
            output: message.unwrap_or_default().to_string(),
            elapsed: Duration::from_millis(12),
        }
    }

    fn discovered() -> Vec<DiscoveredTest> {
        ["a", "b", "c"]
            .iter()
            .map(|name| DiscoveredTest {
                id: id(name),
                binary: 0,
                file: None,
                line: None,
                ignore: false,
                should_panic: false,
                tags: vec![],
            })
            .collect()
    }

    #[test]
    fn json_reporter_events() {
        let lines = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let mut reporter = JsonReporter::capturing(lines.clone());
        let tests = discovered();
        reporter.suite_started(&tests);
        for (name, out) in [
            ("a", Outcome::Passed),
            ("b", Outcome::Failed),
            ("c", Outcome::Ignored),
        ] {
            let result = outcome(name, out, (out == Outcome::Failed).then_some("boom"));
            reporter.test_started(&result.id);
            reporter.test_finished(&result);
        }
        reporter.suite_finished(&Summary {
            total: 3,
            passed: 1,
            failed: 1,
            ignored: 1,
            ..Default::default()
        });

        let events = lines
            .lock()
            .unwrap()
            .iter()
            .map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap())
            .collect::<Vec<_>>();
        assert_eq!(events.len(), 8);
        assert_eq!(
            events[0],
            serde_json::json!({"type": "suite", "event": "started", "test_count": 3})
        );
        assert_eq!(events[1]["event"], "started");
        assert_eq!(events[1]["name"], "lib::a");
        assert_eq!(events[2]["event"], "ok");
        assert_eq!(events[3]["name"], "lib::b");
        assert_eq!(events[4]["event"], "failed");
        assert_eq!(events[4]["stdout"], "boom");
        assert_eq!(events[4]["platform"], "host");
        assert_eq!(events[6]["event"], "ignored");
        assert_eq!(events[7]["type"], "suite");
        assert_eq!(events[7]["event"], "failed");
        assert_eq!(events[7]["passed"], 1);
        assert_eq!(events[7]["failed"], 1);
        assert_eq!(events[7]["ignored"], 1);
    }

    #[test]
    fn junit_writer_counts_and_escapes() {
        let path = std::env::temp_dir().join(format!("dx-junit-test-{}", std::process::id()));
        let mut writer = JunitWriter::new(path.clone());
        writer.suite_started(&discovered());
        for (name, out) in [
            ("a", Outcome::Passed),
            ("b", Outcome::Failed),
            ("c", Outcome::Ignored),
        ] {
            let result = outcome(name, out, (out == Outcome::Failed).then_some("1 < 2"));
            writer.test_finished(&result);
        }
        writer.suite_finished(&Summary::default());

        let xml = std::fs::read_to_string(&path).unwrap();
        _ = std::fs::remove_file(&path);
        assert!(
            xml.contains("tests=\"3\" failures=\"1\" skipped=\"1\""),
            "{xml}"
        );
        assert!(xml.contains("message=\"1 &lt; 2\""), "{xml}");
        assert!(xml.contains("<skipped/>"), "{xml}");
    }
}
