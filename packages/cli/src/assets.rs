use brotli::enc::BrotliEncoderParams;
use std::path::Path;
use std::{ffi::OsString, path::PathBuf};
use walkdir::WalkDir;

use std::{fs::File, io::Write};

use crate::Result;
use dioxus_cli_config::CrateConfig;
use dioxus_cli_config::Platform;
use manganis_cli_support::{AssetManifest, AssetManifestExt};

pub fn asset_manifest(bin: Option<&str>, crate_config: &CrateConfig) -> AssetManifest {
    AssetManifest::load_from_path(
        bin,
        crate_config.crate_dir.join("Cargo.toml"),
        crate_config.workspace_dir.join("Cargo.lock"),
    )
}

/// Create a head file that contains all of the imports for assets that the user project uses
pub fn create_assets_head(config: &CrateConfig, manifest: &AssetManifest) -> Result<()> {
    let mut file = File::create(config.out_dir().join("__assets_head.html"))?;
    file.write_all(manifest.head().as_bytes())?;
    Ok(())
}

/// Process any assets collected from the binary
pub(crate) fn process_assets(config: &CrateConfig, manifest: &AssetManifest) -> anyhow::Result<()> {
    let static_asset_output_dir = PathBuf::from(
        config
            .dioxus_config
            .web
            .app
            .base_path
            .clone()
            .unwrap_or_default(),
    );
    let static_asset_output_dir = config.out_dir().join(static_asset_output_dir);

    manifest.copy_static_assets_to(static_asset_output_dir)?;

    Ok(())
}

/// A guard that sets up the environment for the web renderer to compile in. This guard sets the location that assets will be served from
pub(crate) struct AssetConfigDropGuard;

impl AssetConfigDropGuard {
    pub fn new() -> Self {
        // Set up the collect asset config
        manganis_cli_support::Config::default()
            .with_assets_serve_location("/")
            .save();
        Self {}
    }
}

impl Drop for AssetConfigDropGuard {
    fn drop(&mut self) {
        // Reset the config
        manganis_cli_support::Config::default().save();
    }
}

pub fn copy_assets_dir(config: &CrateConfig, platform: Platform) -> anyhow::Result<()> {
    tracing::info!("Copying public assets to the output directory...");
    let out_dir = config.out_dir();
    let asset_dir = config.asset_dir();

    if asset_dir.is_dir() {
        // Only pre-compress the assets from the web build. Desktop assets are not served, so they don't need to be pre_compressed
        let pre_compress = platform == Platform::Web && config.dioxus_config.web.pre_compress;

        copy_dir_to(asset_dir, out_dir, pre_compress)?;
    }
    Ok(())
}

fn copy_dir_to(src_dir: PathBuf, dest_dir: PathBuf, pre_compress: bool) -> std::io::Result<()> {
    let entries = std::fs::read_dir(&src_dir)?;
    let mut children: Vec<std::thread::JoinHandle<std::io::Result<()>>> = Vec::new();

    for entry in entries.flatten() {
        let entry_path = entry.path();
        let path_relative_to_src = entry_path.strip_prefix(&src_dir).unwrap();
        let output_file_location = dest_dir.join(path_relative_to_src);
        children.push(std::thread::spawn(move || {
            if entry.file_type()?.is_dir() {
                // If the file is a directory, recursively copy it into the output directory
                if let Err(err) =
                    copy_dir_to(entry_path.clone(), output_file_location, pre_compress)
                {
                    tracing::error!(
                        "Failed to pre-compress directory {}: {}",
                        entry_path.display(),
                        err
                    );
                }
            } else {
                // Make sure the directory exists
                std::fs::create_dir_all(output_file_location.parent().unwrap())?;
                // Copy the file to the output directory
                std::fs::copy(&entry_path, &output_file_location)?;

                // Then pre-compress the file if needed
                if pre_compress {
                    if let Err(err) = pre_compress_file(&entry_path.clone()) {
                        tracing::error!(
                            "Failed to pre-compress static assets {}: {}",
                            entry_path.display(),
                            err
                        );
                    }
                }
            }

            Ok(())
        }));
    }
    for child in children {
        child.join().unwrap()?;
    }
    Ok(())
}

/// pre-compress a file with brotli
pub(crate) fn pre_compress_file(path: &Path) -> std::io::Result<()> {
    let new_extension = match path.extension() {
        Some(ext) => {
            if ext.to_string_lossy().to_lowercase().ends_with("br") {
                return Ok(());
            }
            let mut ext = ext.to_os_string();
            ext.push(".br");
            ext
        }
        None => OsString::from("br"),
    };
    let file = std::fs::File::open(&path)?;
    let mut stream = std::io::BufReader::new(file);
    let output = path.with_extension(new_extension);
    let mut buffer = std::fs::File::create(output)?;
    let params = BrotliEncoderParams::default();
    brotli::BrotliCompress(&mut stream, &mut buffer, &params)?;
    Ok(())
}

/// pre-compress all files in a folder
pub(crate) fn pre_compress_folder(path: &Path) -> std::io::Result<()> {
    let walk_dir = WalkDir::new(path);
    for entry in walk_dir.into_iter().filter_map(|e| e.ok()) {
        let entry_path = entry.path();
        if entry_path.is_file() {
            pre_compress_file(&entry_path)?;
        }
    }
    Ok(())
}
