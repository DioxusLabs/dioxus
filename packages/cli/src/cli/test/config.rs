use super::{TestArgs, report::TestId};
use crate::config::TestConfig;
use anyhow::{Context, Result, bail};
use std::path::PathBuf;
use std::time::Duration;

/// `dx test` settings after applying CLI flag > Dioxus.toml > built-in default.
pub(crate) struct Resolved {
    pub(crate) timeout: Duration,
    pub(crate) retries: u32,
    pub(crate) test_threads: usize,
    pub(crate) no_fail_fast: bool,
    pub(crate) browser: Option<String>,
    pub(crate) junit: Option<PathBuf>,
    pub(crate) artifacts_dir: Option<PathBuf>,
    pub(crate) message_format: MessageFormat,
    pub(crate) list_format: ListFormat,
    pub(crate) include_ignored: bool,
    pub(crate) ignored: bool,
    pub(crate) partition: Option<Partition>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
pub(crate) enum MessageFormat {
    Human,
    Json,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ListFormat {
    Terse,
    Json,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Partition {
    /// `count:N/M` - keep the `N-1`-th of every `M` tests in sorted order.
    Count { index: usize, modulo: usize },
    /// `hash:N/M` - keep tests whose `DefaultHasher` bucket is `N-1`.
    Hash { index: usize, modulo: usize },
}

impl Partition {
    pub(crate) fn parse(spec: &str) -> Result<Partition> {
        let (kind, rest) = spec.split_once(':').with_context(|| {
            format!("invalid partition `{spec}` (expected `count:N/M` or `hash:N/M`)")
        })?;
        let (n, m) = rest
            .split_once('/')
            .with_context(|| format!("invalid partition `{spec}` (expected `N/M`)"))?;
        let n = n
            .parse::<usize>()
            .with_context(|| format!("invalid partition index in `{spec}`"))?;
        let m = m
            .parse::<usize>()
            .with_context(|| format!("invalid partition count in `{spec}`"))?;
        if m == 0 || n == 0 || n > m {
            bail!("invalid partition `{spec}`: need 1 <= N <= M");
        }
        match kind {
            "count" => Ok(Partition::Count {
                index: n - 1,
                modulo: m,
            }),
            "hash" => Ok(Partition::Hash {
                index: n - 1,
                modulo: m,
            }),
            _ => bail!("invalid partition kind `{kind}` (expected `count` or `hash`)"),
        }
    }

    /// Keep the test iff it belongs to this partition. `position` is the test's
    /// index in sorted order (only used by `count`).
    pub(crate) fn contains(&self, id: &TestId, position: usize) -> bool {
        let (index, modulo) = match self {
            Partition::Count { index, modulo } | Partition::Hash { index, modulo } => {
                (*index, *modulo)
            }
        };
        let bucket = match self {
            Partition::Count { .. } => position,
            Partition::Hash { .. } => {
                let mut hasher = std::collections::hash_map::DefaultHasher::new();
                std::hash::Hash::hash(&id.full(), &mut hasher);
                std::hash::Hasher::finish(&hasher) as usize
            }
        };
        bucket % modulo == index
    }
}

/// Parse a humantime-style duration string (`60s`, `250ms`, `2m`, bare seconds).
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

pub(crate) fn resolve(args: &TestArgs, config: Option<&TestConfig>) -> Result<Resolved> {
    let config = config.cloned().unwrap_or_default();

    let timeout = args
        .timeout
        .clone()
        .or(config.timeout)
        .unwrap_or_else(|| "60s".to_string());
    let retries = args.retries.or(config.retries).unwrap_or(0);
    let test_threads = args
        .test_threads
        .or(config.test_threads)
        .or_else(|| std::thread::available_parallelism().ok().map(|n| n.get()))
        .unwrap_or(1)
        .max(1);
    let no_fail_fast = args.no_fail_fast || config.fail_fast == Some(false);
    let browser = args
        .browser
        .clone()
        .or_else(|| config.browser.map(|p| p.display().to_string()));
    let junit = args.junit.clone().or(config.junit);
    let artifacts_dir = args.artifacts_dir.clone().or(config.artifacts_dir);

    let list_format = match args.format.as_deref() {
        None | Some("terse") => ListFormat::Terse,
        Some("json") => ListFormat::Json,
        Some(other) => bail!("invalid --format `{other}` (expected `terse` or `json`)"),
    };

    let partition = args
        .partition
        .as_deref()
        .map(Partition::parse)
        .transpose()?;

    Ok(Resolved {
        timeout: parse_duration(&timeout)?,
        retries,
        test_threads,
        no_fail_fast,
        browser,
        junit,
        artifacts_dir,
        message_format: args.message_format.unwrap_or(MessageFormat::Human),
        list_format,
        include_ignored: args.include_ignored,
        ignored: args.ignored,
        partition,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_id(name: &str) -> TestId {
        TestId {
            package: "pkg".to_string(),
            target: "lib".to_string(),
            platform: super::super::Platform::Host,
            name: name.to_string(),
        }
    }

    #[test]
    fn count_partitions_cover_once() {
        let names = ["a", "b", "c", "d", "e"];
        for index in [0usize, 1] {
            let partition = Partition::parse(&format!("count:{}/2", index + 1)).unwrap();
            let kept = names
                .iter()
                .enumerate()
                .filter(|(position, name)| partition.contains(&test_id(name), *position))
                .map(|(_, name)| *name)
                .collect::<Vec<_>>();
            match index {
                0 => assert_eq!(kept, ["a", "c", "e"]),
                _ => assert_eq!(kept, ["b", "d"]),
            }
        }
    }

    #[test]
    fn hash_partitions_cover_once() {
        let names: Vec<String> = (0..20).map(|i| format!("test_{i}")).collect();
        let first = Partition::parse("hash:1/2").unwrap();
        let second = Partition::parse("hash:2/2").unwrap();
        for name in &names {
            let id = test_id(name);
            assert_ne!(
                first.contains(&id, 0),
                second.contains(&id, 0),
                "{name} in both or neither partition"
            );
        }
    }

    #[test]
    fn bad_partitions_error() {
        assert!(Partition::parse("count:0/2").is_err());
        assert!(Partition::parse("count:3/2").is_err());
        assert!(Partition::parse("foo").is_err());
        assert!(Partition::parse("count:1/0").is_err());
    }
}
