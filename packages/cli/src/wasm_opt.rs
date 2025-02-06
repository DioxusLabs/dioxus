use crate::Result;
use std::path::Path;

#[derive(Default, Debug, Clone, Copy)]
pub struct WasmOptOptions {
    /// Keep debug symbols in the wasm file
    pub debug_symbols: bool,
}

/// Write these wasm bytes with a particular set of optimizations
pub async fn write_wasm(bytes: &[u8], output_path: &Path, options: WasmOptOptions) -> Result<()> {
    tokio::fs::write(output_path, bytes).await?;
    optimize(output_path, output_path, options).await?;
    Ok(())
}

pub async fn optimize(
    input_path: &Path,
    output_path: &Path,
    options: WasmOptOptions,
) -> Result<()> {
    let mut args = vec![
        // needed by wasm-bindgen
        "--enable-reference-types",
        // needed for our current approach to bundle splitting to work properly
        // todo(jon): emit the main module's data section in chunks instead of all at once
        "--memory-packing",
    ];

    if !options.debug_symbols {
        args.push("--strip-debug");
    }

    tokio::process::Command::new("wasm-opt")
        .arg(input_path)
        .arg("-Oz")
        .arg("-o")
        .arg(output_path)
        .args(args)
        .output()
        .await?;

    Ok(())
}
