use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub(crate) struct BundleConfig {
    pub(crate) identifier: Option<String>,
    pub(crate) publisher: Option<String>,
    pub(crate) icon: Option<Vec<String>>,
    pub(crate) resources: Option<Vec<String>>,
    pub(crate) copyright: Option<String>,
    pub(crate) category: Option<String>,
    pub(crate) short_description: Option<String>,
    pub(crate) long_description: Option<String>,
    pub(crate) external_bin: Option<Vec<String>>,
    pub(crate) deb: Option<DebianSettings>,
    pub(crate) macos: Option<MacOsSettings>,
    pub(crate) windows: Option<WindowsSettings>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub(crate) struct DebianSettings {
    // OS-specific settings:
    /// the list of debian dependencies.
    pub depends: Option<Vec<String>>,
    /// the list of dependencies the package provides.
    pub provides: Option<Vec<String>>,
    /// the list of package conflicts.
    pub conflicts: Option<Vec<String>>,
    /// the list of package replaces.
    pub replaces: Option<Vec<String>>,
    /// List of custom files to add to the deb package.
    /// Maps the path on the debian package to the path of the file to include (relative to the current working directory).
    pub files: HashMap<PathBuf, PathBuf>,
    /// Path to a custom desktop file Handlebars template.
    ///
    /// Available variables: `categories`, `comment` (optional), `exec`, `icon` and `name`.
    pub desktop_template: Option<PathBuf>,
    /// Define the section in Debian Control file. See : <https://www.debian.org/doc/debian-policy/ch-archive.html#s-subsections>
    pub section: Option<String>,
    /// Change the priority of the Debian Package. By default, it is set to `optional`.
    /// Recognized Priorities as of now are :  `required`, `important`, `standard`, `optional`, `extra`
    pub priority: Option<String>,
    /// Path of the uncompressed Changelog file, to be stored at /usr/share/doc/package-name/changelog.gz. See
    /// <https://www.debian.org/doc/debian-policy/ch-docs.html#changelog-files-and-release-notes>
    pub changelog: Option<PathBuf>,
    /// Path to script that will be executed before the package is unpacked. See
    /// <https://www.debian.org/doc/debian-policy/ch-maintainerscripts.html>
    pub pre_install_script: Option<PathBuf>,
    /// Path to script that will be executed after the package is unpacked. See
    /// <https://www.debian.org/doc/debian-policy/ch-maintainerscripts.html>
    pub post_install_script: Option<PathBuf>,
    /// Path to script that will be executed before the package is removed. See
    /// <https://www.debian.org/doc/debian-policy/ch-maintainerscripts.html>
    pub pre_remove_script: Option<PathBuf>,
    /// Path to script that will be executed after the package is removed. See
    /// <https://www.debian.org/doc/debian-policy/ch-maintainerscripts.html>
    pub post_remove_script: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub(crate) struct WixSettings {
    pub(crate) language: Vec<(String, Option<PathBuf>)>,
    pub(crate) template: Option<PathBuf>,
    pub(crate) fragment_paths: Vec<PathBuf>,
    pub(crate) component_group_refs: Vec<String>,
    pub(crate) component_refs: Vec<String>,
    pub(crate) feature_group_refs: Vec<String>,
    pub(crate) feature_refs: Vec<String>,
    pub(crate) merge_refs: Vec<String>,
    pub(crate) skip_webview_install: bool,
    pub(crate) license: Option<PathBuf>,
    pub(crate) enable_elevated_update_task: bool,
    pub(crate) banner_path: Option<PathBuf>,
    pub(crate) dialog_image_path: Option<PathBuf>,
    pub(crate) fips_compliant: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub(crate) struct MacOsSettings {
    pub(crate) frameworks: Option<Vec<String>>,
    pub(crate) minimum_system_version: Option<String>,
    pub(crate) license: Option<String>,
    pub(crate) exception_domain: Option<String>,
    pub(crate) signing_identity: Option<String>,
    pub(crate) provider_short_name: Option<String>,
    pub(crate) entitlements: Option<String>,
    pub(crate) info_plist_path: Option<PathBuf>,
    /// List of custom files to add to the application bundle.
    /// Maps the path in the Contents directory in the app to the path of the file to include (relative to the current working directory).
    pub files: HashMap<PathBuf, PathBuf>,
    /// Preserve the hardened runtime version flag, see <https://developer.apple.com/documentation/security/hardened_runtime>
    ///
    /// Settings this to `false` is useful when using an ad-hoc signature, making it less strict.
    pub hardened_runtime: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct WindowsSettings {
    pub(crate) digest_algorithm: Option<String>,
    pub(crate) certificate_thumbprint: Option<String>,
    pub(crate) timestamp_url: Option<String>,
    pub(crate) tsp: bool,
    pub(crate) wix: Option<WixSettings>,
    pub(crate) icon_path: Option<PathBuf>,
    pub(crate) webview_install_mode: WebviewInstallMode,
    pub(crate) webview_fixed_runtime_path: Option<PathBuf>,
    pub(crate) allow_downgrades: bool,
    pub(crate) nsis: Option<NsisSettings>,
    /// Specify a custom command to sign the binaries.
    /// This command needs to have a `%1` in it which is just a placeholder for the binary path,
    /// which we will detect and replace before calling the command.
    ///
    /// Example:
    /// ```text
    /// sign-cli --arg1 --arg2 %1
    /// ```
    ///
    /// By Default we use `signtool.exe` which can be found only on Windows so
    /// if you are on another platform and want to cross-compile and sign you will
    /// need to use another tool like `osslsigncode`.
    pub sign_command: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct NsisSettings {
    pub(crate) template: Option<PathBuf>,
    pub(crate) license: Option<PathBuf>,
    pub(crate) header_image: Option<PathBuf>,
    pub(crate) sidebar_image: Option<PathBuf>,
    pub(crate) installer_icon: Option<PathBuf>,
    pub(crate) install_mode: NSISInstallerMode,
    pub(crate) languages: Option<Vec<String>>,
    pub(crate) custom_language_files: Option<HashMap<String, PathBuf>>,
    pub(crate) display_language_selector: bool,
    pub(crate) start_menu_folder: Option<String>,
    pub(crate) installer_hooks: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) enum NSISInstallerMode {
    CurrentUser,
    PerMachine,
    Both,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) enum WebviewInstallMode {
    Skip,
    DownloadBootstrapper { silent: bool },
    EmbedBootstrapper { silent: bool },
    OfflineInstaller { silent: bool },
    FixedRuntime { path: PathBuf },
}

impl Default for WebviewInstallMode {
    fn default() -> Self {
        Self::OfflineInstaller { silent: false }
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum PackageType {
    MacOsBundle,
    IosBundle,
    WindowsMsi,
    Deb,
    Rpm,
    AppImage,
    Dmg,
    Updater,
}
