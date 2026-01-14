use super::*;
use crate::{TraceSrc, Workspace};
use std::path::Path;

/// Patch wasm-bindgen crates to use DioxusLabs fork for WRY compatibility.
#[derive(Clone, Debug, Parser)]
pub(crate) struct PatchWasmBindgen {
    /// Overwrite existing wasm-bindgen patches if they exist
    #[clap(long)]
    pub(crate) force: bool,
}

const PATCH_GIT_URL: &str = "https://github.com/DioxusLabs/wasm-bindgen-wry";

const PATCH_CRATES: &[&str] = &["wasm-bindgen", "wasm-bindgen-futures", "js-sys", "web-sys"];

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

/// Apply the wasm-bindgen patch to a Cargo.toml file (synchronous version)
pub(crate) fn apply_wasm_bindgen_patch(cargo_toml_path: &Path) -> Result<()> {
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
            crates_io_table.insert(crate_name, toml_edit::Item::Value(inline.into()));
        }
    }

    std::fs::write(cargo_toml_path, doc.to_string())?;
    Ok(())
}

/// Check if we should prompt the user to apply the wasm-bindgen patch.
/// Called during desktop builds to offer patching.
pub(crate) fn check_wasm_bindgen_patch_prompt(workspace_root: &Path) -> Result<()> {
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
        apply_wasm_bindgen_patch(&cargo_toml)?;
        term.write_line("✓ Patch applied to Cargo.toml")?;
    } else {
        term.write_line("Skipped. Run `dx patch-wasm-bindgen` later if needed.")?;
    }

    Ok(())
}

impl PatchWasmBindgen {
    pub(crate) async fn patch_wasm_bindgen(self) -> Result<StructuredOutput> {
        let crate_root = Workspace::crate_root_from_path()?;
        let cargo_toml_path = crate_root.join("Cargo.toml");

        if !cargo_toml_path.exists() {
            return Err(anyhow::anyhow!(
                "No Cargo.toml found at {}",
                cargo_toml_path.display()
            ));
        }

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

            // Create the inline table: { git = "..." }
            let mut inline = toml_edit::InlineTable::new();
            inline.insert("git", toml_edit::Value::from(PATCH_GIT_URL));
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
