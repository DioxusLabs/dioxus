//! Android package bundling.
//!
//! Produces Android artifacts by package type:
//! - `.apk` from the Gradle assemble output generated during the build pipeline
//! - `.aab` from `bundleRelease`

use crate::bundler::BundleContext;
use crate::PackageType;
use anyhow::{bail, Context, Result};
use std::path::PathBuf;

impl BundleContext<'_> {
    /// Resolve or produce the final Android distributable for the requested package type.
    ///
    /// Android is different from the desktop bundlers in this module: most of the
    /// packaging work already happens during the build pipeline when the Gradle
    /// project, Android resources, manifests, and native libraries are assembled.
    /// By the time this method runs, bundling is mostly about surfacing the final
    /// artifact that should be handed back to the CLI.
    ///
    /// Supported package types:
    /// - [`PackageType::Apk`]: validate that the APK produced by the normal Android
    ///   assemble flow exists and return its path.
    /// - [`PackageType::Aab`]: invoke the dedicated Gradle bundle path through
    ///   `BuildRequest::android_gradle_bundle` and return the generated `.aab`.
    ///
    /// This method intentionally does not restage files or rewrite Android metadata.
    /// It is the bridge from the Android build pipeline to the CLI's common bundle
    /// reporting interface.
    pub(crate) async fn bundle_android(&self, package_type: PackageType) -> Result<Vec<PathBuf>> {
        match package_type {
            PackageType::Apk => {
                let apk = self.build.android_apk_path();
                if !apk.exists() {
                    bail!(
                        "APK output not found at {}. Ensure gradle assemble completed successfully.",
                        apk.display()
                    );
                }
                Ok(vec![apk])
            }

            PackageType::Aab => {
                let aab = self
                    .build
                    .android_gradle_bundle()
                    .await
                    .context("Failed to run gradle bundleRelease")?;
                Ok(vec![aab])
            }
            _ => bail!("Unsupported Android package type: {package_type:?}"),
        }
    }
}
