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
//! We don't send events on every command, but instead perform roll-ups weekly. If you don't run the CLI,
//! then we won't send any data - rollups are not done in background processes.
//!
//! Note that we do collect the target triple of your system. This lets us aggregate
//! logs across time about a given installation, IE if your machine is a particular linux distribution,
//! what types of panics or performance issues do you encounter?
//!
//! In the CLI, you can disable this by using any of the methods
//! - installing with the "disable-telemetry" feature flag
//! - setting TELEMETRY=false in your env
//! - setting `dx config set disable-telemetry true`

use std::{collections::HashMap, sync::OnceLock, time::SystemTime};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// We only store non-pii information in telemetry to track issues and performance
/// across the CLI. This includes:
/// - device triple (OS, arch, etc)
/// - whether the CLI is running in CI
/// - the CLI version
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Identity {
    pub device_triple: String,
    pub is_ci: bool,
    pub cli_version: String,
    pub session_id: u128,
}

impl Identity {
    pub fn new(device_triple: String, is_ci: bool, cli_version: String, session_id: u128) -> Self {
        Self {
            device_triple,
            is_ci,
            cli_version,
            session_id,
        }
    }
}

static IDENTITY: OnceLock<Identity> = OnceLock::new();

pub fn set_identity(device_triple: String, is_ci: bool, cli_version: String) {
    _ = IDENTITY.set(Identity::new(
        device_triple,
        is_ci,
        cli_version,
        rand::random::<u128>(),
    ));
}

fn identity() -> Identity {
    IDENTITY.get().unwrap().clone()
}

/// An event, corresponding roughly to a trace!()
///
/// This can be something like a build, bundle, translate, etc
/// We collect the phases of the build in a list of events to get a better sense of how long
/// it took.
///
/// ```rust
/// tracing::trace!(telemetry, stage = "start", "bundling", "Packaging...")
/// tracing::trace!(telemetry, stage = "end", end = "bundling")
/// ```
///
/// On the analytics, side, we reconstruct the trace messages into a sequence of events, using
/// the stage as a marker.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TelemetryEvent {
    pub identity: Identity,
    pub name: String,
    pub module: Option<String>,
    pub message: String,
    pub stage: String,
    pub time: DateTime<Utc>,
    pub values: HashMap<String, serde_json::Value>,
}

impl TelemetryEvent {
    pub fn new(
        name: impl ToString,
        module: Option<String>,
        message: impl ToString,
        stage: impl ToString,
    ) -> Self {
        Self {
            identity: identity(),
            name: strip_paths(&name.to_string()),
            module: module.map(|m| strip_paths(&m)),
            message: strip_paths(&message.to_string()),
            stage: strip_paths(&stage.to_string()),
            time: DateTime::<Utc>::from(SystemTime::now()),
            values: HashMap::new(),
        }
    }

    pub fn with_value<K: ToString, V: serde::Serialize>(mut self, key: K, value: V) -> Self {
        let mut value = serde_json::to_value(value).unwrap();
        strip_paths_value(&mut value);
        self.values.insert(key.to_string(), value);
        self
    }
}

// If the CLI is compiled locally, it can contain backtraces which contain the home path with the username in it.
fn strip_paths(string: &str) -> String {
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
        serde_json::Value::Object(map) => {
            map.values_mut().for_each(strip_paths_value);
        }
        serde_json::Value::Array(arr) => {
            arr.iter_mut().for_each(strip_paths_value);
        }
        _ => {}
    }
}
