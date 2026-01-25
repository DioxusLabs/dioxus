use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Default, Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub(crate) struct ApplicationConfig {
    /// The path where global assets will be added when components are added with `dx components add`
    #[serde(default)]
    pub(crate) asset_dir: Option<PathBuf>,

    #[serde(default)]
    pub(crate) out_dir: Option<PathBuf>,

    #[serde(default = "public_dir_default")]
    #[serde(deserialize_with = "empty_string_is_none")]
    pub(crate) public_dir: Option<PathBuf>,

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

    /// Use this file for the MainActivity.kt associated with the Android app.
    #[serde(default)]
    pub(crate) android_main_activity: Option<PathBuf>,

    /// Specified minimum sdk version for gradle to build the app with.
    #[serde(default)]
    pub(crate) android_min_sdk_version: Option<u32>,
}

fn public_dir_default() -> Option<PathBuf> {
    Some("public".into())
}

fn empty_string_is_none<'de, D>(deserializer: D) -> Result<Option<PathBuf>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let opt: Option<String> = Option::deserialize(deserializer)?;
    match opt {
        Some(s) if s.is_empty() => Ok(None),
        Some(s) => Ok(Some(PathBuf::from(s))),
        None => Ok(None),
    }
}
