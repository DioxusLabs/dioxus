use std::path::PathBuf;

use crate::{assets::AssetManifest, builder::Platform};

pub struct AppBundle {}

impl AppBundle {
    pub fn new(platform: Platform) -> Self {
        todo!()
    }

    pub fn set_main_executable(&mut self) {}

    /// Copy the assets out of the manifest and into the target location
    pub async fn copy_assets(&mut self, manifest: &AssetManifest) {
        todo!()
    }

    pub fn finish(self) -> Self {
        todo!()
    }

    pub fn open(&self) {}

    /// Get the path to the executable
    pub fn path(&self) -> PathBuf {
        todo!()
    }
}

/// The processed bundle infomrmation
#[derive(Clone)]
pub enum BundlePlatform {
    MacOS,
    Ios,
    Fullstack,
    Spa,
    Msi,
    Wix,
    Deb,
    Rpm,
    AppImage,
}
