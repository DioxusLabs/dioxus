use super::*;
use crate::{CliSettings, Workspace};
use krates::semver::Version;
use std::io::IsTerminal;
use std::path::Path;
use std::time::Duration;

/// Patch wasm-bindgen crates to use DioxusLabs fork for WRY compatibility.
#[derive(Clone, Debug, Parser)]
pub(crate) struct PatchWasmBindgen {}

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

/// Fetch all available tags from the GitHub repository, following pagination so exact pins stay
/// findable once the repo has more than a page of tags. Capped at 10 pages (1000 tags) to bound
/// the number of API calls.
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

        let mut request = client.get(&url).header("User-Agent", "dioxus-cli");
        // Use the user's token when available to dodge the 60 req/hr unauthenticated rate limit
        if let Ok(token) = std::env::var("GITHUB_TOKEN") {
            request = request.header("Authorization", format!("Bearer {token}"));
        }
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

/// Parse a version string, stripping the leading 'v' if present
fn parse_version(version: &str) -> Option<Version> {
    Version::parse(version.trim_start_matches('v')).ok()
}

/// Find the best matching tag for the resolved upstream wasm-bindgen version. The tag's base
/// version must be able to shadow the upstream version (same `major.minor`, base `>=` it), and
/// when the fork (`wasm-bindgen-x`, pulled in by dioxus-desktop) is in the graph the tag must
/// match its pin exactly — prerelease included — or the patched git stack carries a mismatched
/// copy of the wry-bindgen runtime. Returns `None` when no published tag qualifies; the caller
/// must skip patching rather than write a known-broken patch.
fn find_best_matching_tag(
    upstream_version: &Version,
    fork_pin: Option<&Version>,
    available_tags: &[String],
) -> Option<String> {
    let base = |v: &Version| Version::new(v.major, v.minor, v.patch);

    available_tags
        .iter()
        .filter_map(|tag| Some((tag, parse_version(tag)?)))
        .filter(|(_, v)| v.major == upstream_version.major && v.minor == upstream_version.minor)
        .filter(|(_, v)| base(v) >= base(upstream_version))
        .filter(|(_, v)| fork_pin.is_none_or(|pin| v == pin))
        .max_by(|(_, a), (_, b)| a.cmp(b))
        .map(|(tag, _)| tag.clone())
}

/// Get the best matching tag for the workspace's resolved wasm-bindgen versions.
///
/// `Err` means the tag list could not be fetched (network failure); `Ok(None)` means no published
/// tag is compatible — the two cases warrant different handling at the prompt.
async fn get_matching_patch_tag(
    upstream_version: &Version,
    fork_pin: Option<&Version>,
) -> Result<Option<String>> {
    let available_tags = fetch_available_tags().await?;
    Ok(find_best_matching_tag(
        upstream_version,
        fork_pin,
        &available_tags,
    ))
}

/// The user-facing explanation for skipping the patch when no tag matches.
fn no_matching_tag_message(upstream: &Version, fork_pin: Option<&Version>) -> String {
    let constraint = match fork_pin {
        Some(pin) => format!("wasm-bindgen-x {pin} (upstream wasm-bindgen {upstream})"),
        None => format!("wasm-bindgen {upstream}"),
    };
    format!(
        "No published wasm-bindgen-wry tag matches {constraint}; skipping the patch since it \
         would produce a broken build. Re-run `dx tools patch-wasm-bindgen` after the next fork \
         release."
    )
}

/// Whether a `git` value in a patch entry points at the dioxus fork repo, tolerating the usual
/// URL spellings (`.git` suffix, trailing slash, different casing).
fn is_fork_url(url: &str) -> bool {
    let normalized = url
        .trim_end_matches('/')
        .trim_end_matches(".git")
        .trim_end_matches('/');
    normalized.eq_ignore_ascii_case(PATCH_GIT_URL)
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

fn classify_entry(crates_io: Option<&dyn toml_edit::TableLike>, crate_name: &str) -> PatchEntry {
    let Some(item) = crates_io.and_then(|table| table.get(crate_name)) else {
        return PatchEntry::Missing;
    };
    let entry = item.as_table_like();
    let git = entry
        .and_then(|table| table.get("git"))
        .and_then(|item| item.as_str());
    if !git.is_some_and(is_fork_url) {
        return PatchEntry::UserManaged;
    }
    match entry
        .and_then(|table| table.get("tag"))
        .and_then(|item| item.as_str())
    {
        Some(tag) => PatchEntry::Fork {
            tag: tag.to_string(),
        },
        None => PatchEntry::UserManaged,
    }
}

/// The `[patch.crates-io]` table of a parsed manifest, if present in any TOML spelling.
fn patch_crates_io_table(doc: &toml_edit::DocumentMut) -> Option<&dyn toml_edit::TableLike> {
    doc.get("patch")?
        .as_table_like()?
        .get("crates-io")?
        .as_table_like()
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

/// Classify the manifest's patch entries against the workspace's `wasm-bindgen-x` pin. The tag
/// written for a pinned workspace always carries the pin's exact version, so this check needs no
/// network round-trip.
fn wasm_bindgen_patch_status(cargo_toml_path: &Path, fork_pin: &Version) -> Result<PatchStatus> {
    let content = std::fs::read_to_string(cargo_toml_path)?;
    let doc: toml_edit::DocumentMut = content
        .parse()
        .map_err(|e| anyhow::anyhow!("Failed to parse Cargo.toml: {}", e))?;
    let crates_io = patch_crates_io_table(&doc);

    let entries: Vec<_> = PATCH_CRATES
        .iter()
        .map(|crate_name| classify_entry(crates_io, crate_name))
        .collect();

    if entries.contains(&PatchEntry::UserManaged) {
        return Ok(PatchStatus::UserManaged);
    }

    let up_to_date = |entry: &PatchEntry| match entry {
        PatchEntry::Fork { tag } => parse_version(tag).as_ref() == Some(fork_pin),
        _ => false,
    };
    if !entries.iter().all(up_to_date) {
        return Ok(PatchStatus::NeedsApply);
    }

    // Lingering fork-owned entries (e.g. written by older CLI versions) make cargo warn about an
    // unused patch on every build; applying removes them.
    let stale = STALE_PATCH_CRATES.iter().any(|crate_name| {
        matches!(
            classify_entry(crates_io, crate_name),
            PatchEntry::Fork { .. }
        )
    });
    if stale {
        return Ok(PatchStatus::NeedsApply);
    }

    Ok(PatchStatus::UpToDate)
}

/// FNV-1a over the path bytes. Implemented inline because the hints file name must be stable
/// across releases, and std's `DefaultHasher` makes no such guarantee.
fn stable_path_hash(path: &Path) -> u64 {
    const FNV_OFFSET: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x100000001b3;

    let mut hash = FNV_OFFSET;
    for &byte in path.as_os_str().as_encoded_bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

/// Path to the hints file that stores CLI state for this workspace, keyed by a hash of the
/// workspace root under the dioxus data dir (same pattern as the component cache). Keeping it
/// out of the cargo target dir means `cargo clean` doesn't re-arm the prompt and a shared
/// `CARGO_TARGET_DIR` doesn't conflate unrelated workspaces.
fn hints_file_path(workspace_root: &Path) -> PathBuf {
    let hash = stable_path_hash(workspace_root);
    Workspace::dioxus_data_dir()
        .join("hints")
        .join(format!("{hash:016x}.json"))
}

#[derive(Default, serde::Serialize, serde::Deserialize)]
struct DxHints {
    /// The `wasm-bindgen-x` pin the user last answered a patch prompt for. The prompt re-arms
    /// when the workspace moves to a different pin (e.g. a dioxus upgrade) so the patch can be
    /// updated on consent.
    #[serde(default)]
    wasm_bindgen_prompted: Option<String>,
}

fn load_hints(workspace_root: &Path) -> DxHints {
    let path = hints_file_path(workspace_root);
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_hints(workspace_root: &Path, hints: &DxHints) -> Result<()> {
    let path = hints_file_path(workspace_root);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(hints)?;
    std::fs::write(&path, json)?;
    Ok(())
}

/// Check if we've already prompted the user about this pin for this workspace
fn was_prompted(workspace_root: &Path, fork_pin: &Version) -> bool {
    load_hints(workspace_root).wasm_bindgen_prompted.as_deref()
        == Some(fork_pin.to_string().as_str())
}

/// Mark that we've prompted the user about this pin for this workspace
fn mark_prompted(workspace_root: &Path, fork_pin: &Version) -> Result<()> {
    let mut hints = load_hints(workspace_root);
    hints.wasm_bindgen_prompted = Some(fork_pin.to_string());
    save_hints(workspace_root, &hints)
}

/// What [`apply_wasm_bindgen_patch`] did to the manifest.
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

/// Apply the wasm-bindgen patch to a Cargo.toml file: add missing entries, retag fork-owned
/// entries that point at another release, remove stale fork-owned entries, and leave
/// user-managed entries untouched. The file is only rewritten when something actually changed.
fn apply_wasm_bindgen_patch(cargo_toml_path: &Path, tag: &str) -> Result<PatchOutcome> {
    let content = std::fs::read_to_string(cargo_toml_path)?;
    let mut doc: toml_edit::DocumentMut = content
        .parse()
        .map_err(|e| anyhow::anyhow!("Failed to parse Cargo.toml: {}", e))?;

    // Get or create the [patch.crates-io] section. A newly created [patch] table is implicit so
    // the written manifest contains only the [patch.crates-io] header, not a stray bare [patch].
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
        match classify_entry(Some(&*crates_io_table), crate_name) {
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
            classify_entry(Some(&*crates_io_table), crate_name),
            PatchEntry::Fork { .. }
        ) {
            crates_io_table.remove(crate_name);
            outcome.removed.push(crate_name);
        }
    }

    if outcome.changed() {
        std::fs::write(cargo_toml_path, doc.to_string())?;
    }
    Ok(outcome)
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
    let Some(fork_pin) = workspace.wasm_bindgen_fork_version() else {
        return Ok(());
    };
    let Some(upstream) = workspace
        .wasm_bindgen_version()
        .as_deref()
        .and_then(parse_version)
    else {
        return Ok(());
    };

    let workspace_root = workspace.workspace_root();

    // Skip if already prompted about this pin for this workspace
    if was_prompted(&workspace_root, &fork_pin) {
        return Ok(());
    }

    let cargo_toml = workspace_root.join("Cargo.toml");
    if !cargo_toml.exists() {
        return Ok(());
    }

    match wasm_bindgen_patch_status(&cargo_toml, &fork_pin)? {
        PatchStatus::UpToDate => {
            mark_prompted(&workspace_root, &fork_pin)?;
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
    let tag = match get_matching_patch_tag(&upstream, Some(&fork_pin)).await {
        Ok(Some(tag)) => tag,
        Ok(None) => {
            tracing::debug!("{}", no_matching_tag_message(&upstream, Some(&fork_pin)));
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
        mark_prompted(&workspace_root, &fork_pin)?;
        term.write_line("Skipped. Run `dx tools patch-wasm-bindgen` later if needed.")?;
        return Ok(());
    }

    // Mark prompted only once the patch is actually written
    let outcome = apply_wasm_bindgen_patch(&cargo_toml, &tag)?;
    mark_prompted(&workspace_root, &fork_pin)?;
    term.write_line(&format!(
        "✓ Patch applied to Cargo.toml (tag: {tag}; {})",
        outcome.summary()
    ))?;

    Ok(())
}

impl PatchWasmBindgen {
    pub(crate) async fn patch_wasm_bindgen(self) -> Result<StructuredOutput> {
        let workspace = Workspace::current().await?;
        let workspace_root = workspace.krates.workspace_root().as_std_path();
        let cargo_toml = workspace_root.join("Cargo.toml");
        let Some(upstream) = workspace
            .wasm_bindgen_version()
            .as_deref()
            .and_then(parse_version)
        else {
            tracing::info!("No wasm-bindgen version found in workspace; skipping patch.");
            return Ok(StructuredOutput::Success);
        };

        let fork_pin = workspace.wasm_bindgen_fork_version();
        let Some(tag) = get_matching_patch_tag(&upstream, fork_pin.as_ref()).await? else {
            tracing::warn!("{}", no_matching_tag_message(&upstream, fork_pin.as_ref()));
            return Ok(StructuredOutput::Success);
        };

        let outcome = apply_wasm_bindgen_patch(&cargo_toml, &tag)?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use krates::semver::Prerelease;
    use std::sync::OnceLock;

    /// Tags fetched live from DioxusLabs/wasm-bindgen-wry, once per test run.
    /// `None` when the fetch failed (offline / rate-limited); callers skip.
    fn fetched_tags() -> Option<&'static [String]> {
        static TAGS: OnceLock<Option<Vec<String>>> = OnceLock::new();
        TAGS.get_or_init(|| {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(fetch_available_tags())
                .map_err(|e| eprintln!("skipping: could not fetch tags from GitHub: {e}"))
                .ok()
        })
        .as_deref()
    }

    fn parsed_tags(tags: &[String]) -> Vec<Version> {
        let versions: Vec<_> = tags.iter().filter_map(|t| parse_version(t)).collect();
        assert!(!versions.is_empty(), "no parseable tags published");
        versions
    }

    fn v(s: &str) -> Version {
        Version::parse(s).unwrap()
    }

    fn base(v: &Version) -> Version {
        Version::new(v.major, v.minor, v.patch)
    }

    /// Write `content` to a Cargo.toml in a fresh temp dir, returning the dir and the path
    fn manifest(content: &str) -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("Cargo.toml");
        std::fs::write(&path, content).unwrap();
        (dir, path)
    }

    fn status(content: &str, pin: &str) -> PatchStatus {
        let (_dir, path) = manifest(content);
        wasm_bindgen_patch_status(&path, &v(pin)).unwrap()
    }

    const BARE_MANIFEST: &str =
        "# top comment\n[package]\nname = \"demo\" # trailing comment\nversion = \"0.1.0\"\n";

    fn fork_entry(tag: &str) -> String {
        format!("{{ git = \"{PATCH_GIT_URL}\", tag = \"{tag}\" }}")
    }

    fn fully_patched(tag: &str) -> String {
        let entry = fork_entry(tag);
        format!(
            "{BARE_MANIFEST}\n[patch.crates-io]\nwasm-bindgen = {entry}\nwasm-bindgen-futures = {entry}\njs-sys = {entry}\nweb-sys = {entry}\n"
        )
    }

    #[test]
    fn fork_pin_requires_exact_tag() {
        let Some(tags) = fetched_tags() else { return };
        let versions = parsed_tags(tags);

        for pin in &versions {
            // Pinning to a published tag selects exactly that tag
            let selected = find_best_matching_tag(&base(pin), Some(pin), tags)
                .and_then(|tag| parse_version(&tag));
            assert_eq!(selected.as_ref(), Some(pin));

            // A pin with no published tag must select nothing (skip), never a stale tag
            let mut absent = pin.clone();
            absent.pre = Prerelease::new("alpha.99999").unwrap();
            assert!(!versions.contains(&absent));
            assert_eq!(
                find_best_matching_tag(&base(pin), Some(&absent), tags),
                None,
                "pin {absent} must not match any tag"
            );
        }
    }

    #[test]
    fn no_fork_pin_selects_highest_compatible_base() {
        let Some(tags) = fetched_tags() else { return };
        let versions = parsed_tags(tags);

        // Without a fork pin, every published tag's base must be shadowable by a tag from the
        // same minor line that is at least as new
        for tag in &versions {
            let selected = find_best_matching_tag(&base(tag), None, tags)
                .and_then(|t| parse_version(&t))
                .expect("a published tag must match its own base version");
            assert!(versions.contains(&selected));
            assert_eq!((selected.major, selected.minor), (tag.major, tag.minor));
            assert!(selected >= *tag);
        }

        // An upstream newer than every tag base cannot be shadowed
        let newest = versions.iter().map(base).max().unwrap();
        let beyond = Version::new(newest.major, newest.minor, newest.patch + 1);
        assert_eq!(find_best_matching_tag(&beyond, None, tags), None);

        // A minor line with no published tags never matches
        assert!(versions.iter().all(|t| (t.major, t.minor) != (0, 999)));
        assert_eq!(find_best_matching_tag(&v("0.999.0"), None, tags), None);
    }

    #[test]
    fn stable_tag_outranks_prereleases() {
        let Some(tags) = fetched_tags() else { return };
        let newest = parsed_tags(tags).into_iter().max().unwrap();
        let stable = base(&newest);

        let mut tags = tags.to_vec();
        tags.push(format!("v{stable}"));
        assert_eq!(
            find_best_matching_tag(&stable, None, &tags),
            Some(format!("v{stable}"))
        );
    }
}
