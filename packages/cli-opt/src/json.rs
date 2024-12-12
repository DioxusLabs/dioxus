use std::{io::Read, path::Path};

use anyhow::Context;

pub(crate) fn minify_json(source: &str) -> anyhow::Result<String> {
    // First try to parse the json
    let json: serde_json::Value = serde_json::from_str(source)?;
    // Then print it in a minified format
    let json = serde_json::to_string(&json)?;
    Ok(json)
}

pub(crate) fn update_asset_references(json: &str) -> String {
    // Placeholder implementation for updating asset references in JSON files
    // This function should identify and update asset references to the new generated names
    json.to_string()
}

pub(crate) fn process_json(source: &Path, output_path: &Path) -> anyhow::Result<()> {
    let mut source_file = std::fs::File::open(source)?;
    let mut source = String::new();
    source_file.read_to_string(&mut source)?;
    let json = match minify_json(&source) {
        Ok(json) => json,
        Err(err) => {
            tracing::error!("Failed to minify json: {}", err);
            source
        }
    };

    let updated_json = update_asset_references(&json);

    std::fs::write(output_path, updated_json).with_context(|| {
        format!(
            "Failed to write json to output location: {}",
            output_path.display()
        )
    })?;

    Ok(())
}
