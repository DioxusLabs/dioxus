//! Methods for working with the filesystem that are faster than the std fs methods
//! Uses stuff like rayon, caching, and other optimizations
//!
//! Allows configuration in case you want to do some work while copying and allows you to track progress

use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};

use brotli::enc::BrotliEncoderParams;
use flate2::{Compression, write::GzEncoder};
use walkdir::WalkDir;

use crate::config::CompressionAlgorithm;

/// Get the path to the version of a file compressed with the given algorithm, or `None` if the file
/// is itself the output of pre-compression
fn compressed_path(path: &Path, algorithm: CompressionAlgorithm) -> Option<PathBuf> {
    let new_extension = match path.extension() {
        Some(ext) => {
            let lowercased = ext.to_string_lossy().to_lowercase();
            if CompressionAlgorithm::ALL
                .iter()
                .any(|algorithm| lowercased == algorithm.extension())
            {
                return None;
            }
            let mut ext = ext.to_os_string();
            ext.push(".");
            ext.push(algorithm.extension());
            ext
        }
        None => OsString::from(algorithm.extension()),
    };

    Some(path.with_extension(new_extension))
}

/// pre-compress a file with the given algorithm
pub(crate) fn pre_compress_file(
    path: &Path,
    algorithm: CompressionAlgorithm,
) -> std::io::Result<()> {
    let Some(compressed_path) = compressed_path(path, algorithm) else {
        return Ok(());
    };

    let file = std::fs::File::open(path)?;
    let mut stream = std::io::BufReader::new(file);
    let mut buffer = std::fs::File::create(compressed_path)?;
    match algorithm {
        CompressionAlgorithm::Brotli => {
            let params = BrotliEncoderParams::default();
            brotli::BrotliCompress(&mut stream, &mut buffer, &params)?;
        }
        CompressionAlgorithm::Gzip => {
            let mut encoder = GzEncoder::new(&mut buffer, Compression::best());
            std::io::copy(&mut stream, &mut encoder)?;
            encoder.finish()?;
        }
    }

    Ok(())
}

/// pre-compress all files in a folder with the given algorithm, removing any compressed files left
/// behind by the algorithms that are not in use
pub(crate) fn pre_compress_folder(
    path: &Path,
    pre_compress: Option<CompressionAlgorithm>,
) -> std::io::Result<()> {
    let walk_dir = WalkDir::new(path);
    for entry in walk_dir.into_iter().filter_map(|e| e.ok()) {
        let entry_path = entry.path();
        if !entry_path.is_file() {
            continue;
        }

        // Drop the compressed files of every algorithm we aren't emitting so a previous build
        // doesn't leave stale artifacts behind
        for stale in CompressionAlgorithm::ALL
            .iter()
            .filter(|algorithm| Some(**algorithm) != pre_compress)
        {
            if let Some(compressed_path) = compressed_path(entry_path, *stale) {
                _ = std::fs::remove_file(compressed_path);
            }
        }

        if let Some(algorithm) = pre_compress {
            tracing::info!("Pre-compressing file {}", entry_path.display());
            if let Err(err) = pre_compress_file(entry_path, algorithm) {
                tracing::error!("Failed to pre-compress file {entry_path:?}: {err}");
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;

    /// The contents we compress in these tests, long enough that both encoders emit more than a
    /// trivial header
    const CONTENTS: &[u8] = b"the quick brown fox jumps over the lazy dog, over and over again";

    fn write_file(dir: &Path, name: &str) -> PathBuf {
        let path = dir.join(name);
        std::fs::write(&path, CONTENTS).unwrap();
        path
    }

    #[test]
    fn compressed_path_uses_the_algorithm_extension() {
        let path = Path::new("assets/app.js");
        assert_eq!(
            compressed_path(path, CompressionAlgorithm::Brotli).unwrap(),
            Path::new("assets/app.js.br")
        );
        assert_eq!(
            compressed_path(path, CompressionAlgorithm::Gzip).unwrap(),
            Path::new("assets/app.js.gz")
        );
    }

    #[test]
    fn compressed_path_skips_already_compressed_files() {
        for algorithm in CompressionAlgorithm::ALL {
            assert_eq!(compressed_path(Path::new("app.js.br"), *algorithm), None);
            assert_eq!(compressed_path(Path::new("app.js.gz"), *algorithm), None);
        }
    }

    #[test]
    fn brotli_output_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_file(dir.path(), "app.js");
        pre_compress_file(&path, CompressionAlgorithm::Brotli).unwrap();

        let compressed = std::fs::File::open(dir.path().join("app.js.br")).unwrap();
        let mut decompressed = Vec::new();
        brotli::Decompressor::new(compressed, 4096)
            .read_to_end(&mut decompressed)
            .unwrap();
        assert_eq!(decompressed, CONTENTS);
    }

    #[test]
    fn gzip_output_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_file(dir.path(), "app.js");
        pre_compress_file(&path, CompressionAlgorithm::Gzip).unwrap();

        let compressed = std::fs::File::open(dir.path().join("app.js.gz")).unwrap();
        let mut decompressed = Vec::new();
        flate2::read::GzDecoder::new(compressed)
            .read_to_end(&mut decompressed)
            .unwrap();
        assert_eq!(decompressed, CONTENTS);
    }

    #[test]
    fn folder_emits_only_the_selected_algorithm() {
        let dir = tempfile::tempdir().unwrap();
        write_file(dir.path(), "app.js");

        pre_compress_folder(dir.path(), Some(CompressionAlgorithm::Gzip)).unwrap();
        assert!(dir.path().join("app.js.gz").exists());
        assert!(!dir.path().join("app.js.br").exists());

        // Switching algorithms drops the files the previous build left behind
        pre_compress_folder(dir.path(), Some(CompressionAlgorithm::Brotli)).unwrap();
        assert!(dir.path().join("app.js.br").exists());
        assert!(!dir.path().join("app.js.gz").exists());

        pre_compress_folder(dir.path(), None).unwrap();
        assert!(!dir.path().join("app.js.br").exists());
        assert!(!dir.path().join("app.js.gz").exists());
        assert!(dir.path().join("app.js").exists());
    }
}
