use std::path::Path;

use anyhow::Context;
use manganis_core::JsAssetOptions;

use crate::opt::hash::hash_file_contents;
use crate::opt::js_module_detect::js_is_module;

pub(crate) fn process_js(
    js_options: &JsAssetOptions,
    source: &Path,
    output_path: &Path,
    esbuild_path: Option<&Path>,
) -> anyhow::Result<()> {
    if js_options.minified() {
        if let Some(esbuild) = esbuild_path {
            let is_module = js_is_module(js_options, source);
            match run_esbuild(esbuild, source, output_path, is_module) {
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

    // Fallback / no minification: copy unprocessed
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

/// Run esbuild to minify a JavaScript file in place.
///
/// When `is_module` is true, `--format=esm` is passed so the minifier preserves
/// module syntax (`import`/`export`); the consuming `<script>` tag is expected to
/// be `type="module"`. Otherwise no `--format` flag is set, which causes esbuild
/// to keep the input's format verbatim — a classic IIFE script stays a classic
/// script.
fn run_esbuild(
    esbuild: &Path,
    source: &Path,
    output_path: &Path,
    is_module: bool,
) -> anyhow::Result<()> {
    let mut cmd = std::process::Command::new(esbuild);
    cmd.arg(source);
    cmd.arg(format!("--outfile={}", output_path.display()));
    cmd.arg("--log-level=warning");
    cmd.arg("--minify");
    if is_module {
        cmd.arg("--format=esm");
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
) -> anyhow::Result<()> {
    hash_file_contents(source, hasher)
}
