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

impl ResourceAsset {
    pub fn parse_any(raw: &str) -> Result<Self, AssetParseError> {
        // get the location where the asset is absolute, relative to
        //
        // IE
        // /users/dioxus/dev/app/
        // is the root of
        // /users/dioxus/dev/app/assets/blah.css
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
            .map(PathBuf::from)
            .unwrap();

        // 1. the input file should be a pathbuf
        let input = PathBuf::from(raw);

        // 2. absolute path to the asset
        let absolute = manifest_dir.join(raw.trim_start_matches('/'));
        let absolute =
            absolute
                .canonicalize()
                .map_err(|err| AssetParseError::AssetDoesntExist {
                    err,
                    path: absolute,
                })?;

        // 3. the bundled path is the unique name of the asset
        let bundled = Self::make_unique_name(absolute.clone())?;

        Ok(Self {
            input,
            absolute,
            bundled,
        })
    }

    fn make_unique_name(file_path: PathBuf) -> Result<String, AssetParseError> {
        // Create a hasher
        let mut hash = std::collections::hash_map::DefaultHasher::new();

        // Open the file to get its options
        let file = std::fs::File::open(&file_path).map_err(AssetParseError::FailedToReadAsset)?;
        let modified = file
            .metadata()
            .and_then(|metadata| metadata.modified())
            .unwrap_or(SystemTime::UNIX_EPOCH);

        // Hash a bunch of metadata
        // name, options, modified time, and maybe the version of our crate
        // Hash the last time the file was updated and the file source. If either of these change, we need to regenerate the unique name
        modified.hash(&mut hash);
        file_path.hash(&mut hash);

        let uuid = hash.finish();
        let file_name = file_path
            .file_stem()
            .expect("file_path should have a file_stem")
            .to_string_lossy();

        let extension = file_path
            .extension()
            .map(|f| f.to_string_lossy())
            .unwrap_or_default();

        Ok(format!("{file_name}-{uuid:x}.{extension}"))
    }
}

#[derive(Debug)]
pub enum AssetParseError {
    ParseError(String),
    AssetDoesntExist {
        err: std::io::Error,
        path: std::path::PathBuf,
    },
    FailedToReadAsset(std::io::Error),
    FailedToReadMetadata(std::io::Error),
}

impl std::fmt::Display for AssetParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AssetParseError::ParseError(err) => write!(f, "Failed to parse asset: {}", err),
            AssetParseError::AssetDoesntExist { err, path } => {
                write!(f, "Asset at {} doesn't exist: {}", path.display(), err)
            }
            AssetParseError::FailedToReadAsset(err) => write!(f, "Failed to read asset: {}", err),
            AssetParseError::FailedToReadMetadata(err) => {
                write!(f, "Failed to read asset metadata: {}", err)
            }
        }
    }
}
