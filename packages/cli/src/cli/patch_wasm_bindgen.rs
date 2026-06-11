use super::*;
use crate::{CliSettings, Workspace};
use krates::semver::Version;
use std::io::IsTerminal;
use std::path::Path;
use std::time::Duration;

/// Patch wasm-bindgen crates to use DioxusLabs fork for WRY compatibility.
#[derive(Clone, Debug, Parser)]
pub(crate) struct PatchWasmBindgen {}

impl PatchWasmBindgen {
    pub(crate) async fn patch_wasm_bindgen(self) -> Result<StructuredOutput> {
        let workspace = Workspace::current().await?;
        let Some(patch) = WasmBindgenPatch::new(&workspace) else {
            tracing::info!("No wasm-bindgen version found in workspace; skipping patch.");
            return Ok(StructuredOutput::Success);
        };

        let Some(tag) = patch.find_tag().await? else {
            tracing::warn!("{}", patch.no_matching_tag_message());
            return Ok(StructuredOutput::Success);
        };

        let outcome = patch.apply(&tag)?;
        for crate_name in &outcome.skipped {
            tracing::info!(
                "Left `{crate_name}` alone: [patch.crates-io] already redirects it to a custom source."
            );
        }
        if outcome.changed() {
            tracing::info!(
                "Patch applied to Cargo.toml (tag: {tag}; {})",
                outcome.summary()
            );
        } else {
            tracing::info!("Cargo.toml is already patched (tag: {tag}); nothing to change.");
        }
        Ok(StructuredOutput::Success)
    }
}

/// Check if we should prompt the user to apply the wasm-bindgen patch.
/// Called during desktop builds to offer patching.
pub(crate) async fn check_wasm_bindgen_patch_prompt(workspace: &Workspace) -> Result<()> {
    let verbosity = crate::logging::verbosity_or_default();

    // Only prompt in interactive TUI mode: never in CI, when output is piped or machine-read
    // JSON, or when the user asked to stay off the network.
    if CliSettings::is_ci()
        || verbosity.json_output
        || verbosity.offline
        || !std::io::stdout().is_terminal()
    {
        return Ok(());
    }

    // Only patch projects that actually run wasm-bindgen code through the wry-bindgen runtime:
    // dioxus-desktop pulls in the `wasm-bindgen-x` shim. Projects that merely have upstream
    // wasm-bindgen somewhere in their graph (reqwest, chrono, web-time, ...) don't need — and
    // must not get — the patch.
    let Some(patch) = WasmBindgenPatch::new(workspace) else {
        return Ok(());
    };
    let Some(fork_pin) = patch.fork_pin.clone() else {
        return Ok(());
    };

    // Skip if already prompted about this pin for this workspace
    if DxHints::was_prompted(&patch.workspace_root, &fork_pin) {
        return Ok(());
    }

    if !patch.cargo_toml.exists() {
        return Ok(());
    }

    match patch.status()? {
        PatchStatus::UpToDate => {
            DxHints::mark_prompted(&patch.workspace_root, &fork_pin)?;
            return Ok(());
        }
        PatchStatus::UserManaged => {
            tracing::debug!(
                "[patch.crates-io] already redirects wasm-bindgen crates to a custom source; \
                 leaving it alone."
            );
            return Ok(());
        }
        PatchStatus::NeedsApply => {}
    }

    // Find a usable tag *before* prompting. When the fetch fails or no published tag matches,
    // skip quietly without marking the workspace as prompted so a future build retries once the
    // tag exists.
    let tag = match patch.find_tag().await {
        Ok(Some(tag)) => tag,
        Ok(None) => {
            tracing::debug!("{}", patch.no_matching_tag_message());
            return Ok(());
        }
        Err(err) => {
            tracing::debug!("Could not fetch wasm-bindgen-wry tags: {err}");
            return Ok(());
        }
    };

    // Show prompt
    let term = console::Term::stdout();
    term.write_str("Your project may use wasm-bindgen crates (web-sys, etc).\n")?;
    term.write_str("For desktop builds, these need a compatibility patch.\n")?;
    term.write_str("\n")?;
    term.write_str("Apply wasm-bindgen patch to Cargo.toml? [Y/n] ")?;
    term.flush()?;

    let input = term.read_line()?;
    let should_patch = input.trim().is_empty() || input.trim().eq_ignore_ascii_case("y");

    if !should_patch {
        // An explicit decline is sticky until the pin changes; don't ask again before then
        DxHints::mark_prompted(&patch.workspace_root, &fork_pin)?;
        term.write_line("Skipped. Run `dx tools patch-wasm-bindgen` later if needed.")?;
        return Ok(());
    }

    // Mark prompted only once the patch is actually written
    let outcome = patch.apply(&tag)?;
    DxHints::mark_prompted(&patch.workspace_root, &fork_pin)?;
    term.write_line(&format!(
        "✓ Patch applied to Cargo.toml (tag: {tag}; {})",
        outcome.summary()
    ))?;

    Ok(())
}

const PATCH_GIT_URL: &str = "https://github.com/DioxusLabs/wasm-bindgen-wry";
const PATCH_GITHUB_REPO: &str = "DioxusLabs/wasm-bindgen-wry";

/// Crates shadowed by the `[patch.crates-io]` entries. Each one exists in the fork repo as a
/// target-switching shim with the same package name and an upstream-compatible version
/// (wasm-bindgen 0.2.x, js-sys/web-sys 0.3.x, wasm-bindgen-futures 0.4.x), so a caret
/// requirement on the upstream crate resolves to the shim. The shims reach the wry-bindgen
/// stack through path dependencies inside the git source, so the wry-* crates need no entries
/// of their own — and can't get any: their registry dependents use exact prerelease pins
/// (`=0.2.123-alpha.N`) while the packages in the git repo carry plain versions, so a wry-*
/// patch entry never applies and cargo warns about an unused patch on every build.
const PATCH_CRATES: &[&str] = &["wasm-bindgen", "wasm-bindgen-futures", "js-sys", "web-sys"];

/// Fork-owned `[patch.crates-io]` entries that are removed when (re)applying the patch. No git
/// tag carries a `wry-bindgen` version that satisfies its registry dependents' exact prerelease
/// pins, so an entry for it sits unused and makes cargo warn on every build.
const STALE_PATCH_CRATES: &[&str] = &["wry-bindgen"];

/// The patch as it applies to one workspace: the manifest to rewrite and the resolved versions
/// a fork tag must shadow.
struct WasmBindgenPatch {
    workspace_root: PathBuf,
    cargo_toml: PathBuf,
    /// The upstream wasm-bindgen version resolved in the workspace's crate graph.
    upstream: Version,
    /// The `wasm-bindgen-x` shim pin pulled in by dioxus-desktop, when it is in the graph. The
    /// fork tag must match it exactly — prerelease included — or the patched git stack carries
    /// a mismatched copy of the wry-bindgen runtime.
    fork_pin: Option<Version>,
}

impl WasmBindgenPatch {
    /// `None` when the workspace doesn't resolve upstream wasm-bindgen at all.
    fn new(workspace: &Workspace) -> Option<Self> {
        let upstream = workspace
            .wasm_bindgen_version()
            .as_deref()
            .and_then(Self::parse_version)?;
        let workspace_root = workspace.workspace_root();
        Some(Self {
            cargo_toml: workspace_root.join("Cargo.toml"),
            workspace_root,
            upstream,
            fork_pin: workspace.wasm_bindgen_fork_version(),
        })
    }

    /// Find the best fork tag for the workspace's resolved versions. The tag's base version must
    /// be able to shadow the upstream version (same `major.minor`, base `>=` it), and it must
    /// match the `wasm-bindgen-x` pin exactly when one exists.
    ///
    /// `Err` means the tag list could not be fetched (network failure); `Ok(None)` means no
    /// published tag is compatible — the two cases warrant different handling at the prompt. On
    /// `Ok(None)` the caller must skip patching rather than write a known-broken patch.
    async fn find_tag(&self) -> Result<Option<String>> {
        let base = |v: &Version| Version::new(v.major, v.minor, v.patch);
        Ok(Self::fetch_available_tags()
            .await?
            .iter()
            .filter_map(|tag| Some((tag, Self::parse_version(tag)?)))
            .filter(|(_, v)| v.major == self.upstream.major && v.minor == self.upstream.minor)
            .filter(|(_, v)| base(v) >= base(&self.upstream))
            .filter(|(_, v)| self.fork_pin.as_ref().is_none_or(|pin| v == pin))
            .max_by(|(_, a), (_, b)| a.cmp(b))
            .map(|(tag, _)| tag.clone()))
    }

    /// Fetch all available tags from the GitHub repository, following pagination so exact pins
    /// stay findable once the repo has more than a page of tags. Capped at 10 pages (1000 tags)
    /// to bound the number of API calls.
    async fn fetch_available_tags() -> Result<Vec<String>> {
        const PER_PAGE: usize = 100;
        const MAX_PAGES: usize = 10;

        #[derive(serde::Deserialize)]
        struct Tag {
            name: String,
        }

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()?;

        let mut tags = Vec::new();
        for page in 1..=MAX_PAGES {
            let url = format!(
                "https://api.github.com/repos/{PATCH_GITHUB_REPO}/tags?per_page={PER_PAGE}&page={page}"
            );

            let request = client.get(&url).header("User-Agent", "dioxus-cli");
            let response = request.send().await?;

            if !response.status().is_success() {
                return Err(anyhow::anyhow!(
                    "Failed to fetch tags from GitHub: {}",
                    response.status()
                ));
            }

            let page_tags: Vec<Tag> = response.json().await?;
            let page_len = page_tags.len();
            tags.extend(page_tags.into_iter().map(|t| t.name));
            if page_len < PER_PAGE {
                break;
            }
        }
        Ok(tags)
    }

    /// The user-facing explanation for skipping the patch when no tag matches.
    fn no_matching_tag_message(&self) -> String {
        let upstream = &self.upstream;
        let constraint = match &self.fork_pin {
            Some(pin) => format!("wasm-bindgen-x {pin} (upstream wasm-bindgen {upstream})"),
            None => format!("wasm-bindgen {upstream}"),
        };
        format!(
            "No published wasm-bindgen-wry tag matches {constraint}; skipping the patch since it \
             would produce a broken build. Re-run `dx tools patch-wasm-bindgen` after the next fork \
             release."
        )
    }

    /// Classify the manifest's patch entries against the workspace's `wasm-bindgen-x` pin. The
    /// tag written for a pinned workspace always carries the pin's exact version, so this check
    /// needs no network round-trip. Without a pin nothing can be confirmed current, so the
    /// manifest always counts as needing an apply.
    fn status(&self) -> Result<PatchStatus> {
        let content = std::fs::read_to_string(&self.cargo_toml)?;
        let doc: toml_edit::DocumentMut = content
            .parse()
            .map_err(|e| anyhow::anyhow!("Failed to parse Cargo.toml: {}", e))?;
        // The [patch.crates-io] table, if present in any TOML spelling
        let crates_io = doc
            .get("patch")
            .and_then(|item| item.as_table_like())
            .and_then(|patch| patch.get("crates-io"))
            .and_then(|item| item.as_table_like());

        let entries: Vec<_> = PATCH_CRATES
            .iter()
            .map(|crate_name| PatchEntry::classify(crates_io, crate_name))
            .collect();

        if entries.contains(&PatchEntry::UserManaged) {
            return Ok(PatchStatus::UserManaged);
        }

        let Some(pin) = &self.fork_pin else {
            return Ok(PatchStatus::NeedsApply);
        };
        let up_to_date = |entry: &PatchEntry| match entry {
            PatchEntry::Fork { tag } => Self::parse_version(tag).as_ref() == Some(pin),
            _ => false,
        };
        if !entries.iter().all(up_to_date) {
            return Ok(PatchStatus::NeedsApply);
        }

        // Lingering fork-owned entries (e.g. written by older CLI versions) make cargo warn
        // about an unused patch on every build; applying removes them.
        let stale = STALE_PATCH_CRATES.iter().any(|crate_name| {
            matches!(
                PatchEntry::classify(crates_io, crate_name),
                PatchEntry::Fork { .. }
            )
        });
        if stale {
            return Ok(PatchStatus::NeedsApply);
        }

        Ok(PatchStatus::UpToDate)
    }

    /// Apply the patch to the manifest: add missing entries, retag fork-owned entries that point
    /// at another release, remove stale fork-owned entries, and leave user-managed entries
    /// untouched. The file is only rewritten when something actually changed.
    fn apply(&self, tag: &str) -> Result<PatchOutcome> {
        let content = std::fs::read_to_string(&self.cargo_toml)?;
        let mut doc: toml_edit::DocumentMut = content
            .parse()
            .map_err(|e| anyhow::anyhow!("Failed to parse Cargo.toml: {}", e))?;

        // Get or create the [patch.crates-io] section. A newly created [patch] table is implicit
        // so the written manifest contains only the [patch.crates-io] header, not a stray bare
        // [patch].
        let patch = doc.entry("patch").or_insert_with(|| {
            let mut table = toml_edit::Table::new();
            table.set_implicit(true);
            toml_edit::Item::Table(table)
        });
        let patch_table = patch
            .as_table_mut()
            .ok_or_else(|| anyhow::anyhow!("[patch] is not a table"))?;

        let crates_io = patch_table
            .entry("crates-io")
            .or_insert_with(|| toml_edit::Item::Table(toml_edit::Table::new()));
        let crates_io_table = crates_io
            .as_table_like_mut()
            .ok_or_else(|| anyhow::anyhow!("[patch.crates-io] is not a table"))?;

        let mut outcome = PatchOutcome::default();
        for &crate_name in PATCH_CRATES {
            match PatchEntry::classify(Some(&*crates_io_table), crate_name) {
                PatchEntry::Missing => {
                    let mut inline = toml_edit::InlineTable::new();
                    inline.insert("git", PATCH_GIT_URL.into());
                    inline.insert("tag", tag.into());
                    crates_io_table.insert(crate_name, toml_edit::Item::Value(inline.into()));
                    outcome.added.push(crate_name);
                }
                PatchEntry::Fork { tag: existing } => {
                    if existing != tag {
                        crates_io_table
                            .get_mut(crate_name)
                            .and_then(|item| item.as_table_like_mut())
                            .expect("fork entries are table-like")
                            .insert("tag", toml_edit::value(tag));
                        outcome.updated.push(crate_name);
                    }
                }
                PatchEntry::UserManaged => outcome.skipped.push(crate_name),
            }
        }

        for &crate_name in STALE_PATCH_CRATES {
            if matches!(
                PatchEntry::classify(Some(&*crates_io_table), crate_name),
                PatchEntry::Fork { .. }
            ) {
                crates_io_table.remove(crate_name);
                outcome.removed.push(crate_name);
            }
        }

        if outcome.changed() {
            std::fs::write(&self.cargo_toml, doc.to_string())?;
        }
        Ok(outcome)
    }

    /// Parse a version string, stripping the leading 'v' if present
    fn parse_version(version: &str) -> Option<Version> {
        Version::parse(version.trim_start_matches('v')).ok()
    }
}

/// The aggregate state of our `[patch.crates-io]` entries in a manifest.
#[derive(Debug, PartialEq)]
enum PatchStatus {
    /// Every entry is present, fork-owned, and pinned to the wanted release; nothing to do.
    UpToDate,
    /// Entries are missing, fork-owned entries point at another release, or stale fork-owned
    /// entries linger: applying would change the manifest.
    NeedsApply,
    /// At least one entry redirects part of the wasm-bindgen stack to a user-managed source;
    /// writing the remaining entries against the dioxus fork would mix two incompatible copies,
    /// so leave the manifest alone entirely.
    UserManaged,
}

/// How a single `[patch.crates-io]` entry relates to the dioxus fork.
#[derive(Debug, PartialEq)]
enum PatchEntry {
    /// No entry for this crate
    Missing,
    /// Points at the dioxus fork, pinned by `tag`: ours to update or remove
    Fork { tag: String },
    /// Anything else: another git URL, a path, or a fork checkout pinned by branch/rev. The user
    /// manages this entry; never overwrite it.
    UserManaged,
}

impl PatchEntry {
    fn classify(crates_io: Option<&dyn toml_edit::TableLike>, crate_name: &str) -> Self {
        let Some(item) = crates_io.and_then(|table| table.get(crate_name)) else {
            return Self::Missing;
        };
        let entry = item.as_table_like();
        // Match the fork repo URL in its usual spellings (`.git` suffix, trailing slash, casing)
        let is_fork_url = |url: &str| {
            url.trim_end_matches('/')
                .trim_end_matches(".git")
                .trim_end_matches('/')
                .eq_ignore_ascii_case(PATCH_GIT_URL)
        };
        let git = entry
            .and_then(|table| table.get("git"))
            .and_then(|item| item.as_str());
        if !git.is_some_and(is_fork_url) {
            return Self::UserManaged;
        }
        match entry
            .and_then(|table| table.get("tag"))
            .and_then(|item| item.as_str())
        {
            Some(tag) => Self::Fork {
                tag: tag.to_string(),
            },
            None => Self::UserManaged,
        }
    }
}

/// What [`WasmBindgenPatch::apply`] did to the manifest.
#[derive(Debug, Default, PartialEq)]
struct PatchOutcome {
    /// Entries newly written against the fork
    added: Vec<&'static str>,
    /// Fork-owned entries whose tag was moved
    updated: Vec<&'static str>,
    /// Stale fork-owned entries that were removed
    removed: Vec<&'static str>,
    /// Entries left alone because they point at a user-managed source
    skipped: Vec<&'static str>,
}

impl PatchOutcome {
    /// Whether the manifest on disk was rewritten
    fn changed(&self) -> bool {
        !(self.added.is_empty() && self.updated.is_empty() && self.removed.is_empty())
    }

    /// Human-readable list of what changed, e.g. "added js-sys, web-sys; updated wasm-bindgen"
    fn summary(&self) -> String {
        let mut parts = Vec::new();
        for (verb, crates) in [
            ("added", &self.added),
            ("updated", &self.updated),
            ("removed", &self.removed),
        ] {
            if !crates.is_empty() {
                parts.push(format!("{verb} {}", crates.join(", ")));
            }
        }
        parts.join("; ")
    }
}

/// CLI state for a workspace, stored under the dioxus data dir.
#[derive(Default, serde::Serialize, serde::Deserialize)]
struct DxHints {
    /// The `wasm-bindgen-x` pin the user last answered a patch prompt for. The prompt re-arms
    /// when the workspace moves to a different pin (e.g. a dioxus upgrade) so the patch can be
    /// updated on consent.
    #[serde(default)]
    wasm_bindgen_prompted: Option<String>,
}

impl DxHints {
    /// Check if we've already prompted the user about this pin for this workspace
    fn was_prompted(workspace_root: &Path, fork_pin: &Version) -> bool {
        Self::load(workspace_root).wasm_bindgen_prompted.as_deref()
            == Some(fork_pin.to_string().as_str())
    }

    /// Mark that we've prompted the user about this pin for this workspace
    fn mark_prompted(workspace_root: &Path, fork_pin: &Version) -> Result<()> {
        let mut hints = Self::load(workspace_root);
        hints.wasm_bindgen_prompted = Some(fork_pin.to_string());
        hints.save(workspace_root)
    }

    fn load(workspace_root: &Path) -> Self {
        std::fs::read_to_string(Self::path(workspace_root))
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    fn save(&self, workspace_root: &Path) -> Result<()> {
        let path = Self::path(workspace_root);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, serde_json::to_string_pretty(self)?)?;
        Ok(())
    }

    /// Path to the hints file for this workspace, keyed by a hash of the workspace root under
    /// the dioxus data dir (same pattern as the component cache). Keeping it out of the cargo
    /// target dir means `cargo clean` doesn't re-arm the prompt and a shared `CARGO_TARGET_DIR`
    /// doesn't conflate unrelated workspaces. The hash is FNV-1a because the file name must be
    /// stable across releases and std's `DefaultHasher` makes no such guarantee.
    fn path(workspace_root: &Path) -> PathBuf {
        use std::hash::Hasher;

        let mut hasher = fnv::FnvHasher::default();
        hasher.write(workspace_root.as_os_str().as_encoded_bytes());
        let hash = hasher.finish();
        Workspace::dioxus_data_dir()
            .join("hints")
            .join(format!("{hash:016x}.json"))
    }
}
