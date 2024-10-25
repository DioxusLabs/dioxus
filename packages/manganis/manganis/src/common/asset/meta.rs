use std::{
    fmt::Display,
    hash::{DefaultHasher, Hash, Hasher},
    path::{Path, PathBuf},
};

use anyhow::Context;
use base64::Engine;
use serde::{Deserialize, Serialize};

use crate::{config, FileOptions};

/// A metadata asset
#[derive(Serialize, Deserialize, Debug, PartialEq, PartialOrd, Clone)]
pub struct MetadataAsset {
    key: String,
    value: String,
}

impl MetadataAsset {
    /// Creates a new metadata asset
    pub fn new(key: &str, value: &str) -> Self {
        Self {
            key: key.to_string(),
            value: value.to_string(),
        }
    }

    /// Returns the key of the metadata asset
    pub fn key(&self) -> &str {
        &self.key
    }

    /// Returns the value of the metadata asset
    pub fn value(&self) -> &str {
        &self.value
    }
}
