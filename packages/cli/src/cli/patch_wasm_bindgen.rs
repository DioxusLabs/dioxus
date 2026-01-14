use super::*;
use crate::{CliSettings, Workspace};
use krates::semver::{Version, VersionReq};
use std::io::IsTerminal;
use std::path::Path;

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

    let client = reqwest::Client::new();
    let response = client
        .get(&url)
        .header("User-Agent", "dioxus-cli")
        .send()
        .await?;

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

/// Find the best matching tag for the given wasm-bindgen version using semver compatibility
fn find_best_matching_tag(target_version: &str, available_tags: &[String]) -> Option<String> {
    let target = parse_version(target_version)?;

    // Create a semver requirement for compatible versions (^x.y.z)
    let version_req = VersionReq::parse(&format!("^{}", target)).ok()?;

    // Parse all available tags and filter to valid versions
    let mut parsed_tags: Vec<(String, Version)> = available_tags
        .iter()
        .filter_map(|tag| {
            let parsed = parse_version(tag)?;
            Some((tag.clone(), parsed))
        })
        .collect();

    // Sort by version (descending) so we prefer newer versions
    parsed_tags.sort_by(|a, b| b.1.cmp(&a.1));

    // First, try to find an exact match
    if let Some((tag, _)) = parsed_tags.iter().find(|(_, v)| v == &target) {
        return Some(tag.clone());
    }

    // Second, find the newest semver-compatible version
    if let Some((tag, _)) = parsed_tags.iter().find(|(_, v)| version_req.matches(v)) {
        return Some(tag.clone());
    }

    // Finally, just return the newest available tag as a fallback
    parsed_tags.first().map(|(tag, _)| tag.clone())
}

/// Get the best matching tag for the workspace's wasm-bindgen version
pub(crate) async fn get_matching_patch_tag(wasm_bindgen_version: &str) -> Result<String> {
    let available_tags = fetch_available_tags().await?;

    find_best_matching_tag(wasm_bindgen_version, &available_tags).ok_or_else(|| {
        anyhow::anyhow!(
            "No compatible wasm-bindgen-wry tag found for version {}",
            wasm_bindgen_version
        )
    })
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

/// Path to the hints file that stores CLI state for this workspace
fn hints_file_path(workspace_root: &Path) -> PathBuf {
    workspace_root.join("target").join("dx").join(".dx-hints")
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

    // Get or create the [patch.crates-io] section
    let patch = doc
        .entry("patch")
        .or_insert_with(|| toml_edit::Item::Table(toml_edit::Table::new()));
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

    let workspace_root = workspace.krates.workspace_root().as_std_path();

    // Only try to patch if we have a wasm-bindgen version
    let Some(wasm_bindgen_version) = workspace.wasm_bindgen_version() else {
        return Ok(());
    };

    // Skip if already prompted for this workspace
    if was_prompted(workspace_root) {
        return Ok(());
    }

    let cargo_toml = workspace_root.join("Cargo.toml");

    // Skip if patch already exists in Cargo.toml
    if !needs_wasm_bindgen_patch(&cargo_toml)? {
        mark_prompted(workspace_root)?;
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

    // Mark as prompted so we don't ask again
    mark_prompted(workspace_root)?;

    if should_patch {
        let tag = get_matching_patch_tag(&wasm_bindgen_version).await?;
        apply_wasm_bindgen_patch(&cargo_toml, &tag)?;
        term.write_line(&format!("✓ Patch applied to Cargo.toml (tag: {})", tag))?;
    } else {
        term.write_line("Skipped. Run `dx patch-wasm-bindgen` later if needed.")?;
    }

    Ok(())
}

impl PatchWasmBindgen {
    pub(crate) async fn patch_wasm_bindgen(self) -> Result<StructuredOutput> {
        let workspace = Workspace::current().await?;
        let workspace_root = workspace.krates.workspace_root().as_std_path();
        let cargo_toml = workspace_root.join("Cargo.toml");
        let Some(wasm_bindgen_version) = workspace.wasm_bindgen_version() else {
            tracing::info!("No wasm-bindgen version found in workspace; skipping patch.");
            return Ok(StructuredOutput::Success);
        };

        let tag = get_matching_patch_tag(&wasm_bindgen_version).await?;
        apply_wasm_bindgen_patch(&cargo_toml, &tag)?;
        tracing::info!("Patch applied to Cargo.toml (tag: {})", tag);
        Ok(StructuredOutput::Success)
    }
}
