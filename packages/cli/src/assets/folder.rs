use std::path::Path;

use manganis_common::{FileOptions, FolderAsset};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

use crate::file::Process;

/// Process a folder, optimizing and copying all assets into the output folder
pub fn process_folder(folder: &FolderAsset, output_folder: &Path) -> anyhow::Result<()> {
    // Push the unique name of the folder to the output folder
    let output_folder = output_folder.join(folder.unique_name());

    if output_folder.exists() {
        return Ok(());
    }

    // .location()
    // // .source()
    // .as_path()
    let folder = folder.path();

    // Optimize and copy all assets in the folder in parallel
    process_folder_inner(folder, &output_folder)
}

fn process_folder_inner(folder: &Path, output_folder: &Path) -> anyhow::Result<()> {
    // Create the folder
    std::fs::create_dir_all(output_folder)?;

    // Then optimize children
    let files: Vec<_> = std::fs::read_dir(folder)
        .into_iter()
        .flatten()
        .flatten()
        .collect();

    files.par_iter().try_for_each(|file| {
        let file = file.path();
        let metadata = file.metadata()?;
        let output_path = output_folder.join(file.strip_prefix(folder)?);
        if metadata.is_dir() {
            process_folder_inner(&file, &output_path)
        } else {
            process_file_minimal(&file, &output_path)
        }
    })?;

    Ok(())
}

/// Optimize a file without changing any of its contents significantly (e.g. by changing the extension)
fn process_file_minimal(input_path: &Path, output_path: &Path) -> anyhow::Result<()> {
    todo!()
    // let options =
    //     FileOptions::default_for_extension(input_path.extension().and_then(|e| e.to_str()));
    // let source = input_path.to_path_buf();
    // options.process(&source, output_path)?;
    // Ok(())
}
