//! # Telemetry for the Dioxus CLI
//!
//! Dioxus uses telemetry in the CLI to get insight into metrics like performance, panics, and usage
//! of various arguments. This data helps us track down bugs and improve quality of the tooling.
//!
//! Usage of telemetry in open source products can be controversial. Our goal here is to collect
//! minimally invasive data used exclusively to improve our tooling. Github issues only show *some*
//! of the problem, but many users stumble into issues which go unreported.
//!
//! Our policy follows:
//! - minimally invasive
//! - anonymous
//! - periodic
//! - transparent
//! - easy to disable
//!
//! We send a heartbeat when the CLI is executed and then rollups of logs over time.
//! - Heartbeat: helps us track version distribution of the CLI and critical "failures on launch" useful during new version rollouts.
//! - Rollups: helps us track performance and issues over time, as well as usage of various commands.
//!
//! Rollups are not done in background processes, but rather directly by the `dx` CLI.
//! If you don't run the CLI, then we won't send any data.
//!
//! We don't collect any PII, but we do collect three "controversial" pieces of data:
//! - the target triple of your system (OS, arch, etc)
//! - a session ID which is a random number generated on each run
//! - a distinct ID per `.dx` installation which is a random number generated on initial run.
//!
//! The distinct ID is used to track the same installation over time, but it is not tied to any user
//! account or PII. Since `dx` doesn't have any accounts or authentication mechanism, this ID is used
//! as a "best effort" identifier. If you still want to participate in telemetry but don't want a
//! distinct ID, you can replace the stable_id.json file in the `.dx` directory with an empty string.
//!
//! In the CLI, you can disable this by using any of the methods:
//! - installing with the "disable-telemetry" feature flag
//! - setting TELEMETRY=false in your env
//! - setting `dx config set disable-telemetry true`

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashMap},
    time::SystemTime,
};

/// An event's data, corresponding roughly to data collected from an individual trace.
///
/// This can be something like a build, bundle, translate, etc
/// We collect the phases of the build in a list of events to get a better sense of how long
/// it took.
///
/// Note that this is just the data and does not include the reporter information.
///
/// On the analytics, side, we reconstruct the trace messages into a sequence of events, using
/// the stage as a marker.
///
/// If the event contains a stack trace, it is considered a crash event and will be sent to the crash reporting service.
///
/// We store this type on disk without the reporter information or any information about the CLI.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TelemetryEventData {
    /// The name of the command that was run, e.g. "dx build", "dx bundle", "dx serve"
    pub command: String,

    /// The action that was taken, e.g. "build", "bundle", "cli_invoked", "cli_crashed" etc
    pub action: String,

    /// An additional message to include in the event, e.g. "start", "end", "error", etc
    pub message: String,

    /// The "name" of the error. In our case, usually" "RustError" or "RustPanic". In other languages
    /// this might be the exception type. In Rust, this is usually the name of the error type. (e.g. "std::io::Error", etc)
    pub error_type: Option<String>,

    /// Whether the event was handled or not. Unhandled errors are the default, but some we recover from (like hotpatching issues).
    pub error_handled: bool,

    /// Additional values to include in the event, e.g. "duration", "enabled", etc.
    pub values: HashMap<String, serde_json::Value>,

    /// Timestamp of the event, in UTC, derived from the user's system time. Might not be reliable.
    pub time: DateTime<Utc>,

    /// The module where the event occurred, stripped of paths for privacy.
    pub module: Option<String>,

    /// The file or module where the event occurred, stripped of paths for privacy, relative to the monorepo root.
    pub file: Option<String>,

    /// The line and column where the event occurred, if applicable.
    pub line: Option<u32>,

    /// The column where the event occurred, if applicable.
    pub column: Option<u32>,

    /// The stack frames of the event, if applicable.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub stack_frames: Vec<StackFrame>,
}

impl TelemetryEventData {
    pub fn new(name: impl ToString, message: impl ToString) -> Self {
        Self {
            command: std::env::args()
                .nth(1)
                .unwrap_or_else(|| "unknown".to_string()),
            action: strip_paths(&name.to_string()),
            message: strip_paths(&message.to_string()),
            file: None,
            module: None,
            time: DateTime::<Utc>::from(SystemTime::now()),
            values: HashMap::new(),
            error_type: None,
            column: None,
            line: None,
            stack_frames: vec![],
            error_handled: false,
        }
    }

    pub fn with_value<K: ToString, V: serde::Serialize>(mut self, key: K, value: V) -> Self {
        let mut value = serde_json::to_value(value).unwrap();
        strip_paths_value(&mut value);
        self.values.insert(key.to_string(), value);
        self
    }

    pub fn with_module(mut self, module: impl ToString) -> Self {
        self.module = Some(strip_paths(&module.to_string()));
        self
    }

    pub fn with_file(mut self, file: impl ToString) -> Self {
        self.file = Some(strip_paths(&file.to_string()));
        self
    }

    pub fn with_line_column(mut self, line: u32, column: u32) -> Self {
        self.line = Some(line);
        self.column = Some(column);
        self
    }

    pub fn with_error_handled(mut self, error_handled: bool) -> Self {
        self.error_handled = error_handled;
        self
    }

    pub fn with_error_type(mut self, error_type: String) -> Self {
        self.error_type = Some(error_type);
        self
    }

    pub fn with_stack_frames(mut self, stack_frames: Vec<StackFrame>) -> Self {
        self.stack_frames = stack_frames;
        self
    }

    pub fn with_values(mut self, fields: serde_json::Map<String, serde_json::Value>) -> Self {
        for (key, value) in fields {
            self = self.with_value(key, value);
        }
        self
    }

    pub fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap()
    }
}

/// Display implementation for TelemetryEventData, such that you can use it in tracing macros with the "%" syntax.
impl std::fmt::Display for TelemetryEventData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", serde_json::to_string(self).unwrap())
    }
}

/// A serialized stack frame, in a format that matches PostHog's stack frame format.
///
/// Read more:
/// <https://github.com/PostHog/posthog-js/blob/6e35a639a4d06804f6844cbde15adf11a069b92b/packages/node/src/extensions/error-tracking/types.ts#L55>
///
/// Supposedly, this is compatible with Sentry's stack frames as well. In the CLI we use sentry-backtrace
/// even though we don't actually use sentry.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub struct StackFrame {
    pub raw_id: String,

    pub mangled_name: String,

    pub resolved_name: String,

    pub lang: String,

    pub resolved: bool,

    pub platform: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filename: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub function: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub module: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lineno: Option<u64>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub colno: Option<u64>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub abs_path: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_line: Option<String>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pre_context: Vec<String>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub post_context: Vec<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub in_app: Option<bool>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instruction_addr: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub addr_mode: Option<String>,

    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub vars: BTreeMap<String, serde_json::Value>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chunk_id: Option<String>,
}

// If the CLI is compiled locally, it can contain backtraces which contain the home path with the username in it.
pub fn strip_paths(string: &str) -> String {
    // Strip the home path from any paths in the backtrace
    let home_dir = dirs::home_dir().unwrap_or_default();

    // Strip every path between the current path and the home directory
    let mut cwd = std::env::current_dir().unwrap_or_default();
    let mut string = string.to_string();
    loop {
        string = string.replace(&*cwd.to_string_lossy(), "<stripped>");
        let Some(parent) = cwd.parent() else {
            break;
        };
        cwd = parent.to_path_buf();
        if cwd == home_dir {
            break;
        }
    }

    // Finally, strip the home directory itself (in case the cwd is outside the home directory)
    string.replace(&*home_dir.to_string_lossy(), "~")
}

fn strip_paths_value(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::String(s) => *s = strip_paths(s),
        serde_json::Value::Object(map) => map.values_mut().for_each(strip_paths_value),
        serde_json::Value::Array(arr) => arr.iter_mut().for_each(strip_paths_value),
        _ => {}
    }
}
