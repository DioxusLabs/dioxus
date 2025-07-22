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
//! We don't send events on every command, but instead perform roll-ups on a daily basis during the
//! first week of installation, and then weekly after that. If you don't run the CLI, then we won't
//! send any data - rollups are not done in background processes. Rollups are also capped in size to
//! a max of 10mb weekly to prevent DDOS of the dioxus telemetry endpoint.
//!
//! Note that we do collect a hash of your system's entropy during installation. This lets us aggregate
//! logs across time about a given installation, IE if your machine is a particular linux distribution,
//! what types of panics or performance issues do you encounter?
//!
//! In the CLI, you can disable this by using any of the methods
//! - installing with the "disable-telemetry" feature flag
//! - setting TELEMETRY=false in your env
//! - setting `dx settings --disable-telemtry`

use std::{collections::HashMap, sync::OnceLock, time::SystemTime};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// We only store non-pii information in telemetry to track issues and performance
/// across the CLI. This includes:
/// - device triple (OS, arch, etc)
/// - whether the CLI is running in CI
/// - the CLI version
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TelemetryPayload {
    pub device_triple: String,
    pub is_ci: bool,
    pub cli_version: String,
}

impl TelemetryPayload {
    pub fn new(device_triple: String, is_ci: bool, cli_version: String) -> Self {
        Self {
            device_triple,
            is_ci,
            cli_version,
        }
    }
}

static SESSION_ID: OnceLock<u128> = OnceLock::new();

fn session_id() -> u128 {
    *SESSION_ID.get_or_init(|| rand::random::<u128>())
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
    pub name: String,
    pub module: Option<String>,
    pub message: String,
    pub stage: String,
    pub time: DateTime<Utc>,
    pub session_id: u128,
    pub values: HashMap<String, String>,
}

impl TelemetryEvent {
    pub fn new(
        name: impl ToString,
        module: Option<String>,
        message: impl ToString,
        stage: impl ToString,
    ) -> Self {
        Self {
            name: name.to_string(),
            module,
            message: message.to_string(),
            stage: stage.to_string(),
            time: DateTime::<Utc>::from(SystemTime::now()),
            session_id: session_id(),
            values: HashMap::new(),
        }
    }

    pub fn with_value<K: ToString, V: ToString>(mut self, key: K, value: V) -> Self {
        self.values.insert(key.to_string(), value.to_string());
        self
    }
}
