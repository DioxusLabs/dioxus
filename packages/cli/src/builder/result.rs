use crate::{AssetManifest, BuildRequest};
use std::path::PathBuf;

/// The raw results of the build, including the exe, assets, timing information, warnings, errors, etc
///
/// This can be for either the server or the app.
///
/// You can combine BuildResults together into an AppBundle which knows how to bundle arbitrary results
/// together.
///
/// The build result has also copied assets over and written any helpful metadata and build stats to
/// the workdir.
///
/// The final AppBundle will include the build results in some combined format.
pub struct BuildResult {
    pub request: BuildRequest,
    pub assets: AssetManifest,
    pub output_location: PathBuf,
}

impl BuildResult {
    /// Take the raw outputs of the build and write the proper format to the workdir
    pub async fn new(assets: AssetManifest, output_location: PathBuf) {}
}
