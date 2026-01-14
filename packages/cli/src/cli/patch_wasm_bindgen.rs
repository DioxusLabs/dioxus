use super::*;
use crate::{TraceSrc, Workspace};
use krates::semver::{Version, VersionReq};
use std::path::Path;

/// Patch wasm-bindgen crates to use DioxusLabs fork for WRY compatibility.
#[derive(Clone, Debug, Parser)]
pub(crate) struct PatchWasmBindgen {
    /// Overwrite existing wasm-bindgen patches if they exist
    #[clap(long)]
    pub(crate) force: bool,
}

const PATCH_GIT_URL: &str = "https://github.com/DioxusLabs/wasm-bindgen-wry";
const PATCH_GITHUB_REPO: &str = "DioxusLabs/wasm-bindgen-wry";

const PATCH_CRATES: &[&str] = &["wasm-bindgen", "wasm-bindgen-futures", "js-sys", "web-sys"];

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
pub(crate) async fn get_matching_patch_tag(workspace: &Workspace) -> Result<String> {
    let wasm_bindgen_version = workspace
        .wasm_bindgen_version()
        .unwrap_or_else(|| "0.2.99".to_string());

    let available_tags = fetch_available_tags().await?;

    find_best_matching_tag(&wasm_bindgen_version, &available_tags).ok_or_else(|| {
        anyhow::anyhow!(
            "No compatible wasm-bindgen-wry tag found for version {}",
            wasm_bindgen_version
        )
    })
}

/// Check if the wasm-bindgen patch is needed (i.e., not already applied)
pub(crate) fn needs_wasm_bindgen_patch(cargo_toml_path: &Path) -> Result<bool> {
    if !cargo_toml_path.exists() {
        return Ok(false);
    }

    let content = std::fs::read_to_string(cargo_toml_path)?;
    let doc: toml_edit::DocumentMut = content
        .parse()
        .map_err(|e| anyhow::anyhow!("Failed to parse Cargo.toml: {}", e))?;

    // Check if [patch.crates-io] has any of our crates
    if let Some(patch) = doc.get("patch") {
        if let Some(crates_io) = patch.get("crates-io") {
            if let Some(table) = crates_io.as_table() {
                for crate_name in PATCH_CRATES {
                    if table.contains_key(crate_name) {
                        // Patch already exists
                        return Ok(false);
                    }
                }
            }
        }
    }

    // No patch found, it's needed
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

    for crate_name in PATCH_CRATES {
        if !crates_io_table.contains_key(crate_name) {
            let mut inline = toml_edit::InlineTable::new();
            inline.insert("git", toml_edit::Value::from(PATCH_GIT_URL));
            inline.insert("tag", toml_edit::Value::from(tag));
            crates_io_table.insert(crate_name, toml_edit::Item::Value(inline.into()));
        }
    }

    std::fs::write(cargo_toml_path, doc.to_string())?;
    Ok(())
}

/// Check if we should prompt the user to apply the wasm-bindgen patch.
/// Called during desktop builds to offer patching.
pub(crate) async fn check_wasm_bindgen_patch_prompt(workspace: &Workspace) -> Result<()> {
    let workspace_root = workspace.krates.workspace_root().as_std_path();

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
    tracing::info!("Your project may use wasm-bindgen crates (web-sys, etc).");
    tracing::info!("For desktop builds, these need a compatibility patch.");
    tracing::info!("");

    let term = console::Term::stdout();
    term.write_str("Apply wasm-bindgen patch to Cargo.toml? [Y/n] ")?;
    term.flush()?;

    let input = term.read_line()?;
    let should_patch = input.trim().is_empty() || input.trim().eq_ignore_ascii_case("y");

    // Mark as prompted so we don't ask again
    mark_prompted(workspace_root)?;

    if should_patch {
        let tag = get_matching_patch_tag(workspace).await?;
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
        let cargo_toml_path = workspace.krates.workspace_root().as_std_path().join("Cargo.toml");

        if !cargo_toml_path.exists() {
            return Err(anyhow::anyhow!(
                "No Cargo.toml found at {}",
                cargo_toml_path.display()
            ));
        }

        // Get the best matching tag for the workspace's wasm-bindgen version
        let tag = get_matching_patch_tag(&workspace).await?;
        tracing::info!(
            dx_src = ?TraceSrc::Dev,
            "Using wasm-bindgen-wry tag: {} (matching wasm-bindgen {})",
            tag,
            workspace.wasm_bindgen_version().unwrap_or_else(|| "unknown".to_string())
        );

        // Read the existing Cargo.toml
        let content = std::fs::read_to_string(&cargo_toml_path)?;
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

        let mut added = Vec::new();
        let mut skipped = Vec::new();

        for crate_name in PATCH_CRATES {
            if crates_io_table.contains_key(crate_name) && !self.force {
                skipped.push(*crate_name);
                continue;
            }

            // Create the inline table: { git = "...", tag = "..." }
            let mut inline = toml_edit::InlineTable::new();
            inline.insert("git", toml_edit::Value::from(PATCH_GIT_URL));
            inline.insert("tag", toml_edit::Value::from(tag.as_str()));
            crates_io_table.insert(crate_name, toml_edit::Item::Value(inline.into()));
            added.push(*crate_name);
        }

        // Write the modified Cargo.toml back
        std::fs::write(&cargo_toml_path, doc.to_string())?;

        // Log results
        if !added.is_empty() {
            tracing::info!(
                dx_src = ?TraceSrc::Dev,
                "Added wasm-bindgen patches: {}",
                added.join(", ")
            );
        }
        if !skipped.is_empty() {
            tracing::warn!(
                "Skipped existing patches (use --force to overwrite): {}",
                skipped.join(", ")
            );
        }

        if added.is_empty() && skipped.is_empty() {
            tracing::info!(dx_src = ?TraceSrc::Dev, "No patches needed.");
        }

        Ok(StructuredOutput::Success)
    }
}
