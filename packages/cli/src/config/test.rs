use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// `[test]` section of `Dioxus.toml` - defaults for `dx test`.
///
/// CLI flags always take precedence over these values.
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub(crate) struct TestConfig {
    /// Per-test timeout, e.g. `"60s"`, `"250ms"`, `"2m"`.
    pub(crate) timeout: Option<String>,

    /// Retry failing tests this many times.
    pub(crate) retries: Option<u32>,

    /// Number of tests to run concurrently.
    pub(crate) test_threads: Option<usize>,

    /// Stop scheduling new tests after the first failure (default true).
    pub(crate) fail_fast: Option<bool>,

    /// Browser executable used for web tests.
    pub(crate) browser: Option<PathBuf>,

    /// Default JUnit XML output path.
    pub(crate) junit: Option<PathBuf>,
}
