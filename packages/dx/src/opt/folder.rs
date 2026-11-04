use std::path::Path;

use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

use crate::opt::file::process_file_to;

/// Process a folder, optimizing and copying all assets into the output folder
pub fn process_folder(
    source: &Path,
    output_folder: &Path,
    esbuild_path: Option<&Path>,
) -> anyhow::Result<()> {
    // Create the folder
    std::fs::create_dir_all(output_folder)?;

    // Then optimize children
    let files: Vec<_> = std::fs::read_dir(source)
        .into_iter()
        .flatten()
        .flatten()
        .collect();

    files.par_iter().try_for_each(|file| {
        let file = file.path();
        let metadata = file.metadata()?;
        let output_path = output_folder.join(file.strip_prefix(source)?);
        if metadata.is_dir() {
            process_folder(&file, &output_path, esbuild_path)
        } else {
            process_file_minimal(&file, &output_path, esbuild_path)
        }
    })?;

    Ok(())
}

/// Optimize a file without changing any of its contents significantly (e.g. by changing the extension)
fn process_file_minimal(
    input_path: &Path,
    output_path: &Path,
    esbuild_path: Option<&Path>,
) -> anyhow::Result<()> {
    process_file_to(
        &manganis_core::AssetOptions::builder().into_asset_options(),
        input_path,
        output_path,
        esbuild_path,
    )?;
    Ok(())
}
