use crate::config::WasmOptLevel;
use crate::{Result, WasmOptConfig};
use std::path::Path;

/// Write these wasm bytes with a particular set of optimizations
pub async fn write_wasm(bytes: &[u8], output_path: &Path, cfg: &WasmOptConfig) -> Result<()> {
    std::fs::write(output_path, bytes)?;
    optimize(output_path, output_path, cfg).await?;
    Ok(())
}

#[allow(unreachable_code)]
pub async fn optimize(input_path: &Path, output_path: &Path, cfg: &WasmOptConfig) -> Result<()> {
    #[cfg(feature = "optimizations")]
    return run_from_lib(input_path, output_path, cfg).await;

    // It's okay not to run wasm-opt but we should *really* try it
    if which::which("wasm-opt").is_err() {
        tracing::warn!("wasm-opt not found and CLI is compiled without optimizations. Skipping optimization for {}", input_path.display());
        return Ok(());
    }

    run_locally(input_path, output_path, cfg).await?;

    Ok(())
}

async fn run_locally(input_path: &Path, output_path: &Path, cfg: &WasmOptConfig) -> Result<()> {
    // defaults needed by wasm-bindgen.
    // wasm is a moving target, and we add these by default since they progressively get enabled by default.
    let mut args = vec![
        "--enable-reference-types",
        "--enable-bulk-memory",
        "--enable-mutable-globals",
        "--enable-nontrapping-float-to-int",
    ];

    if cfg.memory_packing {
        // needed for our current approach to bundle splitting to work properly
        // todo(jon): emit the main module's data section in chunks instead of all at once
        args.push("--memory-packing");
    }

    if !cfg.debug {
        args.push("--strip-debug");
    } else {
        args.push("--debuginfo");
    }

    for extra in &cfg.extra_features {
        args.push(extra);
    }

    let level = match cfg.level {
        WasmOptLevel::Z => "-Oz",
        WasmOptLevel::S => "-Os",
        WasmOptLevel::Zero => "-O0",
        WasmOptLevel::One => "-O1",
        WasmOptLevel::Two => "-O2",
        WasmOptLevel::Three => "-O3",
        WasmOptLevel::Four => "-O4",
    };

    let res = tokio::process::Command::new("wasm-opt")
        .arg(input_path)
        .arg(level)
        .arg("-o")
        .arg(output_path)
        .args(args)
        .output()
        .await?;

    if !res.status.success() {
        let err = String::from_utf8_lossy(&res.stderr);
        tracing::error!("wasm-opt failed with status code {}: {}", res.status, err);
    }

    Ok(())
}

/// Use the `wasm_opt` crate
#[cfg(feature = "optimizations")]
async fn run_from_lib(
    input_path: &Path,
    output_path: &Path,
    options: &WasmOptConfig,
) -> Result<()> {
    use std::str::FromStr;

    let mut level = match options.level {
        WasmOptLevel::Z => wasm_opt::OptimizationOptions::new_optimize_for_size_aggressively(),
        WasmOptLevel::S => wasm_opt::OptimizationOptions::new_optimize_for_size(),
        WasmOptLevel::Zero => wasm_opt::OptimizationOptions::new_opt_level_0(),
        WasmOptLevel::One => wasm_opt::OptimizationOptions::new_opt_level_1(),
        WasmOptLevel::Two => wasm_opt::OptimizationOptions::new_opt_level_2(),
        WasmOptLevel::Three => wasm_opt::OptimizationOptions::new_opt_level_3(),
        WasmOptLevel::Four => wasm_opt::OptimizationOptions::new_opt_level_4(),
    };

    level
        .enable_feature(wasm_opt::Feature::ReferenceTypes)
        .enable_feature(wasm_opt::Feature::BulkMemory)
        .enable_feature(wasm_opt::Feature::MutableGlobals)
        .enable_feature(wasm_opt::Feature::TruncSat)
        .add_pass(wasm_opt::Pass::MemoryPacking)
        .debug_info(options.debug);

    for arg in options.extra_features.iter() {
        if arg.starts_with("--enable-") {
            let feature = arg.trim_start_matches("--enable-");
            if let Ok(feature) = wasm_opt::Feature::from_str(feature) {
                level.enable_feature(feature);
            } else {
                tracing::warn!("Unknown wasm-opt feature: {}", feature);
            }
        } else if arg.starts_with("--disable-") {
            let feature = arg.trim_start_matches("--disable-");
            if let Ok(feature) = wasm_opt::Feature::from_str(feature) {
                level.disable_feature(feature);
            } else {
                tracing::warn!("Unknown wasm-opt feature: {}", feature);
            }
        }
    }

    level
        .run(input_path, output_path)
        .map_err(|err| crate::Error::Other(anyhow::anyhow!(err)))?;

    Ok(())
}
