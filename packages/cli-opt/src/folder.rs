use super::file::process_file_to;
use crate::file::copy_file_to;
use manganis_core::FolderAssetOptions;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use std::path::Path;

/// Process a folder, optimizing and copying all assets into the output folder
pub fn process_folder(
    options: &FolderAssetOptions,
    source: &Path,
    output_folder: &Path,
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
            process_folder(options, &file, &output_path)
        } else {
            match options.optimize_files() {
                true => process_file_minimal(&file, &output_path),
                false => copy_file_to(&file, &output_path),
            }
        }
    })?;

    Ok(())
}

/// Optimize a file without changing any of its contents significantly (e.g. by changing the extension)
fn process_file_minimal(input_path: &Path, output_path: &Path) -> anyhow::Result<()> {
    process_file_to(
        &manganis_core::AssetOptions::Unknown,
        input_path,
        output_path,
    )?;
    Ok(())
}
