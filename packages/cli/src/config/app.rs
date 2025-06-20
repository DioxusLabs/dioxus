use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ApplicationConfig {
    #[serde(default)]
    pub(crate) out_dir: Option<PathBuf>,

    #[serde(default)]
    pub(crate) tailwind_input: Option<PathBuf>,

    #[serde(default)]
    pub(crate) tailwind_output: Option<PathBuf>,

    /// Use this file for the info.plist associated with the iOS app.
    /// `dx` will merge any required settings into this file required to build the app
    #[serde(default)]
    pub(crate) ios_info_plist: Option<PathBuf>,

    /// Use this file for the info.plist associated with the macOS app.
    /// `dx` will merge any required settings into this file required to build the app
    #[serde(default)]
    pub(crate) macos_info_plist: Option<PathBuf>,

    /// Use this file for the entitlements.plist associated with the iOS app.
    #[serde(default)]
    pub(crate) ios_entitlements: Option<PathBuf>,

    /// Use this file for the entitlements.plist associated with the macOS app.
    #[serde(default)]
    pub(crate) macos_entitlements: Option<PathBuf>,

    /// Use this file for the AndroidManifest.xml associated with the Android app.
    /// `dx` will merge any required settings into this file required to build the app
    #[serde(default)]
    pub(crate) android_manifest: Option<PathBuf>,
}
