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

const PATCH_CRATES: &[&str] = &[
    "wasm-bindgen",
    "wasm-bindgen-futures",
    "js-sys",
    "web-sys",
    "wry-bindgen",
];

/// Fetch available tags from the GitHub repository
async fn fetch_available_tags() -> Result<Vec<String>> {
    let url = format!(
        "https://api.github.com/repos/{}/tags?per_page=100",
        PATCH_GITHUB_REPO
    );

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()?;
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

    #[derive(serde::Deserialize)]
    struct Tag {
        name: String,
    }

    let tags: Vec<Tag> = response.json().await?;
    Ok(tags.into_iter().map(|t| t.name).collect())
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
pub(crate) async fn get_matching_patch_tag(
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

/// Check if the wasm-bindgen patch is needed (i.e., not all patches are applied)
pub(crate) fn needs_wasm_bindgen_patch(cargo_toml_path: &Path) -> Result<bool> {
    if !cargo_toml_path.exists() {
        return Ok(false);
    }

    let content = std::fs::read_to_string(cargo_toml_path)?;
    let doc: toml_edit::DocumentMut = content
        .parse()
        .map_err(|e| anyhow::anyhow!("Failed to parse Cargo.toml: {}", e))?;

    // Check if [patch.crates-io] has all of our crates
    if let Some(patch) = doc.get("patch") {
        if let Some(crates_io) = patch.get("crates-io") {
            if let Some(table) = crates_io.as_table() {
                let all_patched = PATCH_CRATES
                    .iter()
                    .all(|crate_name| table.contains_key(crate_name));
                if all_patched {
                    return Ok(false);
                }
            }
        }
    }

    // Some patches are missing, it's needed
    Ok(true)
}

/// Path to the hints file that stores CLI state for this workspace, keyed by a hash of the
/// workspace root under the dioxus data dir (same pattern as the component cache). Keeping it
/// out of the cargo target dir means `cargo clean` doesn't re-arm the prompt and a shared
/// `CARGO_TARGET_DIR` doesn't conflate unrelated workspaces.
fn hints_file_path(workspace_root: &Path) -> PathBuf {
    use std::hash::Hasher;

    let mut hasher = std::hash::DefaultHasher::new();
    std::hash::Hash::hash(workspace_root, &mut hasher);
    let hash = hasher.finish();
    Workspace::dioxus_data_dir()
        .join("hints")
        .join(format!("{hash:016x}.json"))
}

#[derive(Default, serde::Serialize, serde::Deserialize)]
struct DxHints {
    #[serde(default)]
    wasm_bindgen_prompted: bool,
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

/// Check if we've already prompted the user for this workspace
pub(crate) fn was_prompted(workspace_root: &Path) -> bool {
    load_hints(workspace_root).wasm_bindgen_prompted
}

/// Mark that we've prompted the user for this workspace
pub(crate) fn mark_prompted(workspace_root: &Path) -> Result<()> {
    let mut hints = load_hints(workspace_root);
    hints.wasm_bindgen_prompted = true;
    save_hints(workspace_root, &hints)
}

/// Apply the wasm-bindgen patch to a Cargo.toml file
pub(crate) fn apply_wasm_bindgen_patch(cargo_toml_path: &Path, tag: &str) -> Result<()> {
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
        .as_table_mut()
        .ok_or_else(|| anyhow::anyhow!("[patch.crates-io] is not a table"))?;

    for &crate_name in PATCH_CRATES {
        if !crates_io_table.contains_key(crate_name) {
            let mut inline = toml_edit::InlineTable::new();
            inline.insert("git", PATCH_GIT_URL.into());
            inline.insert("tag", tag.into());
            crates_io_table.insert(crate_name, toml_edit::Item::Value(inline.into()));
        }
    }

    std::fs::write(cargo_toml_path, doc.to_string())?;
    Ok(())
}

/// Check if we should prompt the user to apply the wasm-bindgen patch.
/// Called during desktop builds to offer patching.
pub(crate) async fn check_wasm_bindgen_patch_prompt(workspace: &Workspace) -> Result<()> {
    // Only prompt in interactive TUI mode (not in CI or piped)
    if CliSettings::is_ci() || !std::io::stdout().is_terminal() {
        return Ok(());
    }

    let workspace_root = workspace.workspace_root();

    // Only try to patch if we have an upstream wasm-bindgen version
    let Some(upstream) = workspace
        .wasm_bindgen_version()
        .as_deref()
        .and_then(parse_version)
    else {
        return Ok(());
    };

    // Skip if already prompted for this workspace
    if was_prompted(&workspace_root) {
        return Ok(());
    }

    let cargo_toml = workspace_root.join("Cargo.toml");

    // Skip if patch already exists in Cargo.toml
    if !needs_wasm_bindgen_patch(&cargo_toml)? {
        mark_prompted(&workspace_root)?;
        return Ok(());
    }

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
        // An explicit decline is sticky; don't ask again for this workspace
        mark_prompted(&workspace_root)?;
        term.write_line("Skipped. Run `dx tools patch-wasm-bindgen` later if needed.")?;
        return Ok(());
    }

    // A network failure propagates *without* marking the workspace as prompted, so the user is
    // asked again on the next build instead of silently never getting the patch.
    let fork_pin = workspace.wasm_bindgen_fork_version();
    match get_matching_patch_tag(&upstream, fork_pin.as_ref()).await? {
        Some(tag) => {
            // Mark prompted only once the patch is actually written
            apply_wasm_bindgen_patch(&cargo_toml, &tag)?;
            mark_prompted(&workspace_root)?;
            term.write_line(&format!("✓ Patch applied to Cargo.toml (tag: {})", tag))?;
        }
        // No suitable tag exists yet: warn-and-skip, and leave the prompt armed so it re-asks
        // once a matching fork release is published.
        None => term.write_line(&no_matching_tag_message(&upstream, fork_pin.as_ref()))?,
    }

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
        match get_matching_patch_tag(&upstream, fork_pin.as_ref()).await? {
            Some(tag) => {
                apply_wasm_bindgen_patch(&cargo_toml, &tag)?;
                tracing::info!("Patch applied to Cargo.toml (tag: {})", tag);
            }
            None => tracing::warn!("{}", no_matching_tag_message(&upstream, fork_pin.as_ref())),
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

    #[test]
    fn fork_pin_requires_exact_tag() {
        let Some(tags) = fetched_tags() else { return };
        let versions = parsed_tags(tags);

        for pin in &versions {
            // Pinning to a published tag selects exactly that tag
            let selected = find_best_matching_tag(&base(pin), Some(pin), tags)
                .and_then(|tag| parse_version(&tag));
            assert_eq!(selected.as_ref(), Some(pin));

            // A pin with no published tag must select nothing (warn-and-skip), never a stale tag
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
