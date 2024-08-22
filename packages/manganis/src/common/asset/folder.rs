use std::{
    fmt::Display,
    hash::{DefaultHasher, Hash, Hasher},
    path::{Path, PathBuf},
};

use anyhow::Context;
use base64::Engine;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::{config, AssetSource, FileOptions};

/// A folder asset
#[derive(Serialize, Deserialize, Debug, PartialEq, PartialOrd, Clone)]
pub struct FolderAsset {
    location: AssetSource,
}

impl Display for FolderAsset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/**", self.location.source(),)
    }
}

impl FolderAsset {
    /// Creates a new folder asset
    pub fn new(source: AssetSource) -> Self {
        let AssetSource::Local(source) = source else {
            panic!("Folder asset must be a local path");
        };
        assert!(source.canonicalized.is_dir());

        let mut myself = Self {
            location: AssetSource {
                unique_name: Default::default(),
                source: AssetSource::Local(source),
            },
        };

        myself.regenerate_unique_name();

        myself
    }

    /// Returns the location where the folder asset will be served from or None if the asset cannot be served
    pub fn served_location(&self) -> Result<String, ManganisSupportError> {
        resolve_asset_location(&self.location)
    }

    /// Returns the unique name of the folder asset
    pub fn unique_name(&self) -> &str {
        &self.location.unique_name
    }

    /// Returns the location of the folder asset
    pub fn location(&self) -> &AssetSource {
        &self.location
    }

    /// Create a unique hash for the source folder by recursively hashing the files
    fn hash(&self) -> u64 {
        let mut hash = std::collections::hash_map::DefaultHasher::new();
        let folder = self
            .location
            .source
            .as_path()
            .expect("Folder asset must be a local path");

        let mut folders_queued = vec![folder.clone()];

        while let Some(folder) = folders_queued.pop() {
            // Add the folder to the hash
            for segment in folder.iter() {
                segment.hash(&mut hash);
            }

            let files = std::fs::read_dir(folder).into_iter().flatten().flatten();
            for file in files {
                let path = file.path();
                let metadata = path.metadata().unwrap();

                // If the file is a folder, add it to the queue otherwise add it to the hash
                if metadata.is_dir() {
                    folders_queued.push(path);
                } else {
                    // todo: these relative/original paths are not correct
                    let local = self.location.source().local().unwrap();
                    hash_file(
                        &AssetSource::Local(LocalAssetSource {
                            original: local.original.clone(),
                            relative: local.relative.clone(),
                            canonicalized: path,
                        }),
                        &mut hash,
                    );
                }
            }
        }

        // Add the manganis version to the hash
        hash_version(&mut hash);

        hash.finish()
    }

    /// Regenerate the unique name of the folder asset
    fn regenerate_unique_name(&mut self) {
        let uuid = self.hash();
        let file_name = normalized_file_name(&self.location.source, None);
        self.location.unique_name = format!("{file_name}{uuid:x}");
        assert!(self.location.unique_name.len() <= MAX_PATH_LENGTH);
    }
}
