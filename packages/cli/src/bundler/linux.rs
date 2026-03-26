use crate::{
    bundler::{AppCategory, BundleContext},
    PackageType,
};
use anyhow::{bail, Context, Result};
use handlebars::Handlebars;
use image::{GenericImageView, ImageFormat};
use std::{
    fs::{self, File},
    io::{BufReader, Cursor, Write},
    path::{Path, PathBuf},
};
use tokio::process::Command;

const DEFAULT_LINUX_ICON_PNG: &[u8] = include_bytes!("../../assets/default_icon.png");

impl BundleContext<'_> {
    /// Build a self-contained Linux AppImage using `linuxdeploy`.
    ///
    /// AppImage bundling is implemented as a two-phase process:
    /// 1. Construct an AppDir directory tree that looks like a normal Linux desktop
    ///    installation rooted at `usr/`.
    /// 2. Hand that AppDir to `linuxdeploy`, which turns it into a runnable
    ///    `.AppImage` executable.
    ///
    /// Concretely, this method:
    /// 1. Creates `project_out_directory()/bundle/appimage/<name>.AppDir`.
    /// 2. Reuses the shared Linux payload generator so the AppDir contains the same
    ///    executable, resources, `.desktop` file, icons, and sidecar binaries used by
    ///    other Linux package formats.
    /// 3. Adds the top-level `AppRun`, desktop file, and icon symlinks expected by
    ///    AppImage tooling.
    /// 4. Invokes the pre-resolved `linuxdeploy` binary with `OUTPUT=<target>` so the
    ///    final artifact lands at a deterministic path.
    /// 5. Renames the output if `linuxdeploy` used its own filename convention.
    /// 6. Removes the temporary AppDir after the final image has been created.
    ///
    /// The result is a single `.AppImage` file in
    /// `project_out_directory()/bundle/appimage`.
    pub(crate) async fn bundle_linux_appimage(&self) -> Result<Vec<PathBuf>> {
        let name = self.main_binary_name().to_string();
        let version = self.version_string();
        let arch = self.binary_arch();
        let arch_str = arch.appimage_arch();

        let output_dir = self.project_out_directory().join("appimage");
        fs::create_dir_all(&output_dir)?;

        let appimage_filename = format!("{name}_{version}_{arch_str}.AppImage");
        let appimage_path = output_dir.join(&appimage_filename);

        tracing::info!("Bundling {appimage_filename}...");

        let appdir = output_dir.join(format!("{name}.AppDir"));
        if appdir.exists() {
            fs::remove_dir_all(&appdir)?;
        }
        fs::create_dir_all(&appdir)?;

        self.generate_linux_common_data(&appdir)?;
        self.create_linux_appdir_symlinks(&appdir, &name)?;

        let linuxdeploy = self
            .tools
            .linuxdeploy
            .as_ref()
            .context("linuxdeploy was not resolved. This is a bug.")?;

        tracing::info!("Running linuxdeploy...");

        let output = Command::new(linuxdeploy)
            .arg("--appdir")
            .arg(&appdir)
            .arg("--output")
            .arg("appimage")
            .env("OUTPUT", &appimage_path)
            // Run the linuxdeploy AppImage in extract-and-run mode to avoid host
            // runtime/FUSE differences on distros and CI runners.
            .env("APPIMAGE_EXTRACT_AND_RUN", "1")
            .env("NO_STRIP", "true")
            .current_dir(&output_dir)
            .output()
            .await
            .with_context(|| format!("Failed to run linuxdeploy: {}", linuxdeploy.display()))?;

        if !output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!(
                "linuxdeploy failed with exit code {:?}\nstdout:\n{}\nstderr:\n{}",
                output.status.code(),
                stdout.trim(),
                stderr.trim()
            );
        }

        if !appimage_path.exists() {
            let found = find_appimage_output(&output_dir, &name)?;
            if let Some(found_path) = found {
                if found_path != appimage_path {
                    fs::rename(&found_path, &appimage_path).with_context(|| {
                        format!(
                            "Failed to rename {} to {}",
                            found_path.display(),
                            appimage_path.display()
                        )
                    })?;
                }
            } else {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                bail!(
                    "AppImage was not created. Expected at: {}\nlinuxdeploy stdout:\n{}\nlinuxdeploy stderr:\n{}",
                    appimage_path.display(),
                    stdout.trim(),
                    stderr.trim()
                );
            }
        }

        let _ = fs::remove_dir_all(&appdir);

        tracing::info!("Created AppImage: {}", appimage_path.display());
        Ok(vec![appimage_path])
    }

    /// Build a Debian `.deb` package entirely in Rust.
    ///
    /// The Debian bundler does not shell out to `dpkg-deb`. Instead it assembles the
    /// archive directly from its three standard members:
    /// - `debian-binary`
    /// - `control.tar.gz`
    /// - `data.tar.gz`
    ///
    /// The bundling pipeline is:
    /// 1. Create a temporary `_data` directory containing the Linux install payload.
    ///    This payload places the main executable in `/usr/bin`, resources in
    ///    `/usr/lib/<product-name>`, freedesktop metadata under `/usr/share`, sidecar
    ///    binaries, custom files, and an optional compressed changelog.
    /// 2. Compute the installed size from that payload tree.
    /// 3. Build `control.tar.gz`, including the `control` metadata file, `md5sums`,
    ///    and any maintainer scripts configured in the Debian settings.
    /// 4. Build `data.tar.gz` from the staged payload tree.
    /// 5. Write the final `.deb` as an `ar` archive in the correct member order.
    /// 6. Remove the temporary `_data` directory after assembly completes.
    ///
    /// The final package is emitted to `project_out_directory()/bundle/deb`.
    pub(crate) async fn bundle_linux_deb(&self) -> Result<Vec<PathBuf>> {
        let arch = self.binary_arch().deb_arch();
        let package_name = self.deb_package_name();
        let version = self.version_string();

        let output_dir = self.project_out_directory().join("deb");
        fs::create_dir_all(&output_dir)?;

        let deb_filename = format!("{package_name}_{version}_{arch}.deb");
        let deb_path = output_dir.join(&deb_filename);

        tracing::info!("Bundling {deb_filename}...");

        let data_dir = output_dir.join("_data");
        if data_dir.exists() {
            fs::remove_dir_all(&data_dir)?;
        }
        fs::create_dir_all(&data_dir)?;

        self.generate_linux_common_data(&data_dir)?;
        self.add_linux_deb_data(&data_dir)?;

        let installed_size = dir_size_kb(&data_dir)?;
        let control_tar =
            self.build_linux_control_tar(&package_name, &version, arch, installed_size, &data_dir)?;
        let data_tar = build_data_tar(&data_dir)?;

        let deb_file = File::create(&deb_path)
            .with_context(|| format!("Failed to create {}", deb_path.display()))?;
        let mut ar_builder = ar::Builder::new(deb_file);

        let debian_binary = b"2.0\n";
        let mut header = ar::Header::new(b"debian-binary".to_vec(), debian_binary.len() as u64);
        header.set_mode(0o100644);
        header.set_mtime(0);
        header.set_uid(0);
        header.set_gid(0);
        ar_builder.append(&header, &debian_binary[..])?;

        let mut header = ar::Header::new(b"control.tar.gz".to_vec(), control_tar.len() as u64);
        header.set_mode(0o100644);
        header.set_mtime(0);
        header.set_uid(0);
        header.set_gid(0);
        ar_builder.append(&header, control_tar.as_slice())?;

        let mut header = ar::Header::new(b"data.tar.gz".to_vec(), data_tar.len() as u64);
        header.set_mode(0o100644);
        header.set_mtime(0);
        header.set_uid(0);
        header.set_gid(0);
        ar_builder.append(&header, data_tar.as_slice())?;

        let _ = fs::remove_dir_all(&data_dir);

        tracing::info!("Created deb package: {}", deb_path.display());
        Ok(vec![deb_path])
    }

    /// Build an RPM package using the `rpm` crate.
    ///
    /// RPM bundling mirrors the Linux desktop payload used by the Debian flow, but it
    /// expresses the package through `rpm::PackageBuilder` instead of manually
    /// constructing archive members.
    ///
    /// The method performs these steps:
    /// 1. Initialize the RPM builder with package identity, version, architecture,
    ///    description, and license metadata.
    /// 2. Add the main executable at `/usr/bin/<name>`.
    /// 3. Generate a freedesktop `.desktop` file and add it under
    ///    `/usr/share/applications`.
    /// 4. Add configured icons under the appropriate `hicolor` directories.
    /// 5. Copy resources into a temporary directory, enumerate them, and add each file
    ///    under `/usr/lib/<product-name>/...`.
    /// 6. Copy configured sidecar binaries into `/usr/bin` so RPM payloads match the
    ///    other Linux formats.
    /// 7. Reuse Debian-style custom files and maintainer scripts where applicable.
    /// 8. Attach runtime dependency declarations from the Debian settings.
    /// 9. Serialize the final package to disk and remove the temporary staging area.
    ///
    /// The resulting artifact is written to `project_out_directory()/bundle/rpm`.
    pub(crate) async fn bundle_linux_rpm(&self) -> Result<Vec<PathBuf>> {
        let name = self.main_binary_name().to_string();
        let version = self.version_string();
        let arch = self.binary_arch().rpm_arch();
        let license = self.license().unwrap_or("Unknown").to_string();
        let description = self.short_description();
        let resource_dir_name = self.linux_resource_dir_name();

        let output_dir = self.project_out_directory().join("rpm");
        fs::create_dir_all(&output_dir)?;

        let rpm_filename = format!("{name}-{version}-1.{arch}.rpm");
        let rpm_path = output_dir.join(&rpm_filename);

        tracing::info!("Bundling {rpm_filename}...");

        let mut builder = rpm::PackageBuilder::new(&name, &version, &license, arch, &description)
            .using_config(rpm::BuildConfig::v4().compression(rpm::CompressionType::Gzip));

        let binary_path = self.main_binary_path();
        let dest_bin = format!("/usr/bin/{name}");
        builder = builder
            .with_file(&binary_path, rpm::FileOptions::new(dest_bin).mode(0o755))
            .context("Failed to add binary to RPM")?;

        let deb_settings = self.deb();
        let desktop_content =
            self.generate_linux_desktop_file(deb_settings.desktop_template.as_deref())?;
        let desktop_dest = format!("/usr/share/applications/{name}.desktop");

        let temp_dir = output_dir.join("_rpm_temp");
        fs::create_dir_all(&temp_dir)?;
        let temp_desktop = temp_dir.join(format!("{name}.desktop"));
        fs::write(&temp_desktop, &desktop_content)?;

        builder = builder
            .with_file(
                &temp_desktop,
                rpm::FileOptions::new(desktop_dest).mode(0o644),
            )
            .context("Failed to add desktop file to RPM")?;

        let icon_files = self.linux_icon_files_or_default(&temp_dir)?;
        for icon_path in &icon_files {
            let ext = icon_path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_lowercase();

            match ext.as_str() {
                "png" => {
                    if let Ok(file) = fs::File::open(icon_path) {
                        let decoder = png::Decoder::new(BufReader::new(file));
                        if let Ok(reader) = decoder.read_info() {
                            let info = reader.info();
                            let (w, h) = (info.width, info.height);
                            let size = w.max(h);
                            let dest =
                                format!("/usr/share/icons/hicolor/{size}x{size}/apps/{name}.png");
                            builder = builder
                                .with_file(icon_path, rpm::FileOptions::new(dest).mode(0o644))
                                .context("Failed to add icon to RPM")?;
                        }
                    }
                }
                "svg" => {
                    let dest = format!("/usr/share/icons/hicolor/scalable/apps/{name}.svg");
                    builder = builder
                        .with_file(icon_path, rpm::FileOptions::new(dest).mode(0o644))
                        .context("Failed to add SVG icon to RPM")?;
                }
                "ico" => {
                    let img = image::open(icon_path).with_context(|| {
                        format!("Failed to decode ICO icon: {}", icon_path.display())
                    })?;
                    let (width, height) = img.dimensions();
                    let size = width.max(height);

                    let converted_icon = temp_dir.join(format!("{name}-{size}.png"));
                    img.save_with_format(&converted_icon, ImageFormat::Png)
                        .with_context(|| {
                            format!(
                                "Failed to convert ICO icon {} -> {}",
                                icon_path.display(),
                                converted_icon.display()
                            )
                        })?;

                    let dest = format!("/usr/share/icons/hicolor/{size}x{size}/apps/{name}.png");
                    builder = builder
                        .with_file(&converted_icon, rpm::FileOptions::new(dest).mode(0o644))
                        .context("Failed to add converted ICO icon to RPM")?;
                }
                _ => {
                    tracing::warn!(
                        "Skipping icon with unsupported extension '{}': {}",
                        ext,
                        icon_path.display()
                    );
                }
            }
        }

        let resource_temp = temp_dir.join("resources");
        fs::create_dir_all(&resource_temp)?;
        self.copy_resources(&resource_temp)?;

        let resource_files = collect_files(&resource_temp)?;
        for (src, relative) in &resource_files {
            let dest = format!(
                "/usr/lib/{resource_dir_name}/{}",
                relative.to_string_lossy().replace('\\', "/")
            );
            builder = builder
                .with_file(src, rpm::FileOptions::new(&dest).mode(0o644))
                .with_context(|| format!("Failed to add resource {} to RPM", relative.display()))?;
        }

        let ext_bin_temp = temp_dir.join("external_bin");
        fs::create_dir_all(&ext_bin_temp)?;
        let external_bins = self.copy_external_binaries(&ext_bin_temp)?;
        for src in &external_bins {
            let dest_name = src
                .file_name()
                .and_then(|name| name.to_str())
                .context("External binary is missing a file name")?;
            let dest = format!("/usr/bin/{dest_name}");
            builder = builder
                .with_file(src, rpm::FileOptions::new(&dest).mode(0o755))
                .with_context(|| format!("Failed to add external binary {dest_name} to RPM"))?;
        }

        let crate_dir = self.crate_dir();
        for (dest_path, src_path) in &deb_settings.files {
            let src = if src_path.is_absolute() {
                src_path.clone()
            } else {
                crate_dir.join(src_path)
            };
            if src.exists() {
                let dest = dest_path.to_string_lossy().to_string();
                let dest = if dest.starts_with('/') {
                    dest
                } else {
                    format!("/{dest}")
                };
                builder = builder
                    .with_file(&src, rpm::FileOptions::new(&dest).mode(0o644))
                    .context("Failed to add custom file to RPM")?;
            }
        }

        if let Some(script_path) = &deb_settings.pre_install_script {
            let path = resolve_path(&crate_dir, script_path);
            let content = fs::read_to_string(&path).with_context(|| {
                format!("Failed to read pre-install script: {}", path.display())
            })?;
            builder = builder.pre_install_script(content);
        }
        if let Some(script_path) = &deb_settings.post_install_script {
            let path = resolve_path(&crate_dir, script_path);
            let content = fs::read_to_string(&path).with_context(|| {
                format!("Failed to read post-install script: {}", path.display())
            })?;
            builder = builder.post_install_script(content);
        }
        if let Some(script_path) = &deb_settings.pre_remove_script {
            let path = resolve_path(&crate_dir, script_path);
            let content = fs::read_to_string(&path).with_context(|| {
                format!("Failed to read pre-uninstall script: {}", path.display())
            })?;
            builder = builder.pre_uninstall_script(content);
        }
        if let Some(script_path) = &deb_settings.post_remove_script {
            let path = resolve_path(&crate_dir, script_path);
            let content = fs::read_to_string(&path).with_context(|| {
                format!("Failed to read post-uninstall script: {}", path.display())
            })?;
            builder = builder.post_uninstall_script(content);
        }

        if let Some(deps) = &deb_settings.depends {
            for dep in deps {
                builder = builder.requires(rpm::Dependency::any(dep));
            }
        }

        let package = builder.build().context("Failed to build RPM package")?;

        let mut rpm_file = fs::File::create(&rpm_path)
            .with_context(|| format!("Failed to create {}", rpm_path.display()))?;
        package
            .write(&mut rpm_file)
            .context("Failed to write RPM package")?;

        let _ = fs::remove_dir_all(&temp_dir);

        tracing::info!("Created RPM package: {}", rpm_path.display());
        Ok(vec![rpm_path])
    }

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

    /// Generate the Linux payload shared across bundle formats.
    fn generate_linux_common_data(&self, data_dir: &Path) -> Result<()> {
        let bin_name = self.main_binary_name();
        let resource_dir_name = self.linux_resource_dir_name();

        let bin_dir = data_dir.join("usr/bin");
        fs::create_dir_all(&bin_dir)?;
        let bin_dest = bin_dir.join(bin_name);
        fs::copy(self.main_binary_path(), &bin_dest)
            .with_context(|| format!("Failed to copy binary to {}", bin_dest.display()))?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&bin_dest, fs::Permissions::from_mode(0o755))?;
        }

        let desktop_dir = data_dir.join("usr/share/applications");
        fs::create_dir_all(&desktop_dir)?;

        let deb_settings = self.deb();
        let desktop_content =
            self.generate_linux_desktop_file(deb_settings.desktop_template.as_deref())?;
        let desktop_path = desktop_dir.join(format!("{bin_name}.desktop"));
        fs::write(&desktop_path, &desktop_content)?;

        self.copy_linux_icons(data_dir)?;

        let resource_dir = data_dir.join(format!("usr/lib/{resource_dir_name}"));
        fs::create_dir_all(&resource_dir)?;
        self.copy_resources(&resource_dir)?;

        let ext_bin_dir = data_dir.join("usr/bin");
        self.copy_external_binaries(&ext_bin_dir)?;

        Ok(())
    }

    /// Add Debian-specific payload extras to a staged Linux tree.
    fn add_linux_deb_data(&self, data_dir: &Path) -> Result<()> {
        let deb_settings = self.deb();
        let package_name = self.deb_package_name();
        for (deb_path, src_path) in &deb_settings.files {
            let dest = data_dir.join(deb_path.strip_prefix("/").unwrap_or(deb_path));
            if let Some(parent) = dest.parent() {
                fs::create_dir_all(parent)?;
            }
            let src = if src_path.is_absolute() {
                src_path.clone()
            } else {
                self.crate_dir().join(src_path)
            };
            fs::copy(&src, &dest).with_context(|| {
                format!(
                    "Failed to copy custom deb file {} -> {}",
                    src.display(),
                    dest.display()
                )
            })?;
        }

        if let Some(changelog_path) = &deb_settings.changelog {
            let changelog_src = if changelog_path.is_absolute() {
                changelog_path.clone()
            } else {
                self.crate_dir().join(changelog_path)
            };
            if changelog_src.exists() {
                let doc_dir = data_dir.join(format!("usr/share/doc/{package_name}"));
                fs::create_dir_all(&doc_dir)?;
                let changelog_content = fs::read(&changelog_src)?;
                let mut encoder =
                    flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::best());
                encoder.write_all(&changelog_content)?;
                let compressed = encoder.finish()?;
                fs::write(doc_dir.join("changelog.gz"), compressed)?;
            }
        }

        Ok(())
    }

    /// Directory name used for bundled Linux resources.
    fn linux_resource_dir_name(&self) -> String {
        self.product_name()
    }

    /// Generate the contents of a .desktop file for the given bundle context.
    fn generate_linux_desktop_file(&self, desktop_template: Option<&Path>) -> Result<String> {
        let mut handlebars = Handlebars::new();
        handlebars.set_strict_mode(false);

        let template = match desktop_template {
            // Path to template
            Some(path) => fs::read_to_string(path)
                .with_context(|| format!("Failed to read desktop template: {}", path.display()))?,

            // Default .desktop file template (Handlebars).
            None => String::from(
                "[Desktop Entry]
Categories={{categories}}
{{#if comment}}
Comment={{comment}}
{{/if}}
Exec={{exec}}
Icon={{icon}}
Name={{name}}
Terminal=false
Type=Application
",
            ),
        };

        handlebars
            .register_template_string("desktop", &template)
            .context("Failed to register desktop template")?;

        let categories = self
            .app_category()
            .and_then(|c| c.parse::<AppCategory>().ok())
            .map(|cat| cat.freedesktop_categories().to_string())
            .unwrap_or_default();

        let bin_name = self.main_binary_name();
        let product_name = self.product_name();
        let description = self.short_description();
        let has_comment = !description.is_empty();

        let mut json_data = serde_json::Map::new();
        json_data.insert("categories".into(), serde_json::Value::String(categories));
        json_data.insert(
            "exec".into(),
            serde_json::Value::String(bin_name.to_string()),
        );
        json_data.insert(
            "icon".into(),
            serde_json::Value::String(bin_name.to_string()),
        );
        json_data.insert("name".into(), serde_json::Value::String(product_name));
        if has_comment {
            json_data.insert("comment".into(), serde_json::Value::String(description));
        }

        let rendered = handlebars
            .render("desktop", &json_data)
            .context("Failed to render desktop template")?;

        Ok(rendered)
    }

    /// Copy icon files into the freedesktop hicolor icon theme hierarchy.
    fn copy_linux_icons(&self, data_dir: &Path) -> Result<Vec<PathBuf>> {
        let icon_files = self.linux_icon_files_or_default(data_dir)?;
        let bin_name = self.main_binary_name();
        let mut paths = Vec::new();

        for icon_path in &icon_files {
            let ext = icon_path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_lowercase();

            match ext.as_str() {
                "png" => {
                    let file = fs::File::open(icon_path).with_context(|| {
                        format!("Failed to open PNG icon: {}", icon_path.display())
                    })?;
                    let decoder = png::Decoder::new(BufReader::new(file));
                    let reader = decoder.read_info().with_context(|| {
                        format!("Failed to decode PNG dimensions: {}", icon_path.display())
                    })?;
                    let info = reader.info();
                    let (width, height) = (info.width, info.height);
                    let size = width.max(height);

                    let dest_dir =
                        data_dir.join(format!("usr/share/icons/hicolor/{size}x{size}/apps"));
                    fs::create_dir_all(&dest_dir)?;

                    let dest = dest_dir.join(format!("{bin_name}.png"));
                    fs::copy(icon_path, &dest).with_context(|| {
                        format!(
                            "Failed to copy icon {} -> {}",
                            icon_path.display(),
                            dest.display()
                        )
                    })?;

                    tracing::debug!("Copied icon {}x{}: {}", size, size, dest.display());
                    paths.push(dest);
                }
                "svg" => {
                    let dest_dir = data_dir.join("usr/share/icons/hicolor/scalable/apps");
                    fs::create_dir_all(&dest_dir)?;

                    let dest = dest_dir.join(format!("{bin_name}.svg"));
                    fs::copy(icon_path, &dest).with_context(|| {
                        format!(
                            "Failed to copy icon {} -> {}",
                            icon_path.display(),
                            dest.display()
                        )
                    })?;

                    tracing::debug!("Copied SVG icon: {}", dest.display());
                    paths.push(dest);
                }
                "ico" => {
                    let img = image::open(icon_path).with_context(|| {
                        format!("Failed to decode ICO icon: {}", icon_path.display())
                    })?;
                    let (width, height) = img.dimensions();
                    let size = width.max(height);

                    let dest_dir =
                        data_dir.join(format!("usr/share/icons/hicolor/{size}x{size}/apps"));
                    fs::create_dir_all(&dest_dir)?;

                    let dest = dest_dir.join(format!("{bin_name}.png"));
                    img.save_with_format(&dest, ImageFormat::Png)
                        .with_context(|| {
                            format!(
                                "Failed to convert ICO icon {} -> {}",
                                icon_path.display(),
                                dest.display()
                            )
                        })?;

                    tracing::debug!(
                        "Converted ICO icon {}x{} to PNG: {}",
                        width,
                        height,
                        dest.display()
                    );
                    paths.push(dest);
                }
                _ => {
                    tracing::warn!(
                        "Skipping icon with unsupported extension '{}': {}",
                        ext,
                        icon_path.display()
                    );
                }
            }
        }

        Ok(paths)
    }

    fn linux_icon_files_or_default(&self, scratch_dir: &Path) -> Result<Vec<PathBuf>> {
        let icon_files = self.icon_files()?;
        if !icon_files.is_empty() {
            return Ok(icon_files);
        }

        fs::create_dir_all(scratch_dir)?;
        let default_icon = scratch_dir.join(".dx-default-icon.png");
        if !default_icon.exists() {
            fs::write(&default_icon, DEFAULT_LINUX_ICON_PNG).with_context(|| {
                format!(
                    "Failed to write default Linux icon to {}",
                    default_icon.display()
                )
            })?;
        }

        tracing::info!("No bundle icons configured; using the default Dioxus icon");
        Ok(vec![default_icon])
    }

    /// Create the top-level symlinks in the AppDir that AppImage/linuxdeploy expects.
    fn create_linux_appdir_symlinks(&self, appdir: &Path, name: &str) -> Result<()> {
        let apprun = appdir.join("AppRun");
        let bin_target = format!("usr/bin/{name}");

        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(&bin_target, &apprun).with_context(|| {
                format!(
                    "Failed to create AppRun symlink: {} -> {}",
                    apprun.display(),
                    bin_target
                )
            })?;
        }

        #[cfg(not(unix))]
        {
            let src = appdir.join(&bin_target);
            if src.exists() {
                fs::copy(&src, &apprun)?;
            }
        }

        let desktop_link = appdir.join(format!("{name}.desktop"));
        let desktop_target = format!("usr/share/applications/{name}.desktop");

        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(&desktop_target, &desktop_link).with_context(|| {
                format!(
                    "Failed to create desktop symlink: {} -> {}",
                    desktop_link.display(),
                    desktop_target
                )
            })?;
        }

        #[cfg(not(unix))]
        {
            let src = appdir.join(&desktop_target);
            if src.exists() {
                fs::copy(&src, &desktop_link)?;
            }
        }

        if let Some(icon_in_appdir) = find_icon_in_appdir(appdir, name) {
            create_appdir_icon_links(appdir, &icon_in_appdir, name)?;
        }

        Ok(())
    }

    /// Build the control.tar.gz containing the control file, md5sums, and maintainer scripts.
    fn build_linux_control_tar(
        &self,
        package_name: &str,
        version: &str,
        arch: &str,
        installed_size: u64,
        data_dir: &Path,
    ) -> Result<Vec<u8>> {
        let buf = Vec::new();
        let encoder = flate2::write::GzEncoder::new(buf, flate2::Compression::default());
        let mut tar = tar::Builder::new(encoder);

        let control =
            self.generate_linux_control_file(package_name, version, arch, installed_size)?;
        append_tar_bytes(&mut tar, "./control", control.as_bytes(), 0o644)?;

        let md5sums = generate_md5sums(data_dir)?;
        append_tar_bytes(&mut tar, "./md5sums", md5sums.as_bytes(), 0o644)?;

        let deb = self.deb();
        let crate_dir = self.crate_dir();

        if let Some(script_path) = &deb.pre_install_script {
            let path = resolve_path(&crate_dir, script_path);
            let content = fs::read(&path)
                .with_context(|| format!("Failed to read preinst script: {}", path.display()))?;
            append_tar_bytes(&mut tar, "./preinst", &content, 0o755)?;
        }
        if let Some(script_path) = &deb.post_install_script {
            let path = resolve_path(&crate_dir, script_path);
            let content = fs::read(&path)
                .with_context(|| format!("Failed to read postinst script: {}", path.display()))?;
            append_tar_bytes(&mut tar, "./postinst", &content, 0o755)?;
        }
        if let Some(script_path) = &deb.pre_remove_script {
            let path = resolve_path(&crate_dir, script_path);
            let content = fs::read(&path)
                .with_context(|| format!("Failed to read prerm script: {}", path.display()))?;
            append_tar_bytes(&mut tar, "./prerm", &content, 0o755)?;
        }
        if let Some(script_path) = &deb.post_remove_script {
            let path = resolve_path(&crate_dir, script_path);
            let content = fs::read(&path)
                .with_context(|| format!("Failed to read postrm script: {}", path.display()))?;
            append_tar_bytes(&mut tar, "./postrm", &content, 0o755)?;
        }

        let encoder = tar.into_inner()?;
        let data = encoder.finish()?;
        Ok(data)
    }

    /// Generate the Debian control file content.
    fn generate_linux_control_file(
        &self,
        package_name: &str,
        version: &str,
        arch: &str,
        installed_size: u64,
    ) -> Result<String> {
        let deb = self.deb();

        let mut control = String::new();
        control.push_str(&format!("Package: {package_name}\n"));
        control.push_str(&format!("Version: {version}\n"));
        control.push_str(&format!("Architecture: {arch}\n"));
        control.push_str(&format!("Installed-Size: {installed_size}\n"));

        let description = self.short_description();
        // Description is a required field in Debian control files - use the product name as fallback
        let description = if description.is_empty() {
            self.product_name()
        } else {
            description
        };
        control.push_str(&format!("Description: {description}\n"));

        if let Some(long_desc) = self.long_description() {
            for line in long_desc.lines() {
                if line.is_empty() {
                    control.push_str(" .\n");
                } else {
                    control.push_str(&format!(" {line}\n"));
                }
            }
        }

        let section = deb.section.as_deref().unwrap_or("utils");
        control.push_str(&format!("Section: {section}\n"));

        let priority = deb.priority.as_deref().unwrap_or("optional");
        control.push_str(&format!("Priority: {priority}\n"));

        if let Some(url) = self.homepage_url() {
            control.push_str(&format!("Homepage: {url}\n"));
        }

        let maintainer = self
            .authors_comma_separated()
            .unwrap_or_else(|| "Unknown".to_string());
        control.push_str(&format!("Maintainer: {maintainer}\n"));

        if let Some(deps) = &deb.depends {
            if !deps.is_empty() {
                control.push_str(&format!("Depends: {}\n", deps.join(", ")));
            }
        }

        if let Some(recs) = &deb.recommends {
            if !recs.is_empty() {
                control.push_str(&format!("Recommends: {}\n", recs.join(", ")));
            }
        }

        if let Some(provs) = &deb.provides {
            if !provs.is_empty() {
                control.push_str(&format!("Provides: {}\n", provs.join(", ")));
            }
        }

        if let Some(conflicts) = &deb.conflicts {
            if !conflicts.is_empty() {
                control.push_str(&format!("Conflicts: {}\n", conflicts.join(", ")));
            }
        }

        if let Some(replaces) = &deb.replaces {
            if !replaces.is_empty() {
                control.push_str(&format!("Replaces: {}\n", replaces.join(", ")));
            }
        }

        Ok(control)
    }

    /// Generate a Debian-friendly package name.
    fn deb_package_name(&self) -> String {
        self.main_binary_name().to_lowercase().replace('_', "-")
    }
}

/// Find the best icon file within the AppDir's hicolor directory.
fn find_icon_in_appdir(appdir: &Path, name: &str) -> Option<PathBuf> {
    let icons_dir = appdir.join("usr/share/icons/hicolor");
    if !icons_dir.exists() {
        return None;
    }

    let mut best_png: Option<(u32, PathBuf)> = None;
    let mut svg_icon: Option<PathBuf> = None;
    let png_name = format!("{name}.png");
    let svg_name = format!("{name}.svg");

    if let Ok(entries) = fs::read_dir(&icons_dir) {
        for entry in entries.flatten() {
            let size_dir = entry.path();
            let png_icon = size_dir.join("apps").join(&png_name);
            if png_icon.exists() {
                let dir_name = entry.file_name().to_string_lossy().to_string();
                if let Some(size_str) = dir_name.split('x').next() {
                    if let Ok(size) = size_str.parse::<u32>() {
                        if best_png
                            .as_ref()
                            .is_none_or(|(best_size, _)| size > *best_size)
                        {
                            best_png = Some((size, png_icon));
                        }
                    }
                }
            }

            let svg_path = size_dir.join("apps").join(&svg_name);
            if svg_icon.is_none() && svg_path.exists() {
                svg_icon = Some(svg_path);
            }
        }
    }

    best_png.map(|(_, path)| path).or(svg_icon)
}

/// Create the AppDir root icon links that linuxdeploy expects.
fn create_appdir_icon_links(appdir: &Path, icon_path: &Path, name: &str) -> Result<()> {
    let ext = icon_path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("png");
    let named_icon = appdir.join(format!("{name}.{ext}"));
    let dir_icon = appdir.join(".DirIcon");

    #[cfg(unix)]
    {
        let relative = icon_path.strip_prefix(appdir).unwrap_or(icon_path);
        let relative_str = relative.to_string_lossy().to_string();
        std::os::unix::fs::symlink(&relative_str, &named_icon).with_context(|| {
            format!(
                "Failed to create icon symlink: {} -> {}",
                named_icon.display(),
                relative_str
            )
        })?;
        std::os::unix::fs::symlink(&relative_str, &dir_icon).with_context(|| {
            format!(
                "Failed to create .DirIcon symlink: {} -> {}",
                dir_icon.display(),
                relative_str
            )
        })?;
    }

    #[cfg(not(unix))]
    {
        fs::copy(icon_path, &named_icon).with_context(|| {
            format!(
                "Failed to copy icon {} to {}",
                icon_path.display(),
                named_icon.display()
            )
        })?;
        fs::copy(icon_path, &dir_icon).with_context(|| {
            format!(
                "Failed to copy icon {} to {}",
                icon_path.display(),
                dir_icon.display()
            )
        })?;
    }

    Ok(())
}

/// Search for an .AppImage file in the output directory.
fn find_appimage_output(dir: &Path, name: &str) -> Result<Option<PathBuf>> {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(file_name) = path.file_name().and_then(|f| f.to_str()) {
                if file_name.ends_with(".AppImage") && file_name.contains(name) {
                    return Ok(Some(path));
                }
            }
        }
    }
    Ok(None)
}

/// Build data.tar.gz from the data directory.
fn build_data_tar(data_dir: &Path) -> Result<Vec<u8>> {
    let buf = Vec::new();
    let encoder = flate2::write::GzEncoder::new(buf, flate2::Compression::default());
    let mut tar = tar::Builder::new(encoder);

    tar.append_dir_all(".", data_dir)
        .context("Failed to build data.tar.gz")?;

    let encoder = tar.into_inner()?;
    let data = encoder.finish()?;
    Ok(data)
}

/// Generate md5sums file for all files in the data directory.
fn generate_md5sums(data_dir: &Path) -> Result<String> {
    let mut md5sums = String::new();

    for entry in walkdir::WalkDir::new(data_dir) {
        let entry = entry?;
        if !entry.file_type().is_file() {
            continue;
        }

        let path = entry.path();
        let content = fs::read(path)?;
        let digest = md5::compute(&content);
        let relative = path.strip_prefix(data_dir).unwrap_or(path);
        let relative_str = relative.to_string_lossy().replace('\\', "/");

        md5sums.push_str(&format!("{:x}  {relative_str}\n", digest));
    }

    Ok(md5sums)
}

/// Append raw bytes as a file entry in a tar archive.
fn append_tar_bytes<W: Write>(
    tar: &mut tar::Builder<W>,
    path: &str,
    data: &[u8],
    mode: u32,
) -> Result<()> {
    let mut header = tar::Header::new_gnu();
    header.set_path(path)?;
    header.set_size(data.len() as u64);
    header.set_mode(mode);
    header.set_mtime(0);
    header.set_uid(0);
    header.set_gid(0);
    header.set_cksum();

    tar.append(&header, Cursor::new(data))
        .with_context(|| format!("Failed to add {path} to tar"))?;

    Ok(())
}

/// Collect all files in a directory, returning (absolute_path, relative_path) pairs.
fn collect_files(dir: &Path) -> Result<Vec<(PathBuf, PathBuf)>> {
    let mut files = Vec::new();
    if !dir.exists() {
        return Ok(files);
    }
    for entry in walkdir::WalkDir::new(dir) {
        let entry = entry?;
        if !entry.file_type().is_file() {
            continue;
        }
        let abs = entry.path().to_path_buf();
        let rel = entry
            .path()
            .strip_prefix(dir)
            .unwrap_or(entry.path())
            .to_path_buf();
        files.push((abs, rel));
    }
    Ok(files)
}

/// Calculate total size of a directory tree in kilobytes.
fn dir_size_kb(path: &Path) -> Result<u64> {
    let mut total: u64 = 0;
    for entry in walkdir::WalkDir::new(path) {
        let entry = entry?;
        if entry.file_type().is_file() {
            total += entry.metadata()?.len();
        }
    }

    Ok(total.div_ceil(1024))
}

/// Resolve a path that may be relative to the crate directory.
fn resolve_path(crate_dir: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        crate_dir.join(path)
    }
}
