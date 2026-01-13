use super::*;
use crate::{TraceSrc, Workspace};

/// Patch wasm-bindgen crates to use DioxusLabs fork for WRY compatibility.
#[derive(Clone, Debug, Parser)]
pub(crate) struct PatchWasmBindgen {
    /// Overwrite existing wasm-bindgen patches if they exist
    #[clap(long)]
    pub(crate) force: bool,
}

const PATCH_GIT_URL: &str = "https://github.com/DioxusLabs/wasm-bindgen-wry";

const PATCH_CRATES: &[&str] = &["wasm-bindgen", "wasm-bindgen-futures", "js-sys", "web-sys"];

impl PatchWasmBindgen {
    pub(crate) async fn patch_wasm_bindgen(self) -> Result<StructuredOutput> {
        let crate_root = Workspace::crate_root_from_path()?;
        let cargo_toml_path = crate_root.join("Cargo.toml");

        if !cargo_toml_path.exists() {
            return Err(Error::Other(anyhow::anyhow!(
                "No Cargo.toml found at {}",
                cargo_toml_path.display()
            )));
        }

        // Read the existing Cargo.toml
        let content = std::fs::read_to_string(&cargo_toml_path)?;
        let mut doc = content
            .parse::<toml_edit::DocumentMut>()
            .map_err(|e| Error::Other(anyhow::anyhow!("Failed to parse Cargo.toml: {}", e)))?;

        // Get or create the [patch.crates-io] section
        let patch = doc.entry("patch").or_insert_with(|| {
            toml_edit::Item::Table(toml_edit::Table::new())
        });
        let patch_table = patch
            .as_table_mut()
            .ok_or_else(|| Error::Other(anyhow::anyhow!("[patch] is not a table")))?;

        let crates_io = patch_table
            .entry("crates-io")
            .or_insert_with(|| toml_edit::Item::Table(toml_edit::Table::new()));
        let crates_io_table = crates_io
            .as_table_mut()
            .ok_or_else(|| Error::Other(anyhow::anyhow!("[patch.crates-io] is not a table")))?;

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
