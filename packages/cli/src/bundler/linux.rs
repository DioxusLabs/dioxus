use crate::bundler::{category::AppCategory, context::Arch, BundleContext};
use anyhow::{bail, Context, Result};
use handlebars::Handlebars;
use std::{
    fs::{self, File},
    io::{BufReader, Cursor, Write},
    path::{Path, PathBuf},
};
use tokio::process::Command;

/// Default .desktop file template (Handlebars).
const DEFAULT_DESKTOP_TEMPLATE: &str = "[Desktop Entry]
Categories={{categories}}
{{#if comment}}
Comment={{comment}}
{{/if}}
Exec={{exec}}
Icon={{icon}}
Name={{name}}
Terminal=false
Type=Application
";

impl BundleContext<'_> {
    /// Bundle the project as an AppImage.
    pub(crate) async fn bundle_linux_appimage(&self) -> Result<Vec<PathBuf>> {
        let name = self.main_binary_name().to_string();
        let version = self.version_string();
        let arch = self.binary_arch();
        let arch_str = appimage_arch(arch);

        let output_dir = self.project_out_directory().join("bundle").join("appimage");
        fs::create_dir_all(&output_dir)?;

        let appimage_filename = format!("{name}_{version}_{arch_str}.AppImage");
        let appimage_path = output_dir.join(&appimage_filename);

        tracing::info!("Bundling {appimage_filename}...");

        let appdir = output_dir.join(format!("{name}.AppDir"));
        if appdir.exists() {
            fs::remove_dir_all(&appdir)?;
        }
        fs::create_dir_all(&appdir)?;

        self.generate_linux_data(&appdir)?;
        self.create_linux_appdir_symlinks(&appdir, &name)?;

        let linuxdeploy = self
            .tools
            .linuxdeploy
            .as_ref()
            .context("linuxdeploy was not resolved. This is a bug.")?;

        tracing::info!("Running linuxdeploy...");

        let status = Command::new(linuxdeploy)
            .arg("--appdir")
            .arg(&appdir)
            .arg("--output")
            .arg("appimage")
            .env("OUTPUT", &appimage_path)
            .env("NO_STRIP", "true")
            .current_dir(&output_dir)
            .status()
            .await
            .with_context(|| format!("Failed to run linuxdeploy: {}", linuxdeploy.display()))?;

        if !status.success() {
            bail!(
                "linuxdeploy failed with exit code: {}",
                status.code().unwrap_or(-1)
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
                bail!(
                    "AppImage was not created. Expected at: {}",
                    appimage_path.display()
                );
            }
        }

        let _ = fs::remove_dir_all(&appdir);

        tracing::info!("Created AppImage: {}", appimage_path.display());
        Ok(vec![appimage_path])
    }

    /// Bundle the project as a .deb package.
    pub(crate) async fn bundle_linux_deb(&self) -> Result<Vec<PathBuf>> {
        let arch = deb_arch(self.binary_arch());
        let package_name = self.deb_package_name();
        let version = self.version_string();

        let output_dir = self.project_out_directory().join("bundle").join("deb");
        fs::create_dir_all(&output_dir)?;

        let deb_filename = format!("{package_name}_{version}_{arch}.deb");
        let deb_path = output_dir.join(&deb_filename);

        tracing::info!("Bundling {deb_filename}...");

        let data_dir = output_dir.join("_data");
        if data_dir.exists() {
            fs::remove_dir_all(&data_dir)?;
        }
        fs::create_dir_all(&data_dir)?;

        self.generate_linux_data(&data_dir)?;

        let installed_size = dir_size_kb(&data_dir)?;
        let control_tar = self.build_linux_control_tar(
            &package_name,
            &version,
            arch,
            installed_size,
            &data_dir,
        )?;
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

    /// Bundle the project as an RPM package.
    pub(crate) async fn bundle_linux_rpm(&self) -> Result<Vec<PathBuf>> {
        let name = self.main_binary_name().to_string();
        let version = self.version_string();
        let arch = rpm_arch(self.binary_arch());
        let license = self.license().unwrap_or("Unknown").to_string();
        let description = self.short_description();

        let output_dir = self.project_out_directory().join("bundle").join("rpm");
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

        let icon_files = self.icon_files()?;
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
                "/usr/lib/{name}/{}",
                relative.to_string_lossy().replace('\\', "/")
            );
            builder = builder
                .with_file(src, rpm::FileOptions::new(&dest).mode(0o644))
                .with_context(|| format!("Failed to add resource {} to RPM", relative.display()))?;
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
            let content = fs::read_to_string(&path)
                .with_context(|| format!("Failed to read pre-install script: {}", path.display()))?;
            builder = builder.pre_install_script(content);
        }
        if let Some(script_path) = &deb_settings.post_install_script {
            let path = resolve_path(&crate_dir, script_path);
            let content = fs::read_to_string(&path)
                .with_context(|| format!("Failed to read post-install script: {}", path.display()))?;
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

    /// Generate the data directory tree for the Linux package payload.
    fn generate_linux_data(&self, data_dir: &Path) -> Result<()> {
        let bin_name = self.main_binary_name();

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

        let resource_dir = data_dir.join(format!("usr/lib/{bin_name}"));
        fs::create_dir_all(&resource_dir)?;
        self.copy_resources(&resource_dir)?;

        let ext_bin_dir = data_dir.join("usr/bin");
        self.copy_external_binaries(&ext_bin_dir)?;

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
                let doc_dir = data_dir.join(format!("usr/share/doc/{bin_name}"));
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

    /// Generate the contents of a .desktop file for the given bundle context.
    fn generate_linux_desktop_file(&self, desktop_template: Option<&Path>) -> Result<String> {
        let mut handlebars = Handlebars::new();
        handlebars.set_strict_mode(false);

        let template = if let Some(path) = desktop_template {
            fs::read_to_string(path)
                .with_context(|| format!("Failed to read desktop template: {}", path.display()))?
        } else {
            DEFAULT_DESKTOP_TEMPLATE.to_string()
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
        json_data.insert("exec".into(), serde_json::Value::String(bin_name.to_string()));
        json_data.insert("icon".into(), serde_json::Value::String(bin_name.to_string()));
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
        let icon_files = self.icon_files()?;
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
                    let file = fs::File::open(icon_path)
                        .with_context(|| format!("Failed to open PNG icon: {}", icon_path.display()))?;
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

    /// Find the path to the largest PNG icon from the configured icon files.
    fn find_linux_largest_icon(&self) -> Result<Option<PathBuf>> {
        let icon_files = self.icon_files()?;
        let mut best: Option<(u32, PathBuf)> = None;

        for icon_path in icon_files {
            let ext = icon_path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_lowercase();

            if ext == "png" {
                if let Ok(file) = fs::File::open(&icon_path) {
                    let decoder = png::Decoder::new(BufReader::new(file));
                    if let Ok(reader) = decoder.read_info() {
                        let info = reader.info();
                        let (w, h) = (info.width, info.height);
                        let size = w.max(h);
                        if best
                            .as_ref()
                            .is_none_or(|(best_size, _)| size > *best_size)
                        {
                            best = Some((size, icon_path));
                        }
                    }
                }
            }
        }

        Ok(best.map(|(_, path)| path))
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

        if let Some(largest_icon) = self.find_linux_largest_icon()? {
            let icon_link = appdir.join(format!("{name}.png"));

            if let Some(icon_in_appdir) = find_icon_in_appdir(appdir, name) {
                let relative = icon_in_appdir
                    .strip_prefix(appdir)
                    .unwrap_or(&icon_in_appdir);
                let relative_str = relative.to_string_lossy().to_string();

                #[cfg(unix)]
                std::os::unix::fs::symlink(&relative_str, &icon_link).with_context(|| {
                    format!(
                        "Failed to create icon symlink: {} -> {}",
                        icon_link.display(),
                        relative_str
                    )
                })?;

                #[cfg(not(unix))]
                fs::copy(&icon_in_appdir, &icon_link)?;
            } else {
                fs::copy(&largest_icon, &icon_link).with_context(|| {
                    format!("Failed to copy icon {} to AppDir", largest_icon.display())
                })?;
            }
        }

        Ok(())
    }

    /// Generate a Debian-friendly package name.
    fn deb_package_name(&self) -> String {
        self.main_binary_name().to_lowercase().replace('_', "-")
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
        if !description.is_empty() {
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
}

/// Find the icon file within the AppDir's hicolor directory.
fn find_icon_in_appdir(appdir: &Path, name: &str) -> Option<PathBuf> {
    let icons_dir = appdir.join("usr/share/icons/hicolor");
    if !icons_dir.exists() {
        return None;
    }

    let mut best: Option<(u32, PathBuf)> = None;
    let target_name = format!("{name}.png");

    if let Ok(entries) = fs::read_dir(&icons_dir) {
        for entry in entries.flatten() {
            let size_dir = entry.path();
            let icon_path = size_dir.join("apps").join(&target_name);
            if icon_path.exists() {
                let dir_name = entry.file_name().to_string_lossy().to_string();
                if let Some(size_str) = dir_name.split('x').next() {
                    if let Ok(size) = size_str.parse::<u32>() {
                        if best
                            .as_ref()
                            .is_none_or(|(best_size, _)| size > *best_size)
                        {
                            best = Some((size, icon_path));
                        }
                    }
                }
            }
        }
    }

    best.map(|(_, path)| path)
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

/// Map Arch enum to Debian architecture string.
fn deb_arch(arch: Arch) -> &'static str {
    match arch {
        Arch::X86_64 => "amd64",
        Arch::X86 => "i386",
        Arch::AArch64 => "arm64",
        Arch::Armhf => "armhf",
        Arch::Armel => "armel",
        Arch::Riscv64 => "riscv64",
        Arch::Universal => "all",
    }
}

/// Map Arch to RPM architecture string.
fn rpm_arch(arch: Arch) -> &'static str {
    match arch {
        Arch::X86_64 => "x86_64",
        Arch::X86 => "i686",
        Arch::AArch64 => "aarch64",
        Arch::Armhf => "armv7hl",
        Arch::Armel => "armv6l",
        Arch::Riscv64 => "riscv64",
        Arch::Universal => "noarch",
    }
}

/// Map Arch to the architecture string used in AppImage filenames.
fn appimage_arch(arch: Arch) -> &'static str {
    match arch {
        Arch::X86_64 => "x86_64",
        Arch::X86 => "i386",
        Arch::AArch64 => "aarch64",
        Arch::Armhf => "armhf",
        Arch::Armel => "armel",
        Arch::Riscv64 => "riscv64",
        Arch::Universal => "x86_64",
    }
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
    Ok((total + 1023) / 1024)
}

/// Resolve a path that may be relative to the crate directory.
fn resolve_path(crate_dir: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        crate_dir.join(path)
    }
}
