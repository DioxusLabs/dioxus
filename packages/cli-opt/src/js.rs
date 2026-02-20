use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use anyhow::Context;
use manganis_core::JsAssetOptions;

use crate::hash::hash_file_contents;

/// The path to the esbuild binary, set by the CLI before asset processing begins.
static ESBUILD_BINARY: OnceLock<PathBuf> = OnceLock::new();

/// Set the esbuild binary path. Called once by the CLI at startup.
pub fn set_esbuild_binary_path(path: PathBuf) {
    let _ = ESBUILD_BINARY.set(path);
}

pub(crate) fn process_js(
    js_options: &JsAssetOptions,
    source: &Path,
    output_path: &Path,
    bundle: bool,
) -> anyhow::Result<()> {
    let minify = js_options.minified();
    let needs_esbuild = minify || bundle;

    if needs_esbuild {
        if let Some(esbuild) = ESBUILD_BINARY.get() {
            match run_esbuild(esbuild, source, output_path, bundle, minify) {
                Ok(()) => return Ok(()),
                Err(err) => {
                    tracing::error!(
                        "Failed to process JS with esbuild. Falling back to copy: {err}"
                    );
                }
            }
        } else {
            tracing::warn!("esbuild binary path not set. Copying JS without processing.");
        }
    }

    // Fallback: copy unprocessed
    let mut source_file = std::fs::File::open(source)?;
    let mut writer = std::io::BufWriter::new(std::fs::File::create(output_path)?);
    std::io::copy(&mut source_file, &mut writer).with_context(|| {
        format!(
            "Failed to write JS to output location: {}",
            output_path.display()
        )
    })?;

    Ok(())
}

/// Run esbuild to bundle and/or minify a JavaScript file.
fn run_esbuild(
    esbuild: &Path,
    source: &Path,
    output_path: &Path,
    bundle: bool,
    minify: bool,
) -> anyhow::Result<()> {
    let mut cmd = std::process::Command::new(esbuild);
    cmd.arg(source);
    cmd.arg(format!("--outfile={}", output_path.display()));
    cmd.arg("--log-level=warning");

    if bundle {
        cmd.arg("--bundle");
        cmd.arg("--format=esm");
    }

    if minify {
        cmd.arg("--minify");
    }

    tracing::debug!("Running esbuild: {:?}", cmd);

    let output = cmd.output().context("Failed to run esbuild")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("esbuild failed: {stderr}");
    }

    Ok(())
}

pub(crate) fn hash_js(
    _js_options: &JsAssetOptions,
    source: &Path,
    hasher: &mut impl std::hash::Hasher,
    _bundle: bool,
) -> anyhow::Result<()> {
    hash_file_contents(source, hasher)
}
