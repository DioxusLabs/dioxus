use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::Context;
use manganis_core::{Asset, JsAssetOptions};
use swc::{config::JsMinifyOptions, try_with_handler, BoolOrDataConfig};
use swc_common::{sync::Lrc, FileName};
use swc_common::{SourceMap, GLOBALS};

pub(crate) fn minify_js(source: &Path) -> anyhow::Result<String> {
    let mut source_file = std::fs::File::open(source)?;
    let cm = Arc::new(SourceMap::default());

    let mut js = String::new();
    source_file.read_to_string(&mut js)?;
    let c = swc::Compiler::new(cm.clone());
    let output = GLOBALS
        .set(&Default::default(), || {
            try_with_handler(cm.clone(), Default::default(), |handler| {
                let filename = Lrc::new(FileName::Real(source.to_path_buf()));
                let fm = cm.new_source_file(filename, js.to_string());

                c.minify(
                    fm,
                    handler,
                    &JsMinifyOptions {
                        compress: BoolOrDataConfig::from_bool(true),
                        mangle: BoolOrDataConfig::from_bool(true),
                        ..Default::default()
                    },
                )
                .context("failed to minify javascript")
            })
        })
        .map(|output| output.code);

    match output {
        Ok(output) => Ok(output),
        Err(err) => {
            tracing::error!("Failed to minify javascript: {}", err);
            Ok(js)
        }
    }
}

pub(crate) fn process_js(
    js_options: &JsAssetOptions,
    source: &Path,
    output_path: &Path,
) -> anyhow::Result<()> {
    let js = if js_options.minified() {
        minify_js(source)?
    } else {
        let mut source_file = std::fs::File::open(&source)?;
        let mut source = String::new();
        source_file.read_to_string(&mut source)?;
        source
    };

    std::fs::write(output_path, js).with_context(|| {
        format!(
            "Failed to write js to output location: {}",
            output_path.display()
        )
    })?;

    Ok(())
}
