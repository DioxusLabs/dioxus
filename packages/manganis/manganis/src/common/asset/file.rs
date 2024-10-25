use std::{
    fmt::Display,
    hash::{DefaultHasher, Hash, Hasher},
    path::{Path, PathBuf},
};

use anyhow::Context;
use base64::Engine;
use serde::{Deserialize, Serialize};

use crate::{config, FileOptions, ResourceAsset as AssetSource};

/// A file asset
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct FileAsset {
    location: AssetSource,
    options: FileOptions,
    url_encoded: bool,
}

/// A folder asset
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct FolderAsset {
    location: AssetSource,
}

impl FolderAsset {
    ///
    pub fn path(&self) -> &Path {
        todo!()
    }
}

impl std::ops::Deref for FolderAsset {
    type Target = AssetSource;

    fn deref(&self) -> &Self::Target {
        &self.location
    }
}

impl std::ops::Deref for FileAsset {
    type Target = AssetSource;

    fn deref(&self) -> &Self::Target {
        &self.location
    }
}

impl FileAsset {
    /// Creates a new file asset
    pub fn new(source: AssetSource) -> Self {
        todo!()
        // if let Some(path) = source.as_path() {
        //     assert!(!path.is_dir());
        // }

        // let options = FileOptions::default_for_extension(source.extension().as_deref());

        // let mut myself = Self {
        //     location: AssetSource {
        //         unique_name: Default::default(),
        //         source,
        //     },
        //     options,
        //     url_encoded: false,
        // };

        // myself.regenerate_unique_name();

        // myself
    }

    /// Set the file options
    pub fn with_options(self, options: FileOptions) -> Self {
        let mut myself = Self {
            location: self.location,
            options,
            url_encoded: false,
        };

        myself.regenerate_unique_name();

        myself
    }

    /// Set whether the file asset should be url encoded
    pub fn set_url_encoded(&mut self, url_encoded: bool) {
        self.url_encoded = url_encoded;
    }

    /// Returns whether the file asset should be url encoded
    pub fn url_encoded(&self) -> bool {
        self.url_encoded
    }

    // /// Returns the location where the file asset will be served from or None if the asset cannot be served
    // pub fn served_location(&self) -> Result<String, ManganisSupportError> {
    //     if self.url_encoded {
    //         let data = self.location.source.read_to_bytes().unwrap();
    //         let data = base64::engine::general_purpose::STANDARD_NO_PAD.encode(data);
    //         let mime = self.location.source.mime_type().unwrap();
    //         Ok(format!("data:{mime};base64,{data}"))
    //     } else {
    //         resolve_asset_location(&self.location)
    //     }
    // }

    /// Returns the location of the file asset
    pub fn location(&self) -> &AssetSource {
        &self.location
    }

    /// Returns the options for the file asset
    pub fn options(&self) -> &FileOptions {
        &self.options
    }

    ///
    pub fn path(&self) -> &Path {
        todo!()
    }

    /// Returns the options for the file asset mutably
    pub fn with_options_mut(&mut self, f: impl FnOnce(&mut FileOptions)) {
        f(&mut self.options);
        self.regenerate_unique_name();
    }

    /// Hash the file asset source and options
    fn hash(&self) -> u64 {
        todo!()
        // let mut hash = std::collections::hash_map::DefaultHasher::new();
        // hash_file(&self.location.source, &mut hash);
        // self.options.hash(&mut hash);
        // hash_version(&mut hash);
        // hash.finish()
    }

    /// Regenerates the unique name of the file asset
    fn regenerate_unique_name(&mut self) {
        todo!()
        // // Generate an unique name for the file based on the options, source, and the current version of manganis
        // let uuid = self.hash();
        // let extension = self.options.extension();
        // let file_name = normalized_file_name(&self.location.source, extension);
        // let extension = extension.map(|e| format!(".{e}")).unwrap_or_default();
        // self.location.unique_name = format!("{file_name}{uuid:x}{extension}");
        // assert!(self.location.unique_name.len() <= MAX_PATH_LENGTH);
    }
}

// impl Display for FileAsset {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         let url_encoded = if self.url_encoded {
//             " [url encoded]"
//         } else {
//             ""
//         };

//         write!(
//             f,
//             "{} [{}]{}",
//             self.location.source(),
//             self.options,
//             url_encoded
//         )
//     }
// }
