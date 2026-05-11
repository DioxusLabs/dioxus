use crate::bundler::{AppCategory, Bundle, BundleContext, copy_dir_recursive};
use crate::{MacOsSettings, PackageType};
use anyhow::{Context, Result, bail};
use image::{DynamicImage, ImageReader};
use std::fs::{self, File};
use std::io::BufWriter;
use std::path::{Path, PathBuf};
use std::process::{Command as StdCommand, Stdio};
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

impl BundleContext<'_> {
    /// Create the final macOS `.app` bundle and apply Apple-specific metadata.
    ///
    /// This is the primary macOS bundling routine. It takes the executable produced by
    /// the Dioxus build pipeline and assembles the canonical app bundle structure in
    /// `project_out_directory()/macos/<BundleName>.app`.
    ///
    /// The method performs the following steps:
    /// 1. Resolve macOS settings and choose the bundle/display name.
    /// 2. Remove any existing output bundle to keep the result deterministic.
    /// 3. Create the `Contents/`, `Contents/MacOS`, `Contents/Resources`, and
    ///    optionally `Contents/Frameworks` directories.
    /// 4. Copy the main executable into `Contents/MacOS/` and mark it executable.
    /// 5. Build or copy an `.icns` file into `Contents/Resources/`.
    /// 6. Copy configured resources and sidecar binaries into the bundle.
    /// 7. Copy configured frameworks and arbitrary extra files from
    ///    `MacOsSettings::files`.
    /// 8. Generate an `Info.plist` from bundle metadata, unless a custom plist path
    ///    was configured and exists.
    /// 9. Write the legacy `PkgInfo` file.
    /// 10. If signing is configured, sign frameworks first, then the main binary, and
    ///     finally the enclosing `.app`.
    /// 11. If notarization credentials are available, zip the `.app`, submit it to
    ///     Apple's notary service, staple the result, and delete the temporary zip.
    ///
    /// Contributor notes:
    /// - The build system guarantees the executable exists, but this method is
    ///   responsible for the final `.app` on-disk shape.
    /// - Framework paths may be absolute or relative to the crate directory. Missing
    ///   paths are tolerated because some entries may refer to system frameworks.
    /// - Lack of signing or notarization credentials is not fatal; an unsigned `.app`
    ///   is still returned as a valid bundle artifact.
    pub(crate) async fn bundle_macos_app(&self) -> Result<Vec<PathBuf>> {
        let product_name = self.product_name();
        let macos_settings = self.macos();

        let bundle_name = macos_settings
            .bundle_name
            .as_deref()
            .unwrap_or(&product_name);

        let output_dir = self.project_out_directory().join("macos");
        let app_dir = output_dir.join(format!("{bundle_name}.app"));

        tracing::info!("Creating macOS .app bundle at {}", app_dir.display());

        if app_dir.exists() {
            fs::remove_dir_all(&app_dir)
                .with_context(|| format!("Failed to clean existing .app: {}", app_dir.display()))?;
        }

        let contents_dir = app_dir.join("Contents");
        let macos_dir = contents_dir.join("MacOS");
        let resources_dir = contents_dir.join("Resources");
        let frameworks_dir = contents_dir.join("Frameworks");

        fs::create_dir_all(&macos_dir)?;
        fs::create_dir_all(&resources_dir)?;

        let binary_src = self.main_binary_path();
        let binary_dest = macos_dir.join(self.main_binary_name());
        tracing::debug!(
            "Copying binary: {} -> {}",
            binary_src.display(),
            binary_dest.display()
        );
        fs::copy(&binary_src, &binary_dest)
            .with_context(|| format!("Failed to copy main binary to {}", binary_dest.display()))?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&binary_dest, fs::Permissions::from_mode(0o755))?;
        }

        let icon_path = self.create_macos_icns_file(&resources_dir)?;
        let icon_filename = icon_path
            .as_ref()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().to_string());

        self.copy_resources(&resources_dir)?;
        let _external_bins = self.copy_external_binaries(&macos_dir)?;

        if let Some(frameworks) = &macos_settings.frameworks {
            if !frameworks.is_empty() {
                fs::create_dir_all(&frameworks_dir)?;
                for framework in frameworks {
                    let framework_path = PathBuf::from(framework);
                    if !framework_path.exists() {
                        let resolved = self.crate_dir().join(&framework_path);
                        if resolved.exists() {
                            copy_framework(&resolved, &frameworks_dir)?;
                        } else {
                            tracing::debug!(
                                "Framework not found as file, assuming system framework: {}",
                                framework
                            );
                        }
                    } else {
                        copy_framework(&framework_path, &frameworks_dir)?;
                    }
                }
            }
        }

        for (bundle_path, source_path) in &macos_settings.files {
            let dest = contents_dir.join(bundle_path);
            let src = if source_path.is_relative() {
                self.crate_dir().join(source_path)
            } else {
                source_path.clone()
            };

            if !src.exists() {
                tracing::warn!(
                    "Custom file not found: {} (for bundle path {})",
                    src.display(),
                    bundle_path.display()
                );
                continue;
            }

            if let Some(parent) = dest.parent() {
                fs::create_dir_all(parent)?;
            }

            if src.is_dir() {
                copy_dir_recursive(&src, &dest)?;
            } else {
                fs::copy(&src, &dest).with_context(|| {
                    format!(
                        "Failed to copy custom file {} -> {}",
                        src.display(),
                        dest.display()
                    )
                })?;
            }
        }

        let info_plist = self.create_macos_info_plist(&macos_settings, icon_filename.as_deref())?;
        let plist_path = contents_dir.join("Info.plist");

        if let Some(custom_plist_path) = &macos_settings.info_plist_path {
            let custom_path = if custom_plist_path.is_relative() {
                self.crate_dir().join(custom_plist_path)
            } else {
                custom_plist_path.clone()
            };
            if custom_path.exists() {
                tracing::info!("Using custom Info.plist from {}", custom_path.display());
                fs::copy(&custom_path, &plist_path)?;
            } else {
                tracing::warn!(
                    "Custom Info.plist not found at {}, generating default",
                    custom_path.display()
                );
                write_plist(&info_plist, &plist_path)?;
            }
        } else {
            write_plist(&info_plist, &plist_path)?;
        }

        fs::write(contents_dir.join("PkgInfo"), "APPL????")?;

        let signing_identity = setup_keychain(macos_settings.signing_identity.as_deref()).await?;

        if let Some(identity) = &signing_identity {
            tracing::info!("Signing .app bundle with identity: {}", identity.identity);

            let mut sign_targets = Vec::new();

            if frameworks_dir.exists() {
                for entry in fs::read_dir(&frameworks_dir)? {
                    let entry = entry?;
                    sign_targets.push(SignTarget { path: entry.path() });
                }
            }

            sign_targets.push(SignTarget {
                path: binary_dest.clone(),
            });
            sign_targets.push(SignTarget {
                path: app_dir.clone(),
            });

            sign_paths(identity, sign_targets, &macos_settings).await?;

            let should_notarize =
                std::env::var("APPLE_ID").is_ok() || std::env::var("APPLE_API_KEY").is_ok();

            if should_notarize {
                let zip_path = output_dir.join(format!("{bundle_name}.zip"));
                tracing::info!("Creating zip for notarization: {}", zip_path.display());

                let status = Command::new("ditto")
                    .args([
                        "-c",
                        "-k",
                        "--sequesterRsrc",
                        "--keepParent",
                        &app_dir.display().to_string(),
                        &zip_path.display().to_string(),
                    ])
                    .status()
                    .await
                    .context("Failed to run ditto for zip creation")?;

                if !status.success() {
                    bail!("ditto failed to create zip for notarization");
                }

                match notarize(&zip_path, &app_dir).await {
                    Ok(()) => {
                        let _ = fs::remove_file(&zip_path);
                    }
                    Err(e) => {
                        let _ = fs::remove_file(&zip_path);
                        return Err(e.context("Notarization failed"));
                    }
                }
            }
        } else {
            tracing::debug!("No signing identity found; skipping code signing");
        }

        Ok(vec![app_dir])
    }

    /// Package the macOS application bundle into a distributable `.dmg` disk image.
    ///
    /// DMG generation depends on having a complete `.app` bundle first. If the
    /// current bundling pass already produced one, this method reuses it. Otherwise it
    /// calls [`BundleContext::bundle_macos_app`] internally and returns both the
    /// resulting `.dmg` and the intermediate `.app` paths so the higher-level
    /// orchestrator can decide whether the `.app` should be preserved or cleaned up.
    ///
    /// The process is:
    /// 1. Resolve the `.app` input, either from `bundles` or by building it now.
    /// 2. Create a temporary staging directory containing the `.app` and an
    ///    `Applications` symlink for drag-and-drop installation.
    /// 3. Invoke `hdiutil create -format UDZO` to build a compressed, read-only disk
    ///    image in `project_out_directory()/macos`.
    /// 4. Optionally sign the generated `.dmg`.
    /// 5. Optionally notarize and staple the `.dmg` if Apple credentials are present.
    ///
    /// Only the final `.dmg` and, when synthesized as a prerequisite, the `.app` are
    /// considered outputs. The temporary DMG staging directory is always discarded.
    pub(crate) async fn bundle_macos_dmg(&self, bundles: &[Bundle]) -> Result<DmgBundled> {
        let product_name = self.product_name();
        let macos_settings = self.macos();

        let bundle_name = macos_settings
            .bundle_name
            .as_deref()
            .unwrap_or(&product_name);

        let (app_paths, app_bundle_paths) = if let Some(app_bundle) = bundles
            .iter()
            .find(|b| b.package_type == PackageType::MacOsBundle)
        {
            (app_bundle.bundle_paths.clone(), Vec::new())
        } else {
            let paths = self.bundle_macos_app().await?;
            (paths.clone(), paths)
        };

        if app_paths.is_empty() {
            bail!("No .app bundle found to package into a DMG");
        }

        let app_path = &app_paths[0];
        if !app_path.exists() {
            bail!(
                ".app bundle does not exist at expected path: {}",
                app_path.display()
            );
        }

        let output_dir = self.project_out_directory().join("macos");
        fs::create_dir_all(&output_dir)?;

        let dmg_filename = format!(
            "{}_{}_{}",
            bundle_name,
            self.version_string(),
            self.binary_arch()
        );
        let dmg_path = output_dir.join(format!("{dmg_filename}.dmg"));

        tracing::info!("Creating DMG at {}", dmg_path.display());

        let staging_dir =
            tempfile::tempdir().context("Failed to create temp dir for DMG staging")?;
        let staging_path = staging_dir.path();

        let staged_app = staging_path.join(app_path.file_name().unwrap());
        copy_dir_recursive(app_path, &staged_app)?;

        #[cfg(unix)]
        {
            std::os::unix::fs::symlink("/Applications", staging_path.join("Applications"))
                .context("Failed to create /Applications symlink in DMG staging")?;
        }

        if dmg_path.exists() {
            fs::remove_file(&dmg_path)?;
        }

        let status = Command::new("hdiutil")
            .args([
                "create",
                "-volname",
                bundle_name,
                "-srcfolder",
                &staging_path.display().to_string(),
                "-ov",
                "-format",
                "UDZO",
                &dmg_path.display().to_string(),
            ])
            .status()
            .await
            .context("Failed to execute `hdiutil create`")?;

        if !status.success() {
            bail!("`hdiutil create` failed with exit code: {status}");
        }

        tracing::info!("DMG created at {}", dmg_path.display());

        let signing_identity = setup_keychain(macos_settings.signing_identity.as_deref()).await?;
        if let Some(identity) = &signing_identity {
            tracing::info!("Signing DMG with identity: {}", identity.identity);
            sign_paths(
                identity,
                vec![SignTarget {
                    path: dmg_path.clone(),
                }],
                &macos_settings,
            )
            .await?;

            let should_notarize =
                std::env::var("APPLE_ID").is_ok() || std::env::var("APPLE_API_KEY").is_ok();

            if should_notarize {
                notarize(&dmg_path, &dmg_path).await?;
            }
        }

        Ok(DmgBundled {
            dmg: vec![dmg_path],
            app: app_bundle_paths,
        })
    }

    /// Create an ICNS file from the icon files configured in the bundle context.
    fn create_macos_icns_file(&self, out_dir: &Path) -> Result<Option<PathBuf>> {
        let icon_paths = self.icon_files()?;
        if icon_paths.is_empty() {
            return Ok(None);
        }

        let dest_path = out_dir.join(format!("{}.icns", self.product_name()));

        for icon_path in &icon_paths {
            if icon_path
                .extension()
                .map(|ext| ext.eq_ignore_ascii_case("icns"))
                .unwrap_or(false)
            {
                tracing::info!("Copying existing .icns file: {}", icon_path.display());
                std::fs::copy(icon_path, &dest_path).with_context(|| {
                    format!(
                        "Failed to copy .icns file from {} to {}",
                        icon_path.display(),
                        dest_path.display()
                    )
                })?;
                return Ok(Some(dest_path));
            }
        }

        let mut family = icns::IconFamily::new();

        for icon_path in &icon_paths {
            let ext = icon_path
                .extension()
                .map(|e| e.to_string_lossy().to_lowercase())
                .unwrap_or_default();

            if ext != "png" {
                tracing::debug!("Skipping non-PNG icon file: {}", icon_path.display());
                continue;
            }

            let img = ImageReader::open(icon_path)
                .with_context(|| format!("Failed to open icon image: {}", icon_path.display()))?
                .decode()
                .with_context(|| format!("Failed to decode icon image: {}", icon_path.display()))?;

            add_image_to_family(&mut family, &img)?;
        }

        if family.is_empty() {
            tracing::warn!("No valid icon images found; skipping .icns generation");
            return Ok(None);
        }

        let file = File::create(&dest_path)
            .with_context(|| format!("Failed to create {}", dest_path.display()))?;
        let writer = BufWriter::new(file);
        family
            .write(writer)
            .with_context(|| format!("Failed to write .icns to {}", dest_path.display()))?;

        tracing::info!("Generated .icns at {}", dest_path.display());
        Ok(Some(dest_path))
    }

    /// Build a `plist::Dictionary` for Info.plist.
    fn create_macos_info_plist(
        &self,
        macos_settings: &MacOsSettings,
        icon_filename: Option<&str>,
    ) -> Result<plist::Dictionary> {
        let mut dict = plist::Dictionary::new();

        let product_name = self.product_name();
        let bundle_name = macos_settings
            .bundle_name
            .as_deref()
            .unwrap_or(&product_name);

        dict.insert(
            "CFBundleDevelopmentRegion".into(),
            plist::Value::String("English".into()),
        );
        dict.insert(
            "CFBundleDisplayName".into(),
            plist::Value::String(bundle_name.to_string()),
        );
        dict.insert(
            "CFBundleExecutable".into(),
            plist::Value::String(self.main_binary_name().to_string()),
        );

        if let Some(icon) = icon_filename {
            dict.insert(
                "CFBundleIconFile".into(),
                plist::Value::String(icon.to_string()),
            );
        }

        dict.insert(
            "CFBundleIdentifier".into(),
            plist::Value::String(self.bundle_identifier()),
        );
        dict.insert(
            "CFBundleInfoDictionaryVersion".into(),
            plist::Value::String("6.0".into()),
        );
        dict.insert(
            "CFBundleName".into(),
            plist::Value::String(bundle_name.to_string()),
        );
        dict.insert(
            "CFBundlePackageType".into(),
            plist::Value::String("APPL".into()),
        );

        let version = self.version_string();
        let bundle_version = macos_settings.bundle_version.as_deref().unwrap_or(&version);

        dict.insert(
            "CFBundleShortVersionString".into(),
            plist::Value::String(version.clone()),
        );
        dict.insert(
            "CFBundleVersion".into(),
            plist::Value::String(bundle_version.to_string()),
        );

        let min_version = macos_settings
            .minimum_system_version
            .as_deref()
            .unwrap_or("10.13");
        dict.insert(
            "LSMinimumSystemVersion".into(),
            plist::Value::String(min_version.to_string()),
        );

        if let Some(category_str) = self.app_category() {
            if let Ok(category) = category_str.parse::<AppCategory>() {
                dict.insert(
                    "LSApplicationCategoryType".into(),
                    plist::Value::String(category.macos_application_category_type().to_string()),
                );
            }
        }

        if let Some(copyright) = self.copyright_string() {
            dict.insert(
                "NSHumanReadableCopyright".into(),
                plist::Value::String(copyright.to_string()),
            );
        }

        dict.insert(
            "NSHighResolutionCapable".into(),
            plist::Value::Boolean(true),
        );

        if let Some(domain) = &macos_settings.exception_domain {
            let mut ats_dict = plist::Dictionary::new();
            let mut exception_dict = plist::Dictionary::new();
            let mut domain_dict = plist::Dictionary::new();
            domain_dict.insert(
                "NSExceptionAllowsInsecureHTTPLoads".into(),
                plist::Value::Boolean(true),
            );
            domain_dict.insert("NSIncludesSubdomains".into(), plist::Value::Boolean(true));
            exception_dict.insert(domain.clone(), plist::Value::Dictionary(domain_dict));
            ats_dict.insert(
                "NSExceptionDomains".into(),
                plist::Value::Dictionary(exception_dict),
            );
            dict.insert(
                "NSAppTransportSecurity".into(),
                plist::Value::Dictionary(ats_dict),
            );
        }

        if let Some(provider) = &macos_settings.provider_short_name {
            dict.insert(
                "ITSAppUsesNonExemptEncryption".into(),
                plist::Value::Boolean(false),
            );
            let _ = provider;
        }

        Ok(dict)
    }
}

/// Add all appropriate size variants of an image to the ICNS family.
fn add_image_to_family(family: &mut icns::IconFamily, img: &DynamicImage) -> Result<()> {
    // The icon sizes (in points) we generate for the .icns file, along with their
    // densities. macOS expects both 1x and 2x variants.
    const ICON_SIZES: &[(u32, u32, u32)] = &[
        (16, 16, 1),
        (16, 16, 2),
        (32, 32, 1),
        (32, 32, 2),
        (64, 64, 1),
        (64, 64, 2),
        (128, 128, 1),
        (128, 128, 2),
        (256, 256, 1),
        (256, 256, 2),
        (512, 512, 1),
        (512, 512, 2),
    ];

    for &(width, height, density) in ICON_SIZES {
        let pixel_width = width * density;
        let pixel_height = height * density;

        let icon_type =
            match icns::IconType::from_pixel_size_and_density(pixel_width, pixel_height, density) {
                Some(t) => t,
                None => continue,
            };

        if family.has_icon_with_type(icon_type) {
            continue;
        }

        let resized = img.resize_exact(
            pixel_width,
            pixel_height,
            image::imageops::FilterType::Lanczos3,
        );

        let rgba = resized.to_rgba8();
        let icns_image = icns::Image::from_data(
            icns::PixelFormat::RGBA,
            pixel_width,
            pixel_height,
            rgba.into_raw(),
        )
        .with_context(|| {
            format!("Failed to create icns::Image for {pixel_width}x{pixel_height}@{density}x")
        })?;

        family
            .add_icon_with_type(&icns_image, icon_type)
            .with_context(|| format!("Failed to add icon type {icon_type:?} to ICNS family"))?;
    }

    Ok(())
}

/// Write a plist dictionary to a file.
fn write_plist(dict: &plist::Dictionary, path: &Path) -> Result<()> {
    plist::Value::Dictionary(dict.clone())
        .to_file_xml(path)
        .with_context(|| format!("Failed to write Info.plist to {}", path.display()))
}

/// Copy a framework (directory or .dylib) to the Frameworks directory.
fn copy_framework(src: &Path, frameworks_dir: &Path) -> Result<()> {
    let dest = frameworks_dir.join(src.file_name().context("Framework path has no filename")?);

    tracing::debug!("Copying framework: {} -> {}", src.display(), dest.display());

    if src.is_dir() {
        copy_dir_recursive(src, &dest)?;
    } else {
        fs::copy(src, &dest)?;
    }

    Ok(())
}

/// Set up the signing identity.
async fn setup_keychain(identity: Option<&str>) -> Result<Option<SigningIdentity>> {
    let certificate_encoded = std::env::var("APPLE_CERTIFICATE").ok();
    let certificate_password = std::env::var("APPLE_CERTIFICATE_PASSWORD")
        .ok()
        .unwrap_or_default();

    if let Some(cert_base64) = certificate_encoded {
        tracing::info!("Setting up temporary keychain for code signing (CI mode)");
        let keychain = setup_temp_keychain(&cert_base64, &certificate_password).await?;
        let identity_name = find_identity_in_keychain(&keychain.path).await?;
        return Ok(Some(SigningIdentity {
            identity: identity_name,
            temp_keychain: Some(keychain),
        }));
    }

    if let Some(id) = identity {
        if !id.is_empty() {
            return Ok(Some(SigningIdentity {
                identity: id.to_string(),
                temp_keychain: None,
            }));
        }
    }

    Ok(None)
}

/// Set up a temporary keychain and import the certificate into it.
async fn setup_temp_keychain(cert_base64: &str, password: &str) -> Result<TempKeychain> {
    use std::io::Write;

    let keychain_password = "dioxus-bundle-keychain";
    let keychain_path = std::env::temp_dir().join("dioxus-signing.keychain-db");

    let cert_data = base64_decode(cert_base64)
        .await
        .context("Failed to decode APPLE_CERTIFICATE from base64")?;

    let cert_file = std::env::temp_dir().join("dioxus-signing-cert.p12");
    let mut f = std::fs::File::create(&cert_file)?;
    f.write_all(&cert_data)?;
    drop(f);

    let _ = Command::new("security")
        .args(["delete-keychain", &keychain_path.display().to_string()])
        .output()
        .await;

    let mut create_keychain_cmd = Command::new("security");
    create_keychain_cmd.args([
        "create-keychain",
        "-p",
        keychain_password,
        &keychain_path.display().to_string(),
    ]);
    run_command(&mut create_keychain_cmd, "create-keychain").await?;

    let mut import_cmd = Command::new("security");
    import_cmd.args([
        "import",
        &cert_file.display().to_string(),
        "-k",
        &keychain_path.display().to_string(),
        "-P",
        password,
        "-T",
        "/usr/bin/codesign",
        "-T",
        "/usr/bin/security",
    ]);
    run_command(&mut import_cmd, "import certificate").await?;

    let output = Command::new("security")
        .args(["list-keychains", "-d", "user"])
        .output()
        .await
        .context("Failed to list keychains")?;
    let current_keychains = String::from_utf8_lossy(&output.stdout);
    let mut keychains: Vec<String> = current_keychains
        .lines()
        .map(|l| l.trim().trim_matches('"').to_string())
        .filter(|l| !l.is_empty())
        .collect();
    keychains.insert(0, keychain_path.display().to_string());

    let mut cmd = Command::new("security");
    cmd.args(["list-keychains", "-d", "user", "-s"]);
    for kc in &keychains {
        cmd.arg(kc);
    }
    run_command(&mut cmd, "list-keychains -s").await?;

    let mut unlock_cmd = Command::new("security");
    unlock_cmd.args([
        "unlock-keychain",
        "-p",
        keychain_password,
        &keychain_path.display().to_string(),
    ]);
    run_command(&mut unlock_cmd, "unlock-keychain").await?;

    let mut set_partition_cmd = Command::new("security");
    set_partition_cmd.args([
        "set-key-partition-list",
        "-S",
        "apple-tool:,apple:",
        "-s",
        "-k",
        keychain_password,
        &keychain_path.display().to_string(),
    ]);
    run_command(&mut set_partition_cmd, "set-key-partition-list").await?;

    let _ = std::fs::remove_file(&cert_file);

    Ok(TempKeychain {
        path: keychain_path,
        password: keychain_password.to_string(),
    })
}

/// Find the signing identity in a keychain using `security find-identity`.
async fn find_identity_in_keychain(keychain_path: &Path) -> Result<String> {
    let output = Command::new("security")
        .args([
            "find-identity",
            "-v",
            "-p",
            "codesigning",
            &keychain_path.display().to_string(),
        ])
        .output()
        .await
        .context("Failed to run `security find-identity`")?;

    let stdout = String::from_utf8_lossy(&output.stdout);

    for line in stdout.lines() {
        let line = line.trim();
        if line.starts_with("1)")
            || line.contains("Developer ID")
            || line.contains("Apple Development")
        {
            if let Some(start) = line.find('"') {
                if let Some(end) = line.rfind('"') {
                    if end > start {
                        return Ok(line[start + 1..end].to_string());
                    }
                }
            }
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 && parts[1].len() == 40 {
                return Ok(parts[1].to_string());
            }
        }
    }

    bail!(
        "No valid signing identity found in keychain {}.\nOutput: {}",
        keychain_path.display(),
        stdout
    )
}

/// Sign a list of paths with the given identity.
async fn sign_paths(
    identity: &SigningIdentity,
    targets: Vec<SignTarget>,
    settings: &MacOsSettings,
) -> Result<()> {
    for target in &targets {
        sign_path(identity, target, settings).await?;
    }
    Ok(())
}

/// Sign a single path with `codesign`.
async fn sign_path(
    identity: &SigningIdentity,
    target: &SignTarget,
    settings: &MacOsSettings,
) -> Result<()> {
    tracing::info!("Signing: {}", target.path.display());

    let mut cmd = Command::new("codesign");
    cmd.args(["--force", "--sign", &identity.identity]);

    if settings.hardened_runtime {
        cmd.arg("--options");
        cmd.arg("runtime");
    }

    if target
        .path
        .extension()
        .map(|e| e == "app" || e == "framework")
        .unwrap_or(false)
    {
        cmd.arg("--deep");
    }

    if let Some(entitlements) = &settings.entitlements {
        cmd.args(["--entitlements", entitlements]);
    }

    if let Some(keychain) = &identity.temp_keychain {
        cmd.args(["--keychain", &keychain.path.display().to_string()]);
    }

    cmd.arg(&target.path);

    run_command(&mut cmd, &format!("codesign {}", target.path.display())).await?;
    Ok(())
}

/// Notarize a .app or .dmg with Apple's notary service.
async fn notarize(notarize_path: &Path, staple_path: &Path) -> Result<()> {
    let apple_id = std::env::var("APPLE_ID").ok();
    let apple_password = std::env::var("APPLE_PASSWORD").ok();
    let apple_team_id = std::env::var("APPLE_TEAM_ID").ok();
    let api_key = std::env::var("APPLE_API_KEY").ok();
    let api_issuer = std::env::var("APPLE_API_ISSUER").ok();
    let api_key_path = std::env::var("APPLE_API_KEY_PATH").ok();

    let mut cmd = Command::new("xcrun");
    cmd.args(["notarytool", "submit"]);
    cmd.arg(notarize_path);

    if let (Some(key), Some(issuer), Some(key_path)) = (&api_key, &api_issuer, &api_key_path) {
        cmd.args(["--key", key_path]);
        cmd.args(["--key-id", key]);
        cmd.args(["--issuer", issuer]);
    } else if let (Some(id), Some(pwd), Some(team)) = (&apple_id, &apple_password, &apple_team_id) {
        cmd.args(["--apple-id", id]);
        cmd.args(["--password", pwd]);
        cmd.args(["--team-id", team]);
    } else {
        bail!(
            "Notarization requires either:\n\
             - APPLE_ID, APPLE_PASSWORD, and APPLE_TEAM_ID env vars, or\n\
             - APPLE_API_KEY, APPLE_API_ISSUER, and APPLE_API_KEY_PATH env vars"
        );
    }

    cmd.arg("--wait");

    tracing::info!("Submitting {} for notarization...", notarize_path.display());
    run_command(&mut cmd, "xcrun notarytool submit").await?;

    tracing::info!("Stapling notarization ticket...");
    let mut staple_cmd = Command::new("xcrun");
    staple_cmd.args(["stapler", "staple"]).arg(staple_path);
    run_command(&mut staple_cmd, "xcrun stapler staple").await?;

    tracing::info!("Notarization complete for {}", staple_path.display());
    Ok(())
}

/// Helper to run a command and return a nice error on failure.
async fn run_command(cmd: &mut Command, description: &str) -> Result<()> {
    tracing::debug!("Running: {:?}", cmd);
    let status = cmd
        .status()
        .await
        .with_context(|| format!("Failed to execute `{description}`"))?;

    if !status.success() {
        bail!("`{description}` failed with exit code: {}", status);
    }
    Ok(())
}

/// Decode base64 (standard or URL-safe).
async fn base64_decode(input: &str) -> Result<Vec<u8>> {
    let mut child = Command::new("base64")
        .args(["--decode"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("Failed to decode base64 certificate")?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(input.as_bytes())
            .await
            .context("Failed writing base64 certificate to decoder stdin")?;
    }

    let output = child
        .wait_with_output()
        .await
        .context("Failed to decode base64 certificate")?;

    if !output.status.success() {
        bail!(
            "base64 --decode failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    Ok(output.stdout)
}

/// The result of DMG bundling, which may include both the `.dmg` and `.app` outputs.
pub(crate) struct DmgBundled {
    /// Paths to the generated `.dmg` file(s).
    pub dmg: Vec<PathBuf>,
    /// Paths to the generated `.app` bundle(s) (if the `.app` was built as a dependency).
    pub app: Vec<PathBuf>,
}

/// A target to be code-signed.
struct SignTarget {
    path: PathBuf,
}

/// A code signing identity, optionally backed by a temporary keychain.
struct SigningIdentity {
    /// The identity string passed to `codesign --sign`.
    /// This is either a team/certificate name or a SHA-1 hash.
    identity: String,
    /// If we created a temporary keychain to import a certificate,
    /// this holds its path so we can clean it up later.
    temp_keychain: Option<TempKeychain>,
}

/// A temporary keychain created for CI certificate import.
#[allow(dead_code)]
struct TempKeychain {
    path: PathBuf,
    password: String,
}

impl Drop for TempKeychain {
    fn drop(&mut self) {
        tracing::debug!("Cleaning up temporary keychain: {}", self.path.display());
        let _ = StdCommand::new("security")
            .args(["delete-keychain", &self.path.display().to_string()])
            .status();
    }
}
