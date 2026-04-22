//! iOS/macOS Swift package manifest helpers and compilation.
//!
//! ### Macos
//!
//! We simply use the macos format where binaries are in `Contents/MacOS` and assets are in `Contents/Resources`
//! We put assets in an assets dir such that it generally matches every other platform and we can
//! output `/assets/blah` from manganis.
//! ```
//! App.app/
//!     Contents/
//!         Info.plist
//!         MacOS/
//!             Frameworks/
//!         Resources/
//!             assets/
//!                 blah.icns
//!                 blah.png
//!         CodeResources
//!         _CodeSignature/
//! ```
//!
//! ### iOS
//!
//! Not the same as mac! ios apps are a bit "flattened" in comparison. simpler format, presumably
//! since most ios apps don't ship frameworks/plugins and such.
//!
//! todo(jon): include the signing and entitlements in this format diagram.
//! ```
//! App.app/
//!     main
//!     assets/
//! ```

use crate::{BuildContext, BundleFormat, Result};
use crate::{BuildRequest, ManifestMapper};
use anyhow::{bail, Context};
use manganis::SwiftPackageMetadata;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};
use target_lexicon::{OperatingSystem, Triple};
use tokio::process::Command;

impl BuildRequest {
    /// Currently does nothing, but eventually we need to check that the mobile tooling is installed.
    ///
    /// For ios, this would be just aarch64-apple-ios + aarch64-apple-ios-sim, as well as xcrun and xcode-select
    ///
    /// We don't auto-install these yet since we're not doing an architecture check. We assume most users
    /// are running on an Apple Silicon Mac, but it would be confusing if we installed these when we actually
    /// should be installing the x86 versions.
    pub async fn verify_ios_tooling(&self) -> Result<()> {
        // open the simulator
        // _ = tokio::process::Command::new("open")
        //     .arg("/Applications/Xcode.app/Contents/Developer/Applications/Simulator.app")
        //     .output()
        //     .await;

        // Now xcrun to open the device
        // todo: we should try and query the device list and/or parse it rather than hardcode this simulator
        // _ = tokio::process::Command::new("xcrun")
        //     .args(["simctl", "boot", "83AE3067-987F-4F85-AE3D-7079EF48C967"])
        //     .output()
        //     .await;

        // if !rustup
        //     .installed_toolchains
        //     .contains(&"aarch64-apple-ios".to_string())
        // {
        //     tracing::error!("You need to install aarch64-apple-ios to build for ios. Run `rustup target add aarch64-apple-ios` to install it.");
        // }

        // if !rustup
        //     .installed_toolchains
        //     .contains(&"aarch64-apple-ios-sim".to_string())
        // {
        //     tracing::error!("You need to install aarch64-apple-ios to build for ios. Run `rustup target add aarch64-apple-ios` to install it.");
        // }

        Ok(())
    }

    pub async fn start_ios_sim(&self) -> Result<()> {
        #[derive(Deserialize, Debug)]
        struct XcrunListJson {
            // "com.apple.CoreSimulator.SimRuntime.iOS-18-4": [{}, {}, {}]
            devices: BTreeMap<String, Vec<XcrunDevice>>,
        }
        #[derive(Deserialize, Debug)]
        struct XcrunDevice {
            #[serde(rename = "lastBootedAt")]
            last_booted_at: Option<String>,
            udid: String,
            name: String,
            state: String,
        }
        let xcrun_list = Command::new("xcrun")
            .arg("simctl")
            .arg("list")
            .arg("-j")
            .output()
            .await?;
        let as_str = String::from_utf8_lossy(&xcrun_list.stdout);
        let xcrun_list_json = serde_json::from_str::<XcrunListJson>(as_str.trim());
        if let Ok(xcrun_list_json) = xcrun_list_json {
            if xcrun_list_json.devices.is_empty() {
                tracing::warn!("No iOS sdks installed found. Please install the iOS SDK in Xcode.");
            }

            if let Some((_rt, devices)) = xcrun_list_json.devices.iter().next() {
                if devices.iter().all(|device| device.state != "Booted") {
                    let last_booted =
                        devices
                            .iter()
                            .max_by_key(|device| match device.last_booted_at {
                                Some(ref last_booted) => last_booted,
                                None => "2000-01-01T01:01:01Z",
                            });

                    if let Some(device) = last_booted {
                        tracing::info!("Booting iOS simulator: \"{}\"", device.name);
                        Command::new("xcrun")
                            .arg("simctl")
                            .arg("boot")
                            .arg(&device.udid)
                            .output()
                            .await?;
                    }
                }
            }
        }
        let path_to_xcode = Command::new("xcode-select")
            .arg("--print-path")
            .output()
            .await?;
        let path_to_xcode: PathBuf = String::from_utf8_lossy(&path_to_xcode.stdout)
            .as_ref()
            .trim()
            .into();
        let path_to_sim = path_to_xcode.join("Applications").join("Simulator.app");
        open::that_detached(path_to_sim)?;
        Ok(())
    }

    pub fn info_plist_contents(&self, bundle: BundleFormat) -> Result<String> {
        /// A permission entry for plist (key + description)
        #[derive(Serialize)]
        struct PlistPermission {
            key: String,
            description: String,
        }

        #[derive(Serialize)]
        pub struct InfoPlistData {
            pub display_name: String,
            pub bundle_name: String,
            pub bundle_identifier: String,
            pub executable_name: String,
            /// App version string (from Cargo.toml)
            pub version: String,
            /// Permission usage descriptions
            pub permissions: Vec<PlistPermission>,
            /// Additional plist entries as raw XML
            pub plist_entries: String,
            /// Raw plist XML to inject
            pub raw_plist: String,
            /// Minimum system version (macOS only)
            pub minimum_system_version: String,
            /// URL schemes for deep linking
            pub url_schemes: Vec<String>,
            /// iOS UIBackgroundModes
            pub background_modes: Vec<String>,
        }

        // Attempt to use the user's manually specified
        let _app = &self.config.application;
        match bundle {
            BundleFormat::MacOS => {
                if let Some(macos_info_plist) = _app.macos_info_plist.as_deref() {
                    return Ok(std::fs::read_to_string(macos_info_plist)?);
                }
            }
            BundleFormat::Ios => {
                if let Some(macos_info_plist) = _app.ios_info_plist.as_deref() {
                    return Ok(std::fs::read_to_string(macos_info_plist)?);
                }
            }
            _ => {}
        }

        // Get permission mapper from config
        let mapper = ManifestMapper::from_config(
            &self.config.permissions,
            &self.config.deep_links,
            &self.config.background,
            &self.config.android,
            &self.config.ios,
            &self.config.macos,
        );

        match bundle {
            BundleFormat::MacOS => {
                // Convert macOS plist entries to permission structs
                let permissions: Vec<PlistPermission> = mapper
                    .macos_plist_entries
                    .iter()
                    .map(|p| PlistPermission {
                        key: p.key.clone(),
                        description: p.value.clone(),
                    })
                    .collect();

                // Generate plist entries from config
                let plist_entries = generate_plist_entries(&self.config.macos.plist);
                let raw_plist = self.config.macos.raw.info_plist.clone().unwrap_or_default();
                let minimum_system_version = self
                    .config
                    .macos
                    .minimum_system_version
                    .clone()
                    .unwrap_or_else(|| "10.15".to_string());

                handlebars::Handlebars::new()
                    .render_template(
                        include_str!("../../assets/macos/mac.plist.hbs"),
                        &InfoPlistData {
                            display_name: self.bundled_app_name(),
                            bundle_name: self.bundled_app_name(),
                            executable_name: self.platform_exe_name(),
                            bundle_identifier: self.bundle_identifier(),
                            version: self.crate_version(),
                            permissions,
                            plist_entries,
                            raw_plist,
                            minimum_system_version,
                            url_schemes: mapper.macos_url_schemes.clone(),
                            background_modes: Vec::new(), // macOS doesn't use UIBackgroundModes
                        },
                    )
                    .map_err(|e| e.into())
            }
            BundleFormat::Ios => {
                // Convert iOS plist entries to permission structs
                let permissions: Vec<PlistPermission> = mapper
                    .ios_plist_entries
                    .iter()
                    .map(|p| PlistPermission {
                        key: p.key.clone(),
                        description: p.value.clone(),
                    })
                    .collect();

                // Generate plist entries from config
                let plist_entries = generate_plist_entries(&self.config.ios.plist);
                let raw_plist = self.config.ios.raw.info_plist.clone().unwrap_or_default();

                handlebars::Handlebars::new()
                    .render_template(
                        include_str!("../../assets/ios/ios.plist.hbs"),
                        &InfoPlistData {
                            display_name: self.bundled_app_name(),
                            bundle_name: self.bundled_app_name(),
                            executable_name: self.platform_exe_name(),
                            bundle_identifier: self.bundle_identifier(),
                            version: self.crate_version(),
                            permissions,
                            plist_entries,
                            raw_plist,
                            minimum_system_version: String::new(), // Not used for iOS
                            url_schemes: mapper.ios_url_schemes.clone(),
                            background_modes: mapper.ios_background_modes.clone(),
                        },
                    )
                    .map_err(|e| e.into())
            }
            _ => Err(anyhow::anyhow!("Unsupported platform for Info.plist")),
        }
    }

    pub async fn codesign_apple(&self, ctx: &BuildContext) -> Result<()> {
        ctx.status_codesigning();

        // We don't want to drop the entitlements file, until the end of the block, so we hoist it to this temporary.
        let mut _saved_entitlements = None;

        let mut app_dev_name = self.apple_team_id.clone();
        if app_dev_name.is_none() {
            app_dev_name = Some(Self::auto_provision_signing_name().await.context(
                "Failed to automatically provision signing name for Apple codesigning.",
            )?);
        }

        let mut entitlements_file = self.apple_entitlements.clone();
        let mut provisioning_profile_path = None;
        if entitlements_file.is_none() {
            let bundle_id = self.bundle_identifier();
            let (entitlements_xml, profile_path) = Self::auto_provision_entitlements(&bundle_id)
                .await
                .context("Failed to auto-provision entitlements for Apple codesigning.")?;

            // Enrich with entitlements from Dioxus.toml config
            let entitlements_xml = self.enrich_entitlements_from_config(entitlements_xml)?;

            let entitlements_temp_file = tempfile::NamedTempFile::new()?;
            std::fs::write(entitlements_temp_file.path(), entitlements_xml)?;
            entitlements_file = Some(entitlements_temp_file.path().to_path_buf());
            provisioning_profile_path = Some(profile_path);
            _saved_entitlements = Some(entitlements_temp_file);
        }

        let entitlements_file = entitlements_file.as_ref().context(
            "No entitlements file provided and could not provision entitlements to sign app.",
        )?;
        let app_dev_name = app_dev_name.as_ref().context(
            "No Apple Development signing name provided and could not auto-provision one.",
        )?;

        tracing::debug!(
            "Codesigning Apple app with entitlements: {} and dev name: {}",
            entitlements_file.display(),
            app_dev_name
        );

        // determine the target exe - the server and macos bundles are different
        let target_exe = match self.bundle {
            BundleFormat::MacOS => self.root_dir(),
            BundleFormat::Ios => self.root_dir(),
            BundleFormat::Server => self.main_exe(),
            _ => bail!("Codesigning is only supported for MacOS and iOS bundles"),
        };

        // iOS devices require the provisioning profile to be embedded in the .app bundle
        if self.bundle == BundleFormat::Ios {
            if let Some(profile_path) = &provisioning_profile_path {
                let dest = target_exe.join("embedded.mobileprovision");
                std::fs::copy(profile_path, &dest)
                    .context("Failed to embed provisioning profile into .app bundle")?;
            }
        }

        // codesign the app
        let output = Command::new("codesign")
            .args([
                "--force",
                "--entitlements",
                entitlements_file.to_str().unwrap(),
                "--sign",
                app_dev_name,
            ])
            .arg(target_exe)
            .output()
            .await
            .context("Failed to codesign the app - is `codesign` in your path?")?;

        if !output.status.success() {
            bail!(
                "Failed to codesign the app: {}",
                String::from_utf8(output.stderr).unwrap_or_default()
            );
        }

        Ok(())
    }

    async fn auto_provision_signing_name() -> Result<String> {
        let identities = Command::new("security")
            .args(["find-identity", "-v", "-p", "codesigning"])
            .output()
            .await
            .context("Failed to run `security find-identity -v -p codesigning` - is `security` in your path?")
            .map(|e| {
                String::from_utf8(e.stdout)
                    .context("Failed to parse `security find-identity -v -p codesigning`")
            })??;

        // Parsing this:
        // 1231231231231asdasdads123123 "Apple Development: foo@gmail.com (XYZYZY)"
        let app_dev_name = regex::Regex::new(r#""Apple Development: (.+)""#)
            .unwrap()
            .captures(&identities)
            .and_then(|caps| caps.get(1))
            .map(|m| m.as_str())
            .context(
                "Failed to find Apple Development in `security find-identity -v -p codesigning`",
            )?;

        Ok(app_dev_name.to_string())
    }

    /// Enrich auto-provisioned entitlements XML with config from Dioxus.toml.
    ///
    /// Injects entitlements from `[ios.entitlements]` or `[macos.entitlements]` sections
    /// and associated domains from `[deep_links]` into the base entitlements XML.
    fn enrich_entitlements_from_config(&self, base_xml: String) -> Result<String> {
        let mut extra_entries = String::new();

        match self.bundle {
            BundleFormat::Ios => {
                let ent = &self.config.ios.entitlements;

                // Associated domains (from deep_links.hosts + ios.entitlements.associated-domains)
                let mapper = ManifestMapper::from_config(
                    &self.config.permissions,
                    &self.config.deep_links,
                    &self.config.background,
                    &self.config.android,
                    &self.config.ios,
                    &self.config.macos,
                );
                let mut domains: Vec<String> = mapper.ios_associated_domains;
                domains.extend(ent.associated_domains.clone());
                domains.dedup();
                if !domains.is_empty() {
                    extra_entries.push_str(
                        "    <key>com.apple.developer.associated-domains</key>\n    <array>\n",
                    );
                    for domain in &domains {
                        extra_entries.push_str(&format!("        <string>{domain}</string>\n"));
                    }
                    extra_entries.push_str("    </array>\n");
                }

                // App groups
                if !ent.app_groups.is_empty() {
                    extra_entries.push_str(
                        "    <key>com.apple.security.application-groups</key>\n    <array>\n",
                    );
                    for group in &ent.app_groups {
                        extra_entries.push_str(&format!("        <string>{group}</string>\n"));
                    }
                    extra_entries.push_str("    </array>\n");
                }

                // APS environment (push notifications)
                if let Some(env) = &ent.aps_environment {
                    extra_entries.push_str(&format!(
                        "    <key>aps-environment</key>\n    <string>{env}</string>\n"
                    ));
                }

                // iCloud
                if ent.icloud {
                    extra_entries.push_str(
                        "    <key>com.apple.developer.icloud-container-identifiers</key>\n    <array/>\n\
                         <key>com.apple.developer.icloud-services</key>\n    <array>\n        <string>CloudDocuments</string>\n    </array>\n"
                    );
                }

                // Keychain access groups
                // (base entitlements already include one from provisioning profile, only add extras)
                if !ent.keychain_access_groups.is_empty() {
                    extra_entries.push_str("    <key>keychain-access-groups</key>\n    <array>\n");
                    for group in &ent.keychain_access_groups {
                        extra_entries.push_str(&format!("        <string>{group}</string>\n"));
                    }
                    extra_entries.push_str("    </array>\n");
                }

                // Apple Pay
                if ent.apple_pay {
                    extra_entries.push_str(
                        "    <key>com.apple.developer.in-app-payments</key>\n    <array>\n        <string>merchant.*</string>\n    </array>\n"
                    );
                }

                // HealthKit
                if ent.healthkit {
                    extra_entries
                        .push_str("    <key>com.apple.developer.healthkit</key>\n    <true/>\n");
                }

                // HomeKit
                if ent.homekit {
                    extra_entries
                        .push_str("    <key>com.apple.developer.homekit</key>\n    <true/>\n");
                }

                // Additional entitlements from the flat map
                for (key, value) in &ent.additional {
                    extra_entries.push_str(&format!(
                        "    <key>{key}</key>\n    {}\n",
                        value_to_plist_xml(value, 1)
                    ));
                }

                // Raw entitlements XML
                if let Some(raw) = &self.config.ios.raw.entitlements {
                    extra_entries.push_str(raw);
                    extra_entries.push('\n');
                }
            }
            BundleFormat::MacOS => {
                let ent = &self.config.macos.entitlements;

                // App Sandbox
                if let Some(v) = ent.app_sandbox {
                    extra_entries.push_str(&format!(
                        "    <key>com.apple.security.app-sandbox</key>\n    <{v}/>\n"
                    ));
                }

                // File access
                if let Some(true) = ent.files_user_selected {
                    extra_entries.push_str(
                        "    <key>com.apple.security.files.user-selected.read-write</key>\n    <true/>\n"
                    );
                }
                if let Some(true) = ent.files_user_selected_readonly {
                    extra_entries.push_str(
                        "    <key>com.apple.security.files.user-selected.read-only</key>\n    <true/>\n"
                    );
                }

                // Network
                if let Some(true) = ent.network_client {
                    extra_entries.push_str(
                        "    <key>com.apple.security.network.client</key>\n    <true/>\n",
                    );
                }
                if let Some(true) = ent.network_server {
                    extra_entries.push_str(
                        "    <key>com.apple.security.network.server</key>\n    <true/>\n",
                    );
                }

                // Device access
                if let Some(true) = ent.camera {
                    extra_entries
                        .push_str("    <key>com.apple.security.device.camera</key>\n    <true/>\n");
                }
                if let Some(true) = ent.microphone {
                    extra_entries.push_str(
                        "    <key>com.apple.security.device.microphone</key>\n    <true/>\n",
                    );
                }
                if let Some(true) = ent.usb {
                    extra_entries
                        .push_str("    <key>com.apple.security.device.usb</key>\n    <true/>\n");
                }
                if let Some(true) = ent.bluetooth {
                    extra_entries.push_str(
                        "    <key>com.apple.security.device.bluetooth</key>\n    <true/>\n",
                    );
                }
                if let Some(true) = ent.print {
                    extra_entries
                        .push_str("    <key>com.apple.security.print</key>\n    <true/>\n");
                }

                // Personal information
                if let Some(true) = ent.location {
                    extra_entries.push_str(
                        "    <key>com.apple.security.personal-information.location</key>\n    <true/>\n"
                    );
                }
                if let Some(true) = ent.addressbook {
                    extra_entries.push_str(
                        "    <key>com.apple.security.personal-information.addressbook</key>\n    <true/>\n"
                    );
                }
                if let Some(true) = ent.calendars {
                    extra_entries.push_str(
                        "    <key>com.apple.security.personal-information.calendars</key>\n    <true/>\n"
                    );
                }

                // Runtime exceptions
                if let Some(true) = ent.disable_library_validation {
                    extra_entries.push_str(
                        "    <key>com.apple.security.cs.disable-library-validation</key>\n    <true/>\n"
                    );
                }
                if let Some(true) = ent.allow_jit {
                    extra_entries
                        .push_str("    <key>com.apple.security.cs.allow-jit</key>\n    <true/>\n");
                }
                if let Some(true) = ent.allow_unsigned_executable_memory {
                    extra_entries.push_str(
                        "    <key>com.apple.security.cs.allow-unsigned-executable-memory</key>\n    <true/>\n"
                    );
                }

                // Additional entitlements from the flat map
                for (key, value) in &ent.additional {
                    extra_entries.push_str(&format!(
                        "    <key>{key}</key>\n    {}\n",
                        value_to_plist_xml(value, 1)
                    ));
                }

                // Raw entitlements XML
                if let Some(raw) = &self.config.macos.raw.entitlements {
                    extra_entries.push_str(raw);
                    extra_entries.push('\n');
                }
            }
            _ => {}
        }

        if extra_entries.is_empty() {
            return Ok(base_xml);
        }

        // Insert before closing </dict></plist>
        if let Some(pos) = base_xml.rfind("</dict>") {
            let mut enriched = base_xml[..pos].to_string();
            enriched.push_str(&extra_entries);
            enriched.push_str(&base_xml[pos..]);
            Ok(enriched)
        } else {
            tracing::warn!("Could not find </dict> in entitlements XML to inject config entries");
            Ok(base_xml)
        }
    }

    async fn auto_provision_entitlements(bundle_id: &str) -> Result<(String, PathBuf)> {
        const CODESIGN_ERROR: &str = r#"This is likely because you haven't
- Created a provisioning profile before
- Accepted the Apple Developer Program License Agreement

The agreement changes frequently and might need to be accepted again.
To accept the agreement, go to https://developer.apple.com/account

To create a provisioning profile, follow the instructions here:
https://developer.apple.com/documentation/xcode/sharing-your-teams-signing-certificates"#;

        // Check the xcode 16 location first
        let mut profiles_folder = dirs::home_dir()
            .context("Your machine has no home-dir")?
            .join("Library/Developer/Xcode/UserData/Provisioning Profiles");

        // If it doesn't exist, check the old location
        if !profiles_folder.exists() {
            profiles_folder = dirs::home_dir()
                .context("Your machine has no home-dir")?
                .join("Library/MobileDevice/Provisioning Profiles");
        }

        if !profiles_folder.exists() || profiles_folder.read_dir()?.next().is_none() {
            tracing::error!(
                r#"No provisioning profiles found when trying to codesign the app.
We checked the folders:
- XCode16: ~/Library/Developer/Xcode/UserData/Provisioning Profiles
- XCode15: ~/Library/MobileDevice/Provisioning Profiles

{CODESIGN_ERROR}
"#
            )
        }

        #[derive(serde::Deserialize, Debug)]
        struct ProvisioningProfile {
            #[serde(rename = "TeamIdentifier")]
            team_identifier: Vec<String>,
            #[serde(rename = "Entitlements")]
            entitlements: ProfileEntitlements,
            #[allow(dead_code)]
            #[serde(rename = "ApplicationIdentifierPrefix")]
            application_identifier_prefix: Vec<String>,
            #[serde(rename = "ProvisionedDevices", default)]
            provisioned_devices: Vec<String>,
        }

        #[derive(serde::Deserialize, Debug)]
        struct ProfileEntitlements {
            #[serde(rename = "application-identifier")]
            application_identifier: String,
            #[serde(rename = "keychain-access-groups")]
            keychain_access_groups: Vec<String>,
        }

        // The .mobileprovision file has some random binary thrown into it, but it's still basically a plist
        // Let's use the plist markers to find the start and end of the plist
        fn cut_plist(bytes: &[u8], byte_match: &[u8]) -> Option<usize> {
            bytes
                .windows(byte_match.len())
                .enumerate()
                .rev()
                .find(|(_, slice)| *slice == byte_match)
                .map(|(i, _)| i + byte_match.len())
        }

        fn parse_profile(path: &Path) -> Result<ProvisioningProfile> {
            let bytes = std::fs::read(path)?;
            let cut1 =
                cut_plist(&bytes, b"<plist").context("Failed to parse .mobileprovision file")?;
            let cut2 = cut_plist(&bytes, r#"</dict>"#.as_bytes())
                .context("Failed to parse .mobileprovision file")?;
            let sub_bytes = &bytes[(cut1 - 6)..cut2];
            plist::from_bytes(sub_bytes).context("Failed to parse .mobileprovision file")
        }

        /// Check if a provisioning profile's application-identifier matches the given bundle ID.
        /// The app ID is in the format "TEAMID.com.example.app" or "TEAMID.*" for wildcard profiles.
        fn profile_matches_bundle_id(app_identifier: &str, bundle_id: &str) -> bool {
            // Strip the team ID prefix (everything before and including the first dot)
            let app_id_suffix = match app_identifier.split_once('.') {
                Some((_, suffix)) => suffix,
                None => return false,
            };

            // Wildcard profile matches everything
            if app_id_suffix == "*" {
                return true;
            }

            // Check exact match
            if app_id_suffix == bundle_id {
                return true;
            }

            // Check wildcard prefix match (e.g. "com.example.*" matches "com.example.app")
            if let Some(prefix) = app_id_suffix.strip_suffix(".*") {
                return bundle_id.starts_with(prefix);
            }

            false
        }

        // Collect all provisioning profiles and find the best match for the bundle ID.
        // Priority: exact app ID match > more provisioned devices > newer file.
        let mut best_match: Option<(PathBuf, ProvisioningProfile, bool, usize)> = None;

        for entry in profiles_folder.read_dir()?.flatten() {
            let path = entry.path();
            let is_mobileprovision = path
                .extension()
                .map(|e| e == "mobileprovision")
                .unwrap_or(false);

            if !is_mobileprovision {
                continue;
            }

            let profile = match parse_profile(&path) {
                Ok(p) => p,
                Err(e) => {
                    tracing::debug!("Skipping profile {}: {e}", path.display());
                    continue;
                }
            };

            let app_id = &profile.entitlements.application_identifier;
            if !profile_matches_bundle_id(app_id, bundle_id) {
                tracing::debug!(
                    "Skipping profile {} (app ID {app_id} does not match bundle ID {bundle_id})",
                    path.display()
                );
                continue;
            }

            let is_exact = !app_id.ends_with(".*") && !app_id.ends_with("*");
            let num_devices = profile.provisioned_devices.len();

            tracing::debug!(
                "Found matching profile {} (app ID: {app_id}, exact: {is_exact}, devices: {num_devices})",
                path.display()
            );

            // Prefer: exact match > more provisioned devices (newer profiles have more devices)
            let dominated = match &best_match {
                Some((_, _, prev_exact, prev_devices)) => {
                    if *prev_exact && !is_exact {
                        true // existing exact match beats wildcard
                    } else if is_exact && !*prev_exact {
                        false // new exact match beats existing wildcard
                    } else {
                        // same specificity — prefer more provisioned devices
                        num_devices <= *prev_devices
                    }
                }
                None => false,
            };

            if !dominated {
                best_match = Some((path, profile, is_exact, num_devices));
            }
        }

        let (profile_path, mbfile) = match best_match {
            Some((path, profile, _, _)) => {
                tracing::info!(
                    "Using provisioning profile: {} (app ID: {})",
                    path.display(),
                    profile.entitlements.application_identifier
                );
                (path, profile)
            }
            None => {
                bail!(
                    "No provisioning profile found matching bundle identifier \"{bundle_id}\".\n\
                     \n\
                     Your provisioning profiles are in: {}\n\
                     \n\
                     To fix this, either:\n  \
                     1. Set `bundle.identifier` in Dioxus.toml to match an existing profile\n  \
                     2. Create a wildcard provisioning profile in your Apple Developer account\n  \
                     3. Open the project in Xcode and let it auto-provision\n\
                     \n\
                     {CODESIGN_ERROR}",
                    profiles_folder.display()
                );
            }
        };

        let entitlements_xml = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
    <key>application-identifier</key>
    <string>{APPLICATION_IDENTIFIER}</string>
    <key>keychain-access-groups</key>
    <array>
        <string>{APP_ID_ACCESS_GROUP}.*</string>
    </array>
    <key>get-task-allow</key>
    <true/>
    <key>com.apple.developer.team-identifier</key>
    <string>{TEAM_IDENTIFIER}</string>
</dict></plist>
        "#,
            APPLICATION_IDENTIFIER = mbfile.entitlements.application_identifier,
            APP_ID_ACCESS_GROUP = mbfile.entitlements.keychain_access_groups[0],
            TEAM_IDENTIFIER = mbfile.team_identifier[0],
        );

        Ok((entitlements_xml, profile_path))
    }

    /// Bundle and compile Swift packages from source into dynamic frameworks.
    ///
    /// This function:
    /// 1. Calls ios_swift::compile_swift_sources to compile Swift packages
    /// 2. The function creates proper .framework bundles from the dylibs
    /// 3. Installs the frameworks to the app's Frameworks folder
    pub async fn compile_swift_sources(
        &self,
        swift_sources: &[SwiftPackageMetadata],
    ) -> Result<()> {
        if swift_sources.is_empty() {
            return Ok(());
        }

        let build_dir = self.target_dir.join("swift-build");
        std::fs::create_dir_all(&build_dir)?;

        // Compile Swift sources and get the framework bundle path
        let framework_path = super::apple::compile_swift_sources(
            swift_sources,
            &self.triple,
            &build_dir,
            self.release,
        )
        .await?;

        // If a framework was created, install it to the Frameworks folder
        if let Some(framework) = framework_path {
            self.install_swift_framework(&framework).await?;
        }

        Ok(())
    }

    /// Install a Swift framework bundle into the app's Frameworks directory.
    async fn install_swift_framework(&self, framework_path: &Path) -> Result<()> {
        let frameworks_dir = self.frameworks_folder();
        std::fs::create_dir_all(&frameworks_dir)?;

        let framework_name = framework_path
            .file_name()
            .ok_or_else(|| anyhow::anyhow!("Invalid framework path: no filename"))?;
        let dest = frameworks_dir.join(framework_name);

        // Remove existing framework if present
        if dest.exists() {
            std::fs::remove_dir_all(&dest)?;
        }

        // Copy the entire framework bundle
        self.copy_build_dir_recursive(framework_path, &dest)?;

        tracing::debug!(
            "Installed Swift framework '{}' to {}",
            framework_name.to_string_lossy(),
            frameworks_dir.display()
        );

        Ok(())
    }

    /// Embed Swift standard libraries into the app bundle when Swift plugins are present.
    pub async fn embed_swift_stdlibs(&self, swift_sources: &[SwiftPackageMetadata]) -> Result<()> {
        if swift_sources.is_empty() {
            return Ok(());
        }

        let platform_flag = match self.bundle {
            BundleFormat::Ios => {
                let triple_str = self.triple.to_string();
                if triple_str.contains("sim") || triple_str.contains("x86_64") {
                    "iphonesimulator"
                } else {
                    "iphoneos"
                }
            }
            BundleFormat::MacOS => "macosx",
            _ => return Ok(()),
        };

        let frameworks_dir = self.frameworks_folder();
        std::fs::create_dir_all(&frameworks_dir)?;

        let exe_path = self.main_exe();
        if !exe_path.exists() {
            anyhow::bail!(
                "Expected executable at {} when embedding Swift stdlibs",
                exe_path.display()
            );
        }

        // Use swift-stdlib-tool to copy Swift runtime libraries needed by:
        // 1. The main executable (--scan-executable)
        // 2. Any Swift frameworks in the Frameworks folder (--scan-folder)
        let output = Command::new("xcrun")
            .arg("swift-stdlib-tool")
            .arg("--copy")
            .arg("--platform")
            .arg(platform_flag)
            .arg("--scan-executable")
            .arg(&exe_path)
            .arg("--scan-folder")
            .arg(&frameworks_dir)
            .arg("--destination")
            .arg(&frameworks_dir)
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            anyhow::bail!(
                "swift-stdlib-tool failed: {}{}",
                stderr.trim(),
                if stdout.trim().is_empty() {
                    "".to_string()
                } else {
                    format!(" | {}", stdout.trim())
                }
            );
        }

        Ok(())
    }

    /// Compile and install Apple Widget Extensions from Dioxus.toml config.
    ///
    /// This processes widget extensions declared in `[[ios.widget_extensions]]` by:
    /// 1. Compiling the Swift package as a Widget Extension executable
    /// 2. Creating the .appex bundle structure with Info.plist
    /// 3. Installing to the app's PlugIns folder
    pub async fn compile_widget_extensions(&self) -> Result<()> {
        let widget_configs = &self.config.ios.widget_extensions;
        if widget_configs.is_empty() {
            return Ok(());
        }

        tracing::debug!(
            "Compiling {} Apple Widget Extension(s)",
            widget_configs.len()
        );

        let build_dir = self.target_dir.join("widget-build");
        std::fs::create_dir_all(&build_dir)?;

        let app_bundle_id = self.bundle_identifier();
        let default_deployment_target = self
            .config
            .ios
            .deployment_target
            .as_deref()
            .unwrap_or("16.0");

        let plugins_dir = self.plugins_folder();
        std::fs::create_dir_all(&plugins_dir)?;

        for widget_config in widget_configs {
            let source_path = self.package_manifest_dir().join(&widget_config.source);
            let deployment_target = widget_config
                .deployment_target
                .as_deref()
                .unwrap_or(default_deployment_target);

            let widget_source = super::apple::AppleWidgetSource {
                source_path,
                display_name: widget_config.display_name.clone(),
                bundle_id_suffix: widget_config.bundle_id_suffix.clone(),
                deployment_target: deployment_target.to_string(),
                module_name: widget_config.module_name.clone(),
            };

            let appex_path = super::apple::compile_apple_widget(
                &widget_source,
                &self.triple,
                &build_dir,
                &app_bundle_id,
                self.release,
            )
            .await
            .with_context(|| {
                format!(
                    "Failed to compile widget extension '{}'",
                    widget_source.display_name
                )
            })?;

            // Install the .appex bundle to PlugIns/
            let appex_name = appex_path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "Widget.appex".to_string());
            let dest_path = plugins_dir.join(&appex_name);

            if dest_path.exists() {
                std::fs::remove_dir_all(&dest_path)?;
            }

            self.copy_build_dir_recursive(&appex_path, &dest_path)?;

            tracing::debug!(
                "Installed widget extension '{}' to {}",
                widget_source.display_name,
                dest_path.display()
            );
        }

        Ok(())
    }
}

/// Compile Swift sources and return the path to the dynamic framework bundle.
///
/// This function:
/// 1. Generates an umbrella Package.swift that includes all Swift plugins
/// 2. Runs `swift build` to compile into a dynamic library
/// 3. Wraps the dylib in a proper .framework bundle for iOS/macOS
/// 4. Returns the path to the resulting `.framework` bundle
async fn compile_swift_sources(
    swift_sources: &[SwiftPackageMetadata],
    target_triple: &Triple,
    build_dir: &Path,
    release: bool,
) -> Result<Option<PathBuf>> {
    if swift_sources.is_empty() {
        return Ok(None);
    }

    tracing::debug!(
        "Compiling {} Swift plugin(s) for {}",
        swift_sources.len(),
        target_triple
    );

    // Create the plugins build directory
    let plugins_dir = build_dir.join("swift-plugins");
    std::fs::create_dir_all(&plugins_dir)?;

    // Copy and prepare all Swift source packages
    let mut plugin_paths = Vec::new();
    for source in swift_sources {
        let source_path = PathBuf::from(source.package_path.as_str());
        let plugin_name = source.plugin_name.as_str();
        let product_name = source.product.as_str();

        if !source_path.exists() {
            tracing::warn!(
                "Swift package path does not exist: {} (for plugin {})",
                source_path.display(),
                plugin_name
            );
            continue;
        }

        let dest_path = plugins_dir.join(plugin_name);
        if dest_path.exists() {
            std::fs::remove_dir_all(&dest_path)?;
        }
        copy_dir_recursive(&source_path, &dest_path)?;

        // Modify the Package.swift to produce a dynamic library
        if let Err(e) = modify_package_for_dynamic_library(&dest_path, product_name) {
            tracing::warn!("Failed to modify Package.swift for dynamic library: {}", e);
        }

        plugin_paths.push((plugin_name.to_string(), product_name.to_string(), dest_path));
        tracing::debug!(
            "Copied Swift plugin '{}' from {} to {}",
            plugin_name,
            source_path.display(),
            plugins_dir.join(plugin_name).display()
        );
    }

    if plugin_paths.is_empty() {
        tracing::warn!("No valid Swift packages found to compile");
        return Ok(None);
    }

    // Determine Swift target triple and SDK
    let (swift_triple, sdk_name) = swift_target_and_sdk(target_triple)?;
    let sdk_path = lookup_sdk_path(&sdk_name).await?;

    // Build configuration
    let configuration = if release { "release" } else { "debug" };

    // Build each plugin package individually
    for (plugin_name, product_name, package_path) in &plugin_paths {
        tracing::debug!(
            "Building Swift plugin '{}' (product: {})",
            plugin_name,
            product_name
        );

        let build_path = package_path.join(".build");

        let mut cmd = Command::new("xcrun");
        cmd.args(["swift", "build"])
            .arg("--package-path")
            .arg(package_path)
            .arg("--configuration")
            .arg(configuration)
            .arg("--triple")
            .arg(&swift_triple)
            .arg("--sdk")
            .arg(&sdk_path)
            .arg("--product")
            .arg(product_name)
            .arg("--build-path")
            .arg(&build_path);

        tracing::debug!("Running: xcrun swift build for {}", product_name);

        let output = cmd.output().await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            anyhow::bail!(
                "Swift build failed for plugin '{}':\n{}\n{}",
                plugin_name,
                stdout,
                stderr
            );
        }

        if !output.stderr.is_empty() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            tracing::debug!("Swift build warnings for {}:\n{}", plugin_name, stderr);
        }
    }

    // Find the output dynamic library for each plugin
    // Swift puts the output in .build/<triple>/<configuration>/lib<ProductName>.dylib
    // or .build/<configuration>/lib<ProductName>.dylib depending on the version
    let mut all_dylibs = Vec::new();

    for (_, product_name, package_path) in &plugin_paths {
        let build_path = package_path.join(".build");
        let lib_name = format!("lib{}.dylib", product_name);

        let lib_search_paths = [
            build_path.join(&swift_triple).join(configuration),
            build_path.join(configuration),
            build_path.clone(),
        ];

        let mut found = false;
        for search_path in &lib_search_paths {
            let lib_path = search_path.join(&lib_name);
            if lib_path.exists() {
                tracing::debug!("Found Swift dynamic library: {}", lib_path.display());
                all_dylibs.push((product_name.clone(), lib_path));
                found = true;
                break;
            }
        }

        if !found {
            tracing::warn!(
                "Could not find compiled Swift dynamic library for product '{}' (expected {})",
                product_name,
                lib_name
            );
        }
    }

    if all_dylibs.is_empty() {
        tracing::warn!("No Swift dynamic libraries were compiled successfully");
        return Ok(None);
    }

    // For dynamic libraries, we need to wrap each in a framework bundle
    // If there's only one library, create a single framework
    // If there are multiple, we'll create frameworks for each (they're independent)
    // The first one is the "primary" framework that gets returned

    let (_primary_name, primary_dylib) = all_dylibs.remove(0);

    // Create the framework bundle from the dylib
    // Use "DioxusSwiftPlugins" as the umbrella framework name
    let framework_name = "DioxusSwiftPlugins";
    let bundle_identifier = "com.dioxus.swift.plugins";

    let framework_path = create_framework_bundle(
        &primary_dylib,
        framework_name,
        build_dir,
        target_triple,
        bundle_identifier,
    )
    .await?;

    // If there are additional dylibs, create separate framework bundles for them
    for (name, dylib_path) in all_dylibs {
        let extra_framework = create_framework_bundle(
            &dylib_path,
            &name,
            build_dir,
            target_triple,
            &format!("com.dioxus.swift.{}", name.to_lowercase()),
        )
        .await?;
        tracing::debug!(
            "Created additional framework: {}",
            extra_framework.display()
        );
    }

    Ok(Some(framework_path))
}

/// Modify a Package.swift to produce a dynamic library instead of static.
/// This is needed for runtime class lookup via NSClassFromString.
fn modify_package_for_dynamic_library(package_path: &Path, product_name: &str) -> Result<()> {
    let package_swift_path = package_path.join("Package.swift");
    if !package_swift_path.exists() {
        anyhow::bail!(
            "Package.swift not found at {}",
            package_swift_path.display()
        );
    }

    let content = std::fs::read_to_string(&package_swift_path)?;

    // Replace .static with .dynamic for the library type
    let modified = content
        .replace("type: .static", "type: .dynamic")
        .replace("type:.static", "type: .dynamic");

    // If no library type was specified, we need to add it
    // Look for .library(name: "ProductName", targets: [...]) and change to
    // .library(name: "ProductName", type: .dynamic, targets: [...])
    let pattern = format!(
        r#".library\s*\(\s*name\s*:\s*"{}"\s*,\s*targets"#,
        regex::escape(product_name)
    );
    let replacement = format!(
        r#".library(name: "{}", type: .dynamic, targets"#,
        product_name
    );

    let modified = if let Ok(re) = regex::Regex::new(&pattern) {
        re.replace_all(&modified, replacement.as_str()).to_string()
    } else {
        modified
    };

    std::fs::write(&package_swift_path, modified)?;
    Ok(())
}

/// Convert a Rust target triple to Swift target triple and SDK name
fn swift_target_and_sdk(triple: &Triple) -> Result<(String, String)> {
    use target_lexicon::{Architecture, Environment, OperatingSystem};

    // Check if this is a simulator target using the environment field
    let is_simulator = triple.environment == Environment::Sim;

    let swift_triple = match (&triple.architecture, &triple.operating_system) {
        (Architecture::Aarch64(_), OperatingSystem::IOS(_)) => {
            if is_simulator {
                "arm64-apple-ios-simulator"
            } else {
                "arm64-apple-ios"
            }
        }
        (Architecture::Aarch64(_), OperatingSystem::MacOSX { .. } | OperatingSystem::Darwin(_)) => {
            "arm64-apple-macosx"
        }
        (Architecture::X86_64, OperatingSystem::IOS(_)) => "x86_64-apple-ios-simulator",
        (Architecture::X86_64, OperatingSystem::MacOSX { .. } | OperatingSystem::Darwin(_)) => {
            "x86_64-apple-macosx"
        }
        _ => anyhow::bail!("Unsupported target for Swift compilation: {}", triple),
    };

    let sdk_name = match &triple.operating_system {
        OperatingSystem::IOS(_) => {
            // Check if this is a simulator target using the environment field
            if is_simulator {
                "iphonesimulator"
            } else {
                "iphoneos"
            }
        }
        OperatingSystem::MacOSX { .. } | OperatingSystem::Darwin(_) => "macosx",
        _ => anyhow::bail!(
            "Unsupported operating system for Swift compilation: {:?}",
            triple.operating_system
        ),
    };

    Ok((swift_triple.to_string(), sdk_name.to_string()))
}

/// Look up the SDK path using xcrun
async fn lookup_sdk_path(sdk_name: &str) -> Result<String> {
    let output = Command::new("xcrun")
        .args(["--sdk", sdk_name, "--show-sdk-path"])
        .output()
        .await
        .context("Failed to run xcrun to find SDK path")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Failed to find SDK '{}': {}", sdk_name, stderr);
    }

    let sdk_path = String::from_utf8(output.stdout)
        .context("Invalid UTF-8 in SDK path")?
        .trim()
        .to_string();

    if sdk_path.is_empty() {
        anyhow::bail!("SDK path for '{}' is empty", sdk_name);
    }

    Ok(sdk_path)
}

/// Recursively copy a directory
fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    std::fs::create_dir_all(dst)?;

    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if ty.is_dir() {
            // Skip .build directories
            if entry.file_name() == ".build" {
                continue;
            }
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path)?;
        }
    }

    Ok(())
}

/// Extract Swift metadata from object files in link arguments
pub fn extract_swift_metadata_from_link_args(
    link_args: &[String],
    workspace_dir: &Path,
) -> Vec<SwiftPackageMetadata> {
    let mut swift_packages = Vec::new();

    // Look through rlibs and object files for Swift metadata
    for arg in link_args {
        let path = PathBuf::from(arg);

        // Only process files in our workspace
        if !path.starts_with(workspace_dir) {
            continue;
        }

        // Check for .rlib files
        if arg.ends_with(".rlib") {
            if let Ok(swift_meta) = extract_swift_from_rlib(&path) {
                swift_packages.extend(swift_meta);
            }
        }
        // Check for .o files
        else if arg.ends_with(".o") || arg.ends_with(".obj") {
            if let Ok(swift_meta) = extract_swift_from_object(&path) {
                swift_packages.extend(swift_meta);
            }
        }
    }

    // Deduplicate by plugin name
    swift_packages.sort_by(|a, b| a.plugin_name.as_str().cmp(b.plugin_name.as_str()));
    swift_packages.dedup_by(|a, b| a.plugin_name.as_str() == b.plugin_name.as_str());

    swift_packages
}

/// Extract Swift metadata from an rlib file
fn extract_swift_from_rlib(rlib_path: &Path) -> Result<Vec<SwiftPackageMetadata>> {
    let mut results = Vec::new();

    let rlib_contents = std::fs::read(rlib_path)?;
    let mut reader = ar::Archive::new(std::io::Cursor::new(rlib_contents));

    while let Some(Ok(entry)) = reader.next_entry() {
        let name = std::str::from_utf8(entry.header().identifier()).unwrap_or_default();

        // Only process .o files
        if !name.ends_with(".rcgu.o") && !name.ends_with(".obj") {
            continue;
        }

        // Read the object file contents
        let mut obj_contents = Vec::new();
        std::io::Read::read_to_end(&mut std::io::BufReader::new(entry), &mut obj_contents)?;

        if let Ok(swift_meta) = extract_swift_from_bytes(&obj_contents) {
            results.extend(swift_meta);
        }
    }

    Ok(results)
}

/// Extract Swift metadata from an object file
fn extract_swift_from_object(obj_path: &Path) -> Result<Vec<SwiftPackageMetadata>> {
    let obj_contents = std::fs::read(obj_path)?;
    extract_swift_from_bytes(&obj_contents)
}

/// Extract Swift metadata from raw object file bytes
fn extract_swift_from_bytes(bytes: &[u8]) -> Result<Vec<SwiftPackageMetadata>> {
    use manganis_core::SymbolData;
    use object::{Object, ObjectSection, ObjectSymbol};

    let mut results = Vec::new();

    let file = match object::File::parse(bytes) {
        Ok(f) => f,
        Err(_) => return Ok(results),
    };

    // Look for __ASSETS__ symbols
    for symbol in file.symbols() {
        let name = match symbol.name() {
            Ok(n) => n,
            Err(_) => continue,
        };

        if !name.starts_with("__ASSETS__") {
            continue;
        }

        // Try to get the symbol's data
        if let Some(section_idx) = symbol.section().index() {
            if let Ok(section) = file.section_by_index(section_idx) {
                if let Ok(data) = section.data() {
                    // Try to find the symbol data in the section
                    let addr = symbol.address();
                    let section_addr = section.address();
                    let offset = (addr - section_addr) as usize;

                    if offset < data.len() {
                        let symbol_data = &data[offset..];
                        // Try to deserialize as SymbolData
                        if let Some((_, SymbolData::SwiftPackage(meta))) =
                            const_serialize::deserialize_const!(SymbolData, symbol_data)
                        {
                            results.push(meta);
                        }
                    }
                }
            }
        }
    }

    Ok(results)
}

/// Recursively collect all Swift source files from a directory
fn collect_swift_files(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut swift_files = Vec::new();

    if !dir.exists() {
        return Ok(swift_files);
    }

    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            // Recursively collect from subdirectories
            swift_files.extend(collect_swift_files(&path)?);
        } else if path.extension().is_some_and(|ext| ext == "swift") {
            swift_files.push(path);
        }
    }

    Ok(swift_files)
}

/// Information about an Apple Widget Extension to compile
pub struct AppleWidgetSource {
    /// Path to the Swift package source directory
    pub source_path: PathBuf,
    /// Display name for the widget (shown in system UI)
    pub display_name: String,
    /// Bundle ID suffix (appended to app bundle ID)
    pub bundle_id_suffix: String,
    /// Minimum deployment target (e.g., "16.0")
    pub deployment_target: String,
    /// Swift module name for the widget.
    /// This MUST match the module name used by the main app's Swift plugin
    /// for ActivityKit type matching to work (e.g., both must define
    /// `ModuleName.LocationPermissionAttributes` as the same type).
    pub module_name: String,
}

/// Compile an Apple Widget Extension from a Swift package source.
///
/// Widget Extensions are compiled as executables (not libraries) and bundled
/// as .appex bundles which are installed in the app's PlugIns folder.
///
/// **Important**: Widget extensions are XPC services that require special initialization.
/// We use `-e _NSExtensionMain` as the entry point instead of the default `_main` that
/// Swift generates with `@main`. The `_NSExtensionMain` entry point (provided by Foundation):
/// 1. Sets up the XPC listener
/// 2. Initializes ExtensionFoundation's `_EXRunningExtension` singleton
/// 3. Registers with PlugInKit
/// 4. Then calls your Widget code
///
/// # Arguments
/// * `widget` - Widget extension source configuration
/// * `target_triple` - The target platform (e.g., aarch64-apple-ios)
/// * `build_dir` - Directory for intermediate build files
/// * `app_bundle_id` - The main app's bundle identifier (widget ID is derived from this)
/// * `release` - Whether to build in release mode
///
/// # Returns
/// Path to the compiled .appex bundle, ready to be installed to PlugIns/
pub async fn compile_apple_widget(
    widget: &AppleWidgetSource,
    target_triple: &Triple,
    build_dir: &Path,
    app_bundle_id: &str,
    release: bool,
) -> Result<PathBuf> {
    use target_lexicon::OperatingSystem;

    // Validate we're on an Apple platform
    let is_ios = matches!(target_triple.operating_system, OperatingSystem::IOS(_));
    let is_macos = matches!(
        target_triple.operating_system,
        OperatingSystem::MacOSX { .. } | OperatingSystem::Darwin(_)
    );

    if !is_ios && !is_macos {
        anyhow::bail!(
            "Apple Widget Extensions are only supported on iOS and macOS, not {:?}",
            target_triple.operating_system
        );
    }

    // Validate source path exists
    if !widget.source_path.exists() {
        anyhow::bail!(
            "Widget Extension source path does not exist: {}",
            widget.source_path.display()
        );
    }

    tracing::debug!(
        "Compiling Apple Widget Extension '{}' for {}",
        widget.display_name,
        target_triple
    );

    // Create the widget build directory
    let widget_build_dir = build_dir.join("widget-extensions");
    std::fs::create_dir_all(&widget_build_dir)?;

    // Copy the Swift package to build directory
    // Use the bundle_id_suffix as a unique name since the folder name might just be "widget"
    let widget_name = widget.bundle_id_suffix.replace("-", "_");
    let source_dir = widget_build_dir.join(format!("{}_src", widget_name));
    if source_dir.exists() {
        std::fs::remove_dir_all(&source_dir)?;
    }
    copy_dir_recursive(&widget.source_path, &source_dir)?;

    // Get Swift target triple and SDK
    let (swift_triple, sdk_name) = swift_target_and_sdk(target_triple)?;

    // Collect all Swift source files from the Sources directory
    let swift_sources_dir = source_dir.join("Sources");
    let swift_files = collect_swift_files(&swift_sources_dir)?;

    if swift_files.is_empty() {
        anyhow::bail!(
            "No Swift source files found in widget extension Sources directory: {}",
            swift_sources_dir.display()
        );
    }

    tracing::debug!(
        "Found {} Swift files for widget: {:?}",
        swift_files.len(),
        swift_files
    );

    // Build output path
    let exec_path = widget_build_dir.join(&widget_name);

    // Compile the widget extension using swiftc directly
    // Widget extensions are XPC services that require _NSExtensionMain as the entry point
    let mut cmd = Command::new("xcrun");
    cmd.arg("--sdk").arg(&sdk_name).arg("swiftc");

    // Add all Swift source files
    for swift_file in &swift_files {
        cmd.arg(swift_file);
    }

    // Output executable
    cmd.arg("-o").arg(&exec_path);

    // Target triple with proper iOS version
    // Format: arm64-apple-ios17.0 or arm64-apple-ios17.0-simulator
    let is_simulator = swift_triple.contains("simulator");
    let base_triple = swift_triple.replace("-simulator", "");
    let swift_target = if is_simulator {
        format!("{}{}-simulator", base_triple, widget.deployment_target)
    } else {
        format!("{}{}", base_triple, widget.deployment_target)
    };
    cmd.arg("-target").arg(&swift_target);

    // Module name - use a consistent name that matches the main app's plugin module
    // This is critical for ActivityKit type matching between app and widget
    cmd.arg("-module-name").arg(&widget.module_name);

    // Optimization flags
    if release {
        cmd.arg("-O").arg("-whole-module-optimization");
    }

    // Extension-specific flags
    cmd.arg("-application-extension");

    // Critical: Use _NSExtensionMain as the entry point for widget extensions
    // Without this, the widget crashes because ExtensionFoundation's singleton isn't initialized
    cmd.arg("-Xlinker")
        .arg("-e")
        .arg("-Xlinker")
        .arg("_NSExtensionMain");

    // Link Objective-C runtime (required for Swift/ObjC interop)
    cmd.arg("-lobjc");

    // Link required frameworks
    cmd.arg("-framework").arg("Foundation");
    cmd.arg("-framework").arg("SwiftUI");
    cmd.arg("-framework").arg("WidgetKit");
    cmd.arg("-framework").arg("ActivityKit");

    tracing::debug!("Running swiftc for widget: {:?}", cmd);

    let output = cmd.output().await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        anyhow::bail!(
            "Swift compilation failed for widget extension '{}':\n{}\n{}",
            widget_name,
            stdout,
            stderr
        );
    }

    tracing::debug!("Compiled widget executable: {}", exec_path.display());

    // Create the .appex bundle
    let appex_name = format!("{}.appex", widget_name);
    let appex_dir = widget_build_dir.join(&appex_name);

    // Remove existing appex if present
    if appex_dir.exists() {
        std::fs::remove_dir_all(&appex_dir)?;
    }
    std::fs::create_dir_all(&appex_dir)?;

    // Copy the executable into the appex bundle
    let bundle_exec = appex_dir.join(&widget_name);
    std::fs::copy(&exec_path, &bundle_exec)?;

    // Create Info.plist for the widget extension
    let widget_bundle_id = format!("{}.{}", app_bundle_id, widget.bundle_id_suffix);
    let min_os_version = &widget.deployment_target;

    let platform_info = if is_ios {
        format!(
            r#"    <key>MinimumOSVersion</key>
    <string>{min_os_version}</string>
    <key>CFBundleSupportedPlatforms</key>
    <array>
        <string>iPhoneOS</string>
    </array>
    <key>UIDeviceFamily</key>
    <array>
        <integer>1</integer>
        <integer>2</integer>
    </array>"#
        )
    } else {
        format!(
            r#"    <key>LSMinimumSystemVersion</key>
    <string>{min_os_version}</string>
    <key>CFBundleSupportedPlatforms</key>
    <array>
        <string>MacOSX</string>
    </array>"#
        )
    };

    let info_plist = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleDevelopmentRegion</key>
    <string>en</string>
    <key>CFBundleDisplayName</key>
    <string>{display_name}</string>
    <key>CFBundleExecutable</key>
    <string>{widget_name}</string>
    <key>CFBundleIdentifier</key>
    <string>{widget_bundle_id}</string>
    <key>CFBundleInfoDictionaryVersion</key>
    <string>6.0</string>
    <key>CFBundleName</key>
    <string>{widget_name}</string>
    <key>CFBundlePackageType</key>
    <string>XPC!</string>
    <key>CFBundleShortVersionString</key>
    <string>1.0</string>
    <key>CFBundleVersion</key>
    <string>1</string>
{platform_info}
    <key>NSExtension</key>
    <dict>
        <key>NSExtensionPointIdentifier</key>
        <string>com.apple.widgetkit-extension</string>
    </dict>
    <key>NSSupportsLiveActivities</key>
    <true/>
</dict>
</plist>"#,
        display_name = widget.display_name,
        widget_name = widget_name,
        widget_bundle_id = widget_bundle_id,
        platform_info = platform_info,
    );

    std::fs::write(appex_dir.join("Info.plist"), info_plist)?;

    tracing::debug!("Created Widget Extension bundle: {}", appex_dir.display());

    Ok(appex_dir)
}

/// Create a proper framework bundle from a dylib for iOS/macOS.
///
/// iOS uses a flat structure while macOS uses a versioned structure.
/// Both require an Info.plist for proper App Store submission.
pub async fn create_framework_bundle(
    dylib_path: &Path,
    framework_name: &str,
    output_dir: &Path,
    target_triple: &Triple,
    bundle_identifier: &str,
) -> Result<PathBuf> {
    let is_ios = matches!(target_triple.operating_system, OperatingSystem::IOS(_));
    let min_os_version = if is_ios { "13.0" } else { "11.0" };

    let framework_dir = output_dir.join(format!("{}.framework", framework_name));

    // Remove existing framework if present
    if framework_dir.exists() {
        std::fs::remove_dir_all(&framework_dir)?;
    }

    if is_ios {
        // iOS uses flat structure: Framework.framework/FrameworkName + Info.plist
        std::fs::create_dir_all(&framework_dir)?;

        // Copy dylib as the framework executable (no extension)
        let exec_path = framework_dir.join(framework_name);
        std::fs::copy(dylib_path, &exec_path)?;

        // Set the install name using install_name_tool
        let output = Command::new("xcrun")
            .arg("install_name_tool")
            .arg("-id")
            .arg(format!(
                "@rpath/{}.framework/{}",
                framework_name, framework_name
            ))
            .arg(&exec_path)
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("install_name_tool failed: {}", stderr);
        }

        // Create Info.plist
        let info_plist = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleDevelopmentRegion</key>
    <string>en</string>
    <key>CFBundleExecutable</key>
    <string>{framework_name}</string>
    <key>CFBundleIdentifier</key>
    <string>{bundle_identifier}</string>
    <key>CFBundleInfoDictionaryVersion</key>
    <string>6.0</string>
    <key>CFBundleName</key>
    <string>{framework_name}</string>
    <key>CFBundlePackageType</key>
    <string>FMWK</string>
    <key>CFBundleShortVersionString</key>
    <string>1.0</string>
    <key>CFBundleVersion</key>
    <string>1</string>
    <key>MinimumOSVersion</key>
    <string>{min_os_version}</string>
    <key>CFBundleSupportedPlatforms</key>
    <array>
        <string>iPhoneOS</string>
    </array>
</dict>
</plist>"#
        );

        std::fs::write(framework_dir.join("Info.plist"), info_plist)?;
    } else {
        // macOS uses versioned structure with symlinks
        let versions_a = framework_dir.join("Versions").join("A");
        let resources_dir = versions_a.join("Resources");
        std::fs::create_dir_all(&resources_dir)?;

        // Copy dylib as the framework executable
        let exec_path = versions_a.join(framework_name);
        std::fs::copy(dylib_path, &exec_path)?;

        // Set install name
        let output = Command::new("xcrun")
            .arg("install_name_tool")
            .arg("-id")
            .arg(format!(
                "@rpath/{}.framework/Versions/A/{}",
                framework_name, framework_name
            ))
            .arg(&exec_path)
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("install_name_tool failed: {}", stderr);
        }

        // Create Info.plist in Resources
        let info_plist = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleDevelopmentRegion</key>
    <string>en</string>
    <key>CFBundleExecutable</key>
    <string>{framework_name}</string>
    <key>CFBundleIdentifier</key>
    <string>{bundle_identifier}</string>
    <key>CFBundleInfoDictionaryVersion</key>
    <string>6.0</string>
    <key>CFBundleName</key>
    <string>{framework_name}</string>
    <key>CFBundlePackageType</key>
    <string>FMWK</string>
    <key>CFBundleShortVersionString</key>
    <string>1.0</string>
    <key>CFBundleVersion</key>
    <string>1</string>
    <key>LSMinimumSystemVersion</key>
    <string>{min_os_version}</string>
</dict>
</plist>"#
        );

        std::fs::write(resources_dir.join("Info.plist"), info_plist)?;

        // Create symbolic links (required for macOS framework structure)
        #[cfg(unix)]
        {
            let versions_dir = framework_dir.join("Versions");
            std::os::unix::fs::symlink("A", versions_dir.join("Current"))?;
            std::os::unix::fs::symlink(
                format!("Versions/Current/{}", framework_name),
                framework_dir.join(framework_name),
            )?;
            std::os::unix::fs::symlink(
                "Versions/Current/Resources",
                framework_dir.join("Resources"),
            )?;
        }
    }

    tracing::debug!(
        "Created {} framework bundle: {}",
        if is_ios { "iOS" } else { "macOS" },
        framework_dir.display()
    );

    Ok(framework_dir)
}

/// Generate plist XML entries from a HashMap of key-value pairs
///
/// Converts a HashMap like `{ "UIBackgroundModes" = ["location", "fetch"] }` to plist XML:
/// ```xml
/// <key>UIBackgroundModes</key>
/// <array>
///     <string>location</string>
///     <string>fetch</string>
/// </array>
/// ```
fn generate_plist_entries(plist: &std::collections::HashMap<String, serde_json::Value>) -> String {
    let mut output = String::new();

    for (key, value) in plist {
        output.push_str(&format!("\t<key>{}</key>\n", key));
        output.push_str(&value_to_plist_xml(value, 1));
    }

    output
}

/// Convert a serde_json::Value to plist XML format
fn value_to_plist_xml(value: &serde_json::Value, indent: usize) -> String {
    let tabs = "\t".repeat(indent);

    match value {
        serde_json::Value::String(s) => format!("{}<string>{}</string>\n", tabs, s),
        serde_json::Value::Bool(b) => {
            if *b {
                format!("{}<true/>\n", tabs)
            } else {
                format!("{}<false/>\n", tabs)
            }
        }
        serde_json::Value::Number(n) => {
            if n.is_i64() {
                format!("{}<integer>{}</integer>\n", tabs, n)
            } else {
                format!("{}<real>{}</real>\n", tabs, n)
            }
        }
        serde_json::Value::Array(arr) => {
            let mut output = format!("{}<array>\n", tabs);
            for item in arr {
                output.push_str(&value_to_plist_xml(item, indent + 1));
            }
            output.push_str(&format!("{}</array>\n", tabs));
            output
        }
        serde_json::Value::Object(obj) => {
            let mut output = format!("{}<dict>\n", tabs);
            for (k, v) in obj {
                output.push_str(&format!("{}\t<key>{}</key>\n", tabs, k));
                output.push_str(&value_to_plist_xml(v, indent + 1));
            }
            output.push_str(&format!("{}</dict>\n", tabs));
            output
        }
        serde_json::Value::Null => String::new(),
    }
}
