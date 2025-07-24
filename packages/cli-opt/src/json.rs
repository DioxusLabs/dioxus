use std::{io::Read, path::Path};

use anyhow::{bail, Context};

pub(crate) fn minify_json(source: &str) -> anyhow::Result<String> {
    // First try to parse the json
    let json: serde_json::Value = serde_json::from_str(source)?;
    // Then print it in a minified format
    let json = serde_json::to_string(&json)?;
    Ok(json)
}

pub(crate) fn process_json(
    source_path: &Path,
    output_path: &Path,
    allow_fallback: bool,
) -> anyhow::Result<()> {
    let mut source_file = std::fs::File::open(source_path)?;
    let mut source = String::new();
    source_file.read_to_string(&mut source)?;
    let json = match minify_json(&source) {
        Ok(json) => json,
        Err(err) => {
            if !allow_fallback {
                bail!(
                    "Failed to minify json from {}: {}",
                    source_path.display(),
                    err
                );
            }

            tracing::error!("Failed to minify json: {}", err);

            source
        }
    };

    std::fs::write(output_path, json).with_context(|| {
        format!(
            "Failed to write json to output location: {}",
            output_path.display()
        )
    })?;

    Ok(())
}
