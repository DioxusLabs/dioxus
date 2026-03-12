//! Android package bundling.
//!
//! Produces Android artifacts by package type:
//! - `.apk` from the Gradle assemble output generated during the build pipeline
//! - `.aab` from `bundleRelease`

use crate::bundler::BundleContext;
use crate::PackageType;
use anyhow::{bail, Context, Result};
use std::path::PathBuf;

/// Bundle Android package artifacts for the requested package type.
///
/// Supported package types:
/// - `PackageType::Apk`
/// - `PackageType::Aab`
pub(crate) async fn bundle_project(
    ctx: &BundleContext<'_>,
    package_type: PackageType,
) -> Result<Vec<PathBuf>> {
    match package_type {
        PackageType::Apk => {
            let apk = ctx.build.android_apk_path();
            if !apk.exists() {
                bail!(
                    "APK output not found at {}. Ensure gradle assemble completed successfully.",
                    apk.display()
                );
            }
            Ok(vec![apk])
        }

        PackageType::Aab => {
            let aab = ctx
                .build
                .android_gradle_bundle()
                .await
                .context("Failed to run gradle bundleRelease")?;
            Ok(vec![aab])
        }
        _ => bail!("Unsupported Android package type: {package_type:?}"),
    }
}
