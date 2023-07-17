use std::path::PathBuf;

use crate::Result;

pub struct Bindgen {
    path: PathBuf,
    version: String,
}

pub fn get() -> Result<Bindgen> {
    // Check if exists

    // If exists return it

    // Otherwise try installing it

    // Then return it if success
}