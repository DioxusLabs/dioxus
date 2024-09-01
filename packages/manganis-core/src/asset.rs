use serde::{Deserialize, Serialize};
use std::{
    hash::{Hash, Hasher},
    path::PathBuf,
    time::SystemTime,
};

/// The location we'll write to the link section - needs to be serializable
///
/// This basically is 1:1 with `manganis/Asset` but with more metadata to be useful to the macro and cli
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct ResourceAsset {
    /// The input path `/assets/blah.css`
    pub input: PathBuf,

    /// The canonicalized asset
    ///
    /// `Users/dioxus/dev/app/assets/blah.css`
    pub absolute: PathBuf,

    /// The post-bundle name of the asset - do we include the `assets` name?
    ///
    /// `blahcss123.css`
    pub bundled: String,
}

/// The maximum length of a path segment
const MAX_PATH_LENGTH: usize = 128;

/// The length of the hash in the output path
const HASH_SIZE: usize = 16;

#[derive(Debug)]
pub struct AssetError {}
impl ResourceAsset {
    pub fn parse_any(raw: &str) -> Result<Self, AssetError> {
        // get the location where the asset is absolute, relative to
        //
        // IE
        // /users/dioxus/dev/app/
        // is the root of
        // /users/dioxus/dev/app/assets/blah.css
        let mfst_dir = std::env::var("CARGO_MANIFEST_DIR")
            .map(PathBuf::from)
            .unwrap();

        // 1. the input file should be a pathbuf
        let input = PathBuf::from(raw);

        // 2.
        let absolute = mfst_dir
            .join(raw.trim_start_matches('/'))
            .canonicalize()
            .unwrap();

        let bundled = Self::make_unique_name(absolute.clone());

        Ok(Self {
            input,
            absolute,
            bundled,
        })
    }

    pub fn make_unique_name(file_path: PathBuf) -> String {
        // Create a hasher
        let mut hash = std::collections::hash_map::DefaultHasher::new();

        // Open the file to get its options
        let file = std::fs::File::open(&file_path).unwrap();
        let metadata = file.metadata().unwrap();
        let modified = metadata
            .modified()
            .unwrap_or_else(|_| SystemTime::UNIX_EPOCH);

        // Hash a bunch of metadata
        // name, options, modified time, and maybe the version of our crate
        // Hash the last time the file was updated and the file source. If either of these change, we need to regenerate the unique name
        modified.hash(&mut hash);
        file_path.hash(&mut hash);

        let uuid = hash.finish();
        let extension = file_path
            .extension()
            .map(|f| f.to_string_lossy())
            .map(|e| format!(".{e}"))
            .unwrap_or_default();
        let file_name = Self::normalize_file_name(file_path);

        let out = format!("{file_name}{uuid:x}{extension}");
        assert!(out.len() <= MAX_PATH_LENGTH);
        out
    }

    fn normalize_file_name(location: PathBuf) -> String {
        let file_name = location.file_name().unwrap();
        let last_segment = file_name.to_string_lossy();
        let extension = location.extension();
        let mut file_name = Self::to_alphanumeric_string_lossy(&last_segment);

        let extension_len = extension.map(|e| e.len() + 1).unwrap_or_default();
        let extension_and_hash_size = extension_len + HASH_SIZE;

        // If the file name is too long, we need to truncate it
        if file_name.len() + extension_and_hash_size > MAX_PATH_LENGTH {
            file_name = file_name[..MAX_PATH_LENGTH - extension_and_hash_size].to_string();
        }

        file_name
    }

    fn to_alphanumeric_string_lossy(name: &str) -> String {
        name.chars()
            .filter(|c| c.is_alphanumeric())
            .collect::<String>()
    }
}
