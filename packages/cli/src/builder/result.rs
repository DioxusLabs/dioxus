use crate::bundler::AppBundle;
use anyhow::Context;
use std::path::PathBuf;

use super::*;

pub struct BuildResult {
    /// Initial request that built this result
    pub request: BuildRequest,

    /// The output bundle
    pub bundle: AppBundle,

    /// The assets manifest
    pub assets: AssetManifest,

    /// The child process of this running app that has yet to be spawned.
    ///
    /// We might need to finangle this into something else
    pub child: Option<tokio::process::Child>,
}

impl BuildResult {
    pub async fn new(
        request: BuildRequest,
        assets: AssetManifest,
        bundle: AppBundle,
    ) -> anyhow::Result<Self> {
        let mut res = Self {
            request,
            assets,
            bundle,
            child: None,
        };

        Ok(res)
    }

    pub fn hotreload_asset(&mut self) {}
}
