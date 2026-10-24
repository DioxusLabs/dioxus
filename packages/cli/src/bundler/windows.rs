use crate::bundler::BundleContext;
use crate::{NSISInstallerMode, WebviewInstallMode, WindowsSettings};
use anyhow::{Context, Result, bail};
use handlebars::Handlebars;
use serde_json::Value as JsonValue;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use tokio::process::Command;
use uuid::Uuid;

impl BundleContext<'_> {
    /// Build a Windows MSI installer using the WiX toolset.
    ///
    /// This method stages the application into a temporary installer layout, renders a
    /// WiX source file, compiles that source into `.wixobj` files with `candle.exe`,
    /// and links the final `.msi` with `light.exe`.
    ///
    /// The bundling pipeline is:
    /// 1. Resolve the WiX toolchain path that was pre-fetched during context setup.
    /// 2. Normalize the application version into WiX's constrained version format.
    /// 3. Compute installer metadata such as the upgrade code, publisher, icons,
    ///    license files, banner images, and optional fragment references.
    /// 4. Populate a `_staging` directory containing the main executable, copied
    ///    resources, and external binaries.
    /// 5. Render the `.wxs` source from either the built-in template or a user
    ///    supplied template.
    /// 6. Compile the main `.wxs` and any configured fragment files with
    ///    `candle.exe`.
    /// 7. Link all generated `.wixobj` files into the final `.msi` with `light.exe`.
    /// 8. Optionally sign the resulting installer with the configured Windows signing
    ///    settings.
    ///
    /// The final artifact is written to `project_out_directory()/bundle/msi`.
    /// Intermediate `.wxs`, `.wixobj`, and `_staging` files are intentionally left in
    /// place because they are useful when diagnosing packaging failures.
    pub(crate) async fn bundle_windows_msi(&self) -> Result<Vec<PathBuf>> {
        let output_dir = self.project_out_directory().join("msi");
        std::fs::create_dir_all(&output_dir).context("Failed to create MSI output directory")?;

        let wix_settings = self.windows().wix.unwrap_or_default();
        let windows_settings = self.windows();

        let wix_dir = self
            .tools
            .wix_dir
            .as_ref()
            .context("WiX tools were not resolved. This is a bug.")?;
        let candle = wix_dir.join("candle.exe");
        let light = wix_dir.join("light.exe");

        let arch = self.binary_arch();
        let arch_str = arch.windows_arch();
        let wix_arch = arch.wix_arch();
        let wix_program_files_folder = arch.wix_program_files_folder();

        let product_name = self.product_name();
        let version = wix_version(
            &wix_settings
                .version
                .clone()
                .unwrap_or_else(|| self.version_string()),
        )?;

        let msi_name = format!("{product_name}_{version}_{arch_str}.msi");
        let output_path = output_dir.join(&msi_name);

        let upgrade_code = wix_settings.upgrade_code.unwrap_or_else(|| {
            let input = format!("{}.exe.app.{}", product_name, arch_str);
            Uuid::new_v5(&Uuid::NAMESPACE_DNS, input.as_bytes())
        });

        let staging_dir = output_dir.join("_staging");
        if staging_dir.exists() {
            std::fs::remove_dir_all(&staging_dir)?;
        }
        std::fs::create_dir_all(&staging_dir)?;

        let main_binary_src = self.main_binary_path();
        let main_binary_name = format!("{}.exe", self.main_binary_name());
        let main_binary_dest = staging_dir.join(&main_binary_name);
        std::fs::copy(&main_binary_src, &main_binary_dest).with_context(|| {
            format!(
                "Failed to copy main binary from {} to {}",
                main_binary_src.display(),
                main_binary_dest.display()
            )
        })?;

        self.copy_resources(&staging_dir)?;
        self.copy_external_binaries(&staging_dir)?;
        let staged_files = collect_staged_files(&staging_dir, Some(&main_binary_dest))?;
        let (install_tree, component_refs_xml) = render_wix_install_tree(&staged_files);

        let mut data = BTreeMap::new();
        data.insert(
            "product_name".to_string(),
            serde_json::Value::String(product_name.clone()),
        );
        data.insert(
            "version".to_string(),
            serde_json::Value::String(version.clone()),
        );
        data.insert(
            "upgrade_code".to_string(),
            serde_json::Value::String(upgrade_code.to_string()),
        );
        data.insert(
            "wix_program_files_folder".to_string(),
            serde_json::Value::String(wix_program_files_folder.to_string()),
        );
        data.insert(
            "main_binary_name".to_string(),
            serde_json::Value::String(main_binary_name.clone()),
        );
        data.insert(
            "main_binary_path".to_string(),
            serde_json::Value::String(main_binary_dest.to_string_lossy().replace('/', "\\")),
        );
        data.insert(
            "short_description".to_string(),
            serde_json::Value::String(self.short_description()),
        );
        data.insert(
            "bundle_id".to_string(),
            serde_json::Value::String(self.bundle_identifier()),
        );

        let publisher = self
            .publisher()
            .map(|s| s.to_string())
            .or_else(|| self.authors_comma_separated())
            .unwrap_or_else(|| product_name.clone());
        data.insert(
            "publisher".to_string(),
            serde_json::Value::String(publisher),
        );

        data.insert(
            "allow_downgrades".to_string(),
            serde_json::Value::Bool(windows_settings.allow_downgrades),
        );
        data.insert(
            "fips_compliant".to_string(),
            serde_json::Value::Bool(wix_settings.fips_compliant),
        );

        if let Some(icon_path) = &windows_settings.icon_path {
            let icon = self.crate_dir().join(icon_path);
            data.insert(
                "icon_path".to_string(),
                serde_json::Value::String(icon.to_string_lossy().replace('/', "\\")),
            );
        }

        if let Some(license) = &wix_settings.license {
            let license_path = self.crate_dir().join(license);
            data.insert(
                "license".to_string(),
                serde_json::Value::String(license_path.to_string_lossy().replace('/', "\\")),
            );
        }

        if let Some(banner) = &wix_settings.banner_path {
            let banner_path = self.crate_dir().join(banner);
            data.insert(
                "banner_path".to_string(),
                serde_json::Value::String(banner_path.to_string_lossy().replace('/', "\\")),
            );
        }
        if let Some(dialog) = &wix_settings.dialog_image_path {
            let dialog_path = self.crate_dir().join(dialog);
            data.insert(
                "dialog_image_path".to_string(),
                serde_json::Value::String(dialog_path.to_string_lossy().replace('/', "\\")),
            );
        }

        let to_json_array = |v: &[String]| -> serde_json::Value {
            serde_json::Value::Array(
                v.iter()
                    .map(|s| serde_json::Value::String(s.clone()))
                    .collect(),
            )
        };
        data.insert(
            "component_group_refs".to_string(),
            to_json_array(&wix_settings.component_group_refs),
        );
        data.insert(
            "component_refs".to_string(),
            to_json_array(&wix_settings.component_refs),
        );
        data.insert(
            "feature_group_refs".to_string(),
            to_json_array(&wix_settings.feature_group_refs),
        );
        data.insert(
            "feature_refs".to_string(),
            to_json_array(&wix_settings.feature_refs),
        );
        data.insert(
            "merge_refs".to_string(),
            to_json_array(&wix_settings.merge_refs),
        );
        data.insert(
            "install_tree".to_string(),
            serde_json::Value::String(install_tree),
        );
        data.insert(
            "component_refs_xml".to_string(),
            serde_json::Value::String(component_refs_xml),
        );

        let wix_template = if let Some(custom_template) = &wix_settings.template {
            let template_path = self.crate_dir().join(custom_template);
            std::fs::read_to_string(&template_path).with_context(|| {
                format!(
                    "Failed to read custom WiX template: {}",
                    template_path.display()
                )
            })?
        } else {
            WIX_TEMPLATE.to_string()
        };

        let wxs_content =
            render_template(&wix_template, &data).context("Failed to render WiX template")?;

        let wxs_path = output_dir.join(format!("{product_name}.wxs"));
        std::fs::write(&wxs_path, &wxs_content).context("Failed to write WiX source file")?;

        let mut fragment_wxs_paths = Vec::new();
        for fragment in &wix_settings.fragment_paths {
            let frag_path = self.crate_dir().join(fragment);
            if !frag_path.exists() {
                bail!("WiX fragment file not found: {}", frag_path.display());
            }
            fragment_wxs_paths.push(frag_path);
        }

        tracing::info!("Running candle.exe to compile WiX source...");
        let wixobj_path = output_dir.join(format!("{product_name}.wixobj"));

        let mut candle_cmd = Command::new(&candle);
        candle_cmd
            .arg("-arch")
            .arg(wix_arch)
            .arg("-o")
            .arg(&wixobj_path)
            .arg(&wxs_path);

        if wix_settings.fips_compliant {
            candle_cmd.arg("-fips");
        }

        candle_cmd.arg("-ext").arg("WixUIExtension");

        tracing::debug!("candle command: {:?}", candle_cmd);

        let candle_output = candle_cmd
            .output()
            .await
            .with_context(|| format!("Failed to run candle.exe at {}", candle.display()))?;

        if !candle_output.status.success() {
            let stderr = String::from_utf8_lossy(&candle_output.stderr);
            let stdout = String::from_utf8_lossy(&candle_output.stdout);
            bail!(
                "candle.exe failed (exit code {:?}):\nstdout: {}\nstderr: {}",
                candle_output.status.code(),
                stdout,
                stderr
            );
        }

        let mut all_wixobj_paths = vec![wixobj_path];
        for frag_wxs in &fragment_wxs_paths {
            let frag_name = frag_wxs.file_stem().unwrap_or_default().to_string_lossy();
            let frag_wixobj = output_dir.join(format!("{frag_name}.wixobj"));

            let mut frag_candle = Command::new(&candle);
            frag_candle
                .arg("-arch")
                .arg(wix_arch)
                .arg("-o")
                .arg(&frag_wixobj)
                .arg(frag_wxs);

            let frag_output = frag_candle.output().await.with_context(|| {
                format!("Failed to compile WiX fragment: {}", frag_wxs.display())
            })?;

            if !frag_output.status.success() {
                let stderr = String::from_utf8_lossy(&frag_output.stderr);
                bail!(
                    "candle.exe failed on fragment {} (exit code {:?}):\n{}",
                    frag_wxs.display(),
                    frag_output.status.code(),
                    stderr
                );
            }
            all_wixobj_paths.push(frag_wixobj);
        }

        tracing::info!("Running light.exe to link MSI...");

        let mut light_cmd = Command::new(&light);
        light_cmd
            .arg("-o")
            .arg(&output_path)
            .arg("-ext")
            .arg("WixUIExtension");

        for wixobj in &all_wixobj_paths {
            light_cmd.arg(wixobj);
        }

        tracing::debug!("light command: {:?}", light_cmd);

        let light_output = light_cmd
            .output()
            .await
            .with_context(|| format!("Failed to run light.exe at {}", light.display()))?;

        if !light_output.status.success() {
            let stderr = String::from_utf8_lossy(&light_output.stderr);
            let stdout = String::from_utf8_lossy(&light_output.stdout);
            bail!(
                "light.exe failed (exit code {:?}):\nstdout: {}\nstderr: {}",
                light_output.status.code(),
                stdout,
                stderr
            );
        }

        if can_sign_windows(&windows_settings) {
            self.try_sign_windows(&output_path).await?;
        }

        if !output_path.exists() {
            bail!(
                "light.exe completed but MSI not found at {}",
                output_path.display()
            );
        }

        tracing::info!("MSI installer created: {}", output_path.display());
        Ok(vec![output_path])
    }

    /// Build a Windows installer executable using NSIS.
    ///
    /// NSIS packaging follows the same broad staging pattern as MSI bundling, but the
    /// installer behavior is expressed through a generated `.nsi` script rather than
    /// WiX XML.
    ///
    /// The method performs these steps:
    /// 1. Resolve the `makensis` binary prepared during tool resolution.
    /// 2. Create an `_staging` directory containing the main executable, copied
    ///    resources, and external binaries.
    /// 3. Translate bundle settings into template values, including install mode,
    ///    installer imagery, licensing, language selection, and publisher metadata.
    /// 4. Generate the WebView2 installation snippet when the selected webview mode
    ///    requires bootstrapping or bundling the Microsoft runtime installer.
    /// 5. Render the `.nsi` script from either the built-in template or a custom
    ///    template file.
    /// 6. Invoke `makensis` to compile the script into the final installer `.exe`.
    /// 7. Optionally sign the generated installer.
    ///
    /// The resulting installer is written to `project_out_directory()/bundle/nsis`.
    /// As with the MSI flow, the script and staging directory are preserved for
    /// debugging rather than treated as disposable hidden state.
    pub(crate) async fn bundle_windows_nsis(&self) -> Result<Vec<PathBuf>> {
        let output_dir = self.project_out_directory().join("nsis");
        std::fs::create_dir_all(&output_dir).context("Failed to create NSIS output directory")?;

        let nsis_settings = self.windows().nsis.unwrap_or_default();
        let windows_settings = self.windows();

        let nsis_dir = self
            .tools
            .nsis_dir
            .as_ref()
            .context("NSIS tools were not resolved. This is a bug.")?;
        let makensis = if cfg!(target_os = "windows") {
            nsis_dir.join("makensis.exe")
        } else {
            nsis_dir.join("makensis")
        };

        let arch = self.binary_arch();
        let arch_str = arch.windows_arch();

        let product_name = self.product_name();
        let version = self.version_string();
        let installer_name = format!("{product_name}_{version}_{arch_str}-setup.exe");
        let output_path = output_dir.join(&installer_name);

        let staging_dir = output_dir.join("_staging");
        if staging_dir.exists() {
            std::fs::remove_dir_all(&staging_dir)?;
        }
        std::fs::create_dir_all(&staging_dir)?;

        let main_binary_src = self.main_binary_path();
        let main_binary_name = if cfg!(target_os = "windows") || self.target().contains("windows") {
            format!("{}.exe", self.main_binary_name())
        } else {
            self.main_binary_name().to_string()
        };
        let main_binary_dest = staging_dir.join(&main_binary_name);
        std::fs::copy(&main_binary_src, &main_binary_dest).with_context(|| {
            format!(
                "Failed to copy main binary from {} to {}",
                main_binary_src.display(),
                main_binary_dest.display()
            )
        })?;

        self.copy_resources(&staging_dir)?;
        self.copy_external_binaries(&staging_dir)?;
        let staged_files = collect_staged_files(&staging_dir, Some(&main_binary_dest))?;

        let (install_webview, webview_install_code) =
            self.generate_windows_webview_install_code(&windows_settings.webview_install_mode)?;

        let mut data = BTreeMap::new();
        data.insert(
            "product_name".to_string(),
            JsonValue::String(product_name.clone()),
        );
        data.insert("version".to_string(), JsonValue::String(version.clone()));
        data.insert(
            "output_path".to_string(),
            JsonValue::String(output_path.to_string_lossy().replace('/', "\\")),
        );
        data.insert(
            "main_binary_path".to_string(),
            JsonValue::String(main_binary_dest.to_string_lossy().replace('/', "\\")),
        );
        data.insert(
            "main_binary_name".to_string(),
            JsonValue::String(main_binary_name.clone()),
        );
        data.insert(
            "short_description".to_string(),
            JsonValue::String(self.short_description()),
        );
        data.insert(
            "bundle_id".to_string(),
            JsonValue::String(self.bundle_identifier()),
        );

        let publisher = self
            .publisher()
            .map(|s| s.to_string())
            .or_else(|| self.authors_comma_separated())
            .unwrap_or_else(|| product_name.clone());
        data.insert("publisher".to_string(), JsonValue::String(publisher));

        if let Some(copyright) = self.copyright_string() {
            data.insert(
                "copyright".to_string(),
                JsonValue::String(copyright.to_string()),
            );
        }

        let install_mode = &nsis_settings.install_mode;
        data.insert(
            "install_mode_per_machine".to_string(),
            JsonValue::Bool(matches!(install_mode, NSISInstallerMode::PerMachine)),
        );
        data.insert(
            "install_mode_both".to_string(),
            JsonValue::Bool(matches!(install_mode, NSISInstallerMode::Both)),
        );

        let start_menu_folder = nsis_settings
            .start_menu_folder
            .clone()
            .unwrap_or_else(|| product_name.clone());
        data.insert(
            "start_menu_folder".to_string(),
            JsonValue::String(start_menu_folder),
        );

        if let Some(icon) = &nsis_settings.installer_icon {
            let icon_path = self.crate_dir().join(icon);
            data.insert(
                "installer_icon".to_string(),
                JsonValue::String(icon_path.to_string_lossy().replace('/', "\\")),
            );
        }

        if let Some(header) = &nsis_settings.header_image {
            let header_path = self.crate_dir().join(header);
            data.insert(
                "header_image".to_string(),
                JsonValue::String(header_path.to_string_lossy().replace('/', "\\")),
            );
        }

        if let Some(sidebar) = &nsis_settings.sidebar_image {
            let sidebar_path = self.crate_dir().join(sidebar);
            data.insert(
                "sidebar_image".to_string(),
                JsonValue::String(sidebar_path.to_string_lossy().replace('/', "\\")),
            );
        }

        if let Some(license) = &nsis_settings.license {
            let license_path = self.crate_dir().join(license);
            data.insert(
                "license".to_string(),
                JsonValue::String(license_path.to_string_lossy().replace('/', "\\")),
            );
        }

        if let Some(hooks) = &nsis_settings.installer_hooks {
            let hooks_path = self.crate_dir().join(hooks);
            data.insert(
                "installer_hooks".to_string(),
                JsonValue::String(hooks_path.to_string_lossy().replace('/', "\\")),
            );
        }

        if let Some(languages) = &nsis_settings.languages {
            let lang_values: Vec<JsonValue> = languages
                .iter()
                .filter(|l| l.as_str() != "English")
                .map(|l| JsonValue::String(l.clone()))
                .collect();
            data.insert(
                "additional_languages".to_string(),
                JsonValue::Array(lang_values),
            );
        }

        data.insert(
            "install_webview".to_string(),
            JsonValue::Bool(install_webview),
        );
        data.insert(
            "webview_install_code".to_string(),
            JsonValue::String(webview_install_code),
        );
        data.insert(
            "staged_files".to_string(),
            JsonValue::Array(
                staged_files
                    .iter()
                    .map(|file| {
                        serde_json::json!({
                            "source": windows_path_string(&file.source),
                            "target_dir": windows_rel_dir(&file.relative_path),
                        })
                    })
                    .collect(),
            ),
        );

        let nsi_content = if let Some(custom_template) = &nsis_settings.template {
            let template_path = self.crate_dir().join(custom_template);
            let template_str = std::fs::read_to_string(&template_path).with_context(|| {
                format!(
                    "Failed to read custom NSIS template: {}",
                    template_path.display()
                )
            })?;
            render_template(&template_str, &data)?
        } else {
            render_template(NSIS_TEMPLATE, &data)?
        };

        let nsi_path = output_dir.join(format!("{product_name}.nsi"));
        std::fs::write(&nsi_path, &nsi_content).context("Failed to write NSIS script")?;

        tracing::info!("Running makensis to build NSIS installer...");
        let mut cmd = Command::new(&makensis);
        cmd.arg("-NOCD");
        cmd.arg(&nsi_path);

        tracing::debug!("makensis command: {:?}", cmd);

        let output = cmd
            .output()
            .await
            .with_context(|| format!("Failed to run makensis at {}", makensis.display()))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            bail!(
                "makensis failed (exit code {:?}):\nstdout: {}\nstderr: {}",
                output.status.code(),
                stdout,
                stderr
            );
        }

        if can_sign_windows(&windows_settings) {
            self.try_sign_windows(&output_path).await?;
        }

        if !output_path.exists() {
            bail!(
                "makensis completed but installer not found at {}",
                output_path.display()
            );
        }

        tracing::info!("NSIS installer created: {}", output_path.display());
        Ok(vec![output_path])
    }

    /// Attempt to sign a binary at the given path using the Windows signing configuration.
    async fn try_sign_windows(&self, path: &Path) -> Result<()> {
        let settings = self.windows();

        if !can_sign_windows(&settings) {
            return Ok(());
        }

        tracing::info!("Signing {}", path.display());

        if let Some(sign_cmd) = &settings.sign_command {
            return run_custom_sign_command(path, &sign_cmd.cmd, &sign_cmd.args).await;
        }

        if let Some(thumbprint) = &settings.certificate_thumbprint {
            return run_signtool_sign(path, thumbprint, &settings).await;
        }

        Ok(())
    }

    /// Generate the NSIS code snippet for WebView2 installation based on the install mode.
    fn generate_windows_webview_install_code(
        &self,
        mode: &WebviewInstallMode,
    ) -> Result<(bool, String)> {
        match mode {
            WebviewInstallMode::Skip | WebviewInstallMode::FixedRuntime { .. } => {
                Ok((false, String::new()))
            }

            WebviewInstallMode::DownloadBootstrapper { silent }
            | WebviewInstallMode::EmbedBootstrapper { silent } => {
                let installer_path = self
                    .tools
                    .webview2_installer
                    .as_ref()
                    .context("WebView2 installer was not pre-downloaded. This is a bug.")?;
                let silent_flag = if *silent { " /silent" } else { "" };
                let code = format!(
                    r#"    ; Install WebView2 via bootstrapper
    SetOutPath "$TEMP"
    File "{bootstrapper}"
    ExecWait '"$TEMP\MicrosoftEdgeWebview2Setup.exe"{silent_flag} /install' $0
    Delete "$TEMP\MicrosoftEdgeWebview2Setup.exe""#,
                    bootstrapper = installer_path.to_string_lossy().replace('/', "\\"),
                );
                Ok((true, code))
            }

            WebviewInstallMode::OfflineInstaller { silent } => {
                let installer_path = self
                    .tools
                    .webview2_installer
                    .as_ref()
                    .context("WebView2 installer was not pre-downloaded. This is a bug.")?;
                let silent_flag = if *silent { " /silent" } else { "" };
                let installer_name = installer_path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy();
                let code = format!(
                    r#"    ; Install WebView2 via offline installer
    SetOutPath "$TEMP"
    File "{installer}"
    ExecWait '"$TEMP\{installer_name}"{silent_flag} /install' $0
    Delete "$TEMP\{installer_name}""#,
                    installer = installer_path.to_string_lossy().replace('/', "\\"),
                );
                Ok((true, code))
            }
        }
    }
}

#[derive(Clone, Debug)]
struct StagedFile {
    source: PathBuf,
    relative_path: PathBuf,
}

#[derive(Default)]
struct WixDirTree {
    files: Vec<StagedFile>,
    dirs: BTreeMap<String, WixDirTree>,
}

impl WixDirTree {
    fn insert(&mut self, file: StagedFile) {
        let mut cursor = self;
        if let Some(parent) = file.relative_path.parent() {
            for component in parent.components() {
                let name = component.as_os_str().to_string_lossy().to_string();
                if name.is_empty() {
                    continue;
                }
                cursor = cursor.dirs.entry(name).or_default();
            }
        }
        cursor.files.push(file);
    }
}

/// Returns `true` if the Windows settings have signing configured.
fn can_sign_windows(settings: &WindowsSettings) -> bool {
    settings.certificate_thumbprint.is_some() || settings.sign_command.is_some()
}

/// Collect all files in the staging directory except the explicitly skipped file.
fn collect_staged_files(staging_dir: &Path, skip_file: Option<&Path>) -> Result<Vec<StagedFile>> {
    let mut files = Vec::new();

    for entry in walkdir::WalkDir::new(staging_dir) {
        let entry = entry?;
        if !entry.file_type().is_file() {
            continue;
        }

        let path = entry.path().to_path_buf();
        if skip_file.is_some_and(|skip| skip == path) {
            continue;
        }

        files.push(StagedFile {
            relative_path: path
                .strip_prefix(staging_dir)
                .unwrap_or(entry.path())
                .to_path_buf(),
            source: path,
        });
    }

    files.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));
    Ok(files)
}

/// Render the staged file tree into WiX `<Directory>` / `<Component>` XML fragments.
fn render_wix_install_tree(staged_files: &[StagedFile]) -> (String, String) {
    let mut tree = WixDirTree::default();
    for file in staged_files {
        tree.insert(file.clone());
    }

    let mut next_id = 0usize;
    let mut component_refs = String::new();
    let install_tree = render_wix_dir_contents(&tree, 20, &mut next_id, &mut component_refs);
    (install_tree, component_refs)
}

fn render_wix_dir_contents(
    tree: &WixDirTree,
    indent: usize,
    next_id: &mut usize,
    component_refs: &mut String,
) -> String {
    let mut xml = String::new();
    let pad = " ".repeat(indent);
    let file_pad = " ".repeat(indent + 4);

    for file in &tree.files {
        let component_id = format!("Component_{}", *next_id);
        *next_id += 1;
        let file_id = format!("File_{}", *next_id);
        *next_id += 1;

        component_refs.push_str(&format!(
            "\n            <ComponentRef Id=\"{component_id}\" />"
        ));

        let file_name = file
            .relative_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy();
        let source = windows_path_string(&file.source);
        xml.push_str(&format!(
            "\n{pad}<Component Id=\"{component_id}\" Guid=\"*\">\n{file_pad}<File Id=\"{file_id}\" Name=\"{}\" Source=\"{}\" KeyPath=\"yes\" />\n{pad}</Component>",
            xml_attr_escape(&file_name),
            xml_attr_escape(&source),
        ));
    }

    for (name, child) in &tree.dirs {
        let dir_id = format!("Directory_{}", *next_id);
        *next_id += 1;
        let child_xml = render_wix_dir_contents(child, indent + 4, next_id, component_refs);
        xml.push_str(&format!(
            "\n{pad}<Directory Id=\"{dir_id}\" Name=\"{}\">{child_xml}\n{pad}</Directory>",
            xml_attr_escape(name),
        ));
    }

    xml
}

fn windows_path_string(path: &Path) -> String {
    path.to_string_lossy().replace('/', "\\")
}

fn windows_rel_dir(relative_path: &Path) -> String {
    relative_path
        .parent()
        .map(|p| p.to_string_lossy().replace('/', "\\"))
        .unwrap_or_default()
}

fn xml_attr_escape(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Run a custom signing command. The `%1` placeholder in args is replaced
/// with the path to the binary to sign.
async fn run_custom_sign_command(path: &Path, cmd: &str, args: &[String]) -> Result<()> {
    let path_str = path.to_string_lossy();
    let resolved_args: Vec<String> = args
        .iter()
        .map(|arg| arg.replace("%1", &path_str))
        .collect();

    tracing::debug!("Running custom sign command: {} {:?}", cmd, resolved_args);

    let status = Command::new(cmd)
        .args(&resolved_args)
        .status()
        .await
        .with_context(|| format!("Failed to run custom sign command: {cmd}"))?;

    if !status.success() {
        bail!(
            "Custom sign command failed with exit code: {:?}",
            status.code()
        );
    }

    Ok(())
}

/// Run signtool.exe to sign a binary with a certificate thumbprint.
async fn run_signtool_sign(
    path: &Path,
    thumbprint: &str,
    settings: &WindowsSettings,
) -> Result<()> {
    let mut args = vec![
        "sign".to_string(),
        "/fd".to_string(),
        settings
            .digest_algorithm
            .clone()
            .unwrap_or_else(|| "sha256".to_string()),
        "/sha1".to_string(),
        thumbprint.to_string(),
    ];

    if let Some(timestamp_url) = &settings.timestamp_url {
        if settings.tsp {
            args.push("/tr".to_string());
            args.push(timestamp_url.clone());
            args.push("/td".to_string());
            args.push(
                settings
                    .digest_algorithm
                    .clone()
                    .unwrap_or_else(|| "sha256".to_string()),
            );
        } else {
            args.push("/t".to_string());
            args.push(timestamp_url.clone());
        }
    }

    args.push(path.to_string_lossy().to_string());

    tracing::debug!("Running signtool with args: {:?}", args);

    let status = Command::new("signtool.exe")
        .args(&args)
        .status()
        .await
        .context("Failed to run signtool.exe. Is the Windows SDK installed?")?;

    if !status.success() {
        bail!("signtool.exe failed with exit code: {:?}", status.code());
    }

    Ok(())
}

/// Render a Handlebars template with the given data.
///
/// Handlebars treats `\{{` as an escape sequence that outputs a literal `{{`.
/// This is a problem for Windows paths like `$INSTDIR\{{product_name}}` where
/// the backslash is a path separator, not an escape. We work around this by
/// replacing `\{{` with a placeholder before Handlebars parses the template,
/// then restoring the backslash in the rendered output.
fn render_template(template: &str, data: &BTreeMap<String, JsonValue>) -> Result<String> {
    const BACKSLASH_PLACEHOLDER: &str = "\x00BSEP\x00";
    let replacement = String::from(BACKSLASH_PLACEHOLDER) + "{{";
    let safe_template = template.replace("\\{{", &replacement);
    let mut hbs = Handlebars::new();
    hbs.set_strict_mode(false);
    hbs.register_escape_fn(|s: &str| s.to_string());
    hbs.register_template_string("template", &safe_template)
        .context("Failed to parse template")?;
    let rendered = hbs
        .render("template", data)
        .context("Failed to render template")?;
    Ok(rendered.replace(BACKSLASH_PLACEHOLDER, "\\"))
}

/// Convert a semver version string to a WiX-compatible version.
fn wix_version(version: &str) -> Result<String> {
    let version = version.split('-').next().unwrap_or(version);
    let parts: Vec<&str> = version.split('.').collect();

    if parts.len() < 2 || parts.len() > 4 {
        bail!(
            "Invalid version for MSI: '{}'. Expected format: major.minor.patch[.build]",
            version
        );
    }

    for (i, part) in parts.iter().enumerate() {
        let num: u64 = part
            .parse()
            .with_context(|| format!("Invalid version component: '{part}'"))?;
        match i {
            0 | 1 if num > 255 => {
                bail!("Version component {part} exceeds maximum value of 255");
            }
            2 | 3 if num > 65535 => {
                bail!("Version component {part} exceeds maximum value of 65535");
            }
            _ => {}
        }
    }

    let version_str = if parts.len() == 2 {
        format!("{}.{}.0", parts[0], parts[1])
    } else {
        parts.join(".")
    };

    Ok(version_str)
}

/// The embedded WiX template.
const WIX_TEMPLATE: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<Wix xmlns="http://schemas.microsoft.com/wix/2006/wi">
    <Product
        Id="*"
        Name="{{product_name}}"
        UpgradeCode="{{upgrade_code}}"
        Language="1033"
        Codepage="1252"
        Version="{{version}}"
        Manufacturer="{{publisher}}">

        <Package
            Id="*"
            Keywords="Installer"
            {{#if short_description}}
            Description="{{short_description}}"
            {{/if}}
            Manufacturer="{{publisher}}"
            InstallScope="perMachine"
            Languages="1033"
            Compressed="yes"
            SummaryCodepage="1252" />

        <MajorUpgrade
            Schedule="afterInstallInitialize"
            {{#if allow_downgrades}}
            AllowDowngrades="yes"
            {{else}}
            DowngradeErrorMessage="A newer version of [ProductName] is already installed. Setup will now exit."
            AllowSameVersionUpgrades="yes"
            {{/if}} />

        <MediaTemplate EmbedCab="yes" {{#if fips_compliant}}CompressionLevel="none"{{/if}} />

        {{#if icon_path}}
        <Icon Id="ProductIcon" SourceFile="{{icon_path}}" />
        <Property Id="ARPPRODUCTICON" Value="ProductIcon" />
        {{/if}}

        {{#if license}}
        <WixVariable Id="WixUILicenseRtf" Value="{{license}}" />
        {{/if}}

        {{#if banner_path}}
        <WixVariable Id="WixUIBannerBmp" Value="{{banner_path}}" />
        {{/if}}
        {{#if dialog_image_path}}
        <WixVariable Id="WixUIDialogBmp" Value="{{dialog_image_path}}" />
        {{/if}}

        {{#if license}}
        <UIRef Id="WixUI_InstallDir" />
        {{else}}
        <UI>
            <UIRef Id="WixUI_InstallDir" />
            <Publish Dialog="WelcomeDlg" Control="Next" Event="NewDialog" Value="InstallDirDlg" Order="2">1</Publish>
            <Publish Dialog="InstallDirDlg" Control="Back" Event="NewDialog" Value="WelcomeDlg" Order="2">1</Publish>
        </UI>
        {{/if}}
        <Property Id="WIXUI_INSTALLDIR" Value="INSTALLDIR" />

        <Directory Id="TARGETDIR" Name="SourceDir">
            <Directory Id="{{wix_program_files_folder}}">
                <Directory Id="INSTALLDIR" Name="{{product_name}}">
                    <Component Id="MainExecutable" Guid="*">
                        <File
                            Id="MainExe"
                            Name="{{main_binary_name}}"
                            Source="{{main_binary_path}}"
                            KeyPath="yes" />
                    </Component>
{{{install_tree}}}
                </Directory>
            </Directory>

            <Directory Id="ProgramMenuFolder">
                <Directory Id="ProgramMenuSubfolder" Name="{{product_name}}">
                    <Component Id="StartMenuShortcut" Guid="*">
                        <Shortcut
                            Id="ApplicationShortcut"
                            Name="{{product_name}}"
                            {{#if short_description}}
                            Description="{{short_description}}"
                            {{/if}}
                            Target="[INSTALLDIR]{{main_binary_name}}"
                            WorkingDirectory="INSTALLDIR" />
                        <RemoveFolder Id="RemoveProgramMenuSubfolder" On="uninstall" />
                        <RegistryValue
                            Root="HKCU"
                            Key="Software\{{publisher}}\{{product_name}}"
                            Name="installed"
                            Type="integer"
                            Value="1"
                            KeyPath="yes" />
                    </Component>
                </Directory>
            </Directory>

            <Directory Id="DesktopFolder">
                <Component Id="DesktopShortcut" Guid="*">
                    <Shortcut
                        Id="DesktopShortcut"
                        Name="{{product_name}}"
                        {{#if short_description}}
                        Description="{{short_description}}"
                        {{/if}}
                        Target="[INSTALLDIR]{{main_binary_name}}"
                        WorkingDirectory="INSTALLDIR" />
                    <RegistryValue
                        Root="HKCU"
                        Key="Software\{{publisher}}\{{product_name}}"
                        Name="desktop_shortcut"
                        Type="integer"
                        Value="1"
                        KeyPath="yes" />
                </Component>
            </Directory>
        </Directory>

        <Feature Id="MainFeature" Title="{{product_name}}" Level="1">
            <ComponentRef Id="MainExecutable" />
            <ComponentRef Id="StartMenuShortcut" />
            <ComponentRef Id="DesktopShortcut" />
{{{component_refs_xml}}}
            {{#each component_group_refs}}
            <ComponentGroupRef Id="{{this}}" />
            {{/each}}
            {{#each component_refs}}
            <ComponentRef Id="{{this}}" />
            {{/each}}
            {{#each feature_group_refs}}
            <FeatureGroupRef Id="{{this}}" />
            {{/each}}
            {{#each feature_refs}}
            <FeatureRef Id="{{this}}" />
            {{/each}}
            {{#each merge_refs}}
            <MergeRef Id="{{this}}" />
            {{/each}}
        </Feature>
    </Product>
</Wix>
"#;

/// The embedded NSIS template script.
const NSIS_TEMPLATE: &str = r#"!include "MUI2.nsh"
!include "FileFunc.nsh"
!include "x64.nsh"

; Basic installer attributes
Name "{{product_name}}"
OutFile "{{output_path}}"
Unicode true
{{#if install_mode_per_machine}}
InstallDir "$PROGRAMFILES\{{product_name}}"
{{else}}
InstallDir "$LOCALAPPDATA\Programs\{{product_name}}"
{{/if}}

; Request appropriate privileges
{{#if install_mode_per_machine}}
RequestExecutionLevel admin
{{else if install_mode_both}}
RequestExecutionLevel admin
{{else}}
RequestExecutionLevel user
{{/if}}

; Version information
VIProductVersion "{{version}}.0"
VIAddVersionKey "ProductName" "{{product_name}}"
VIAddVersionKey "FileVersion" "{{version}}"
VIAddVersionKey "ProductVersion" "{{version}}"
VIAddVersionKey "FileDescription" "{{short_description}}"
{{#if publisher}}
VIAddVersionKey "CompanyName" "{{publisher}}"
{{/if}}
{{#if copyright}}
VIAddVersionKey "LegalCopyright" "{{copyright}}"
{{/if}}

; MUI settings
!define MUI_ABORTWARNING
{{#if installer_icon}}
!define MUI_ICON "{{installer_icon}}"
{{/if}}
{{#if header_image}}
!define MUI_HEADERIMAGE
!define MUI_HEADERIMAGE_BITMAP "{{header_image}}"
{{/if}}
{{#if sidebar_image}}
!define MUI_WELCOMEFINISHPAGE_BITMAP "{{sidebar_image}}"
{{/if}}

; Pages
{{#if license}}
!insertmacro MUI_PAGE_LICENSE "{{license}}"
{{/if}}
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH

; Uninstaller pages
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES

; Language
!insertmacro MUI_LANGUAGE "English"
{{#each additional_languages}}
!insertmacro MUI_LANGUAGE "{{this}}"
{{/each}}

; Installer section
Section "Install"
    SetOutPath $INSTDIR

    ; Install main binary
    File "{{main_binary_path}}"

    ; Install resources
    {{#each staged_files}}
    SetOutPath "$INSTDIR{{#if this.target_dir}}\{{this.target_dir}}{{/if}}"
    File "{{this.source}}"
    {{/each}}

    SetOutPath $INSTDIR

    ; Create uninstaller
    WriteUninstaller "$INSTDIR\uninstall.exe"

    ; Create Start Menu shortcuts
    CreateDirectory "$SMPROGRAMS\{{start_menu_folder}}"
    CreateShortcut "$SMPROGRAMS\{{start_menu_folder}}\{{product_name}}.lnk" "$INSTDIR\{{main_binary_name}}"
    CreateShortcut "$SMPROGRAMS\{{start_menu_folder}}\Uninstall {{product_name}}.lnk" "$INSTDIR\uninstall.exe"

    ; Create Desktop shortcut
    CreateShortcut "$DESKTOP\{{product_name}}.lnk" "$INSTDIR\{{main_binary_name}}"

    ; Write registry keys for Add/Remove Programs
    WriteRegStr SHCTX "Software\Microsoft\Windows\CurrentVersion\Uninstall\{{bundle_id}}" \
        "DisplayName" "{{product_name}}"
    WriteRegStr SHCTX "Software\Microsoft\Windows\CurrentVersion\Uninstall\{{bundle_id}}" \
        "UninstallString" '"$INSTDIR\uninstall.exe"'
    WriteRegStr SHCTX "Software\Microsoft\Windows\CurrentVersion\Uninstall\{{bundle_id}}" \
        "DisplayVersion" "{{version}}"
    {{#if publisher}}
    WriteRegStr SHCTX "Software\Microsoft\Windows\CurrentVersion\Uninstall\{{bundle_id}}" \
        "Publisher" "{{publisher}}"
    {{/if}}
    WriteRegStr SHCTX "Software\Microsoft\Windows\CurrentVersion\Uninstall\{{bundle_id}}" \
        "InstallLocation" "$INSTDIR"

    ; Get installed size
    ${GetSize} "$INSTDIR" "/S=0K" $0 $1 $2
    IntFmt $0 "0x%08X" $0
    WriteRegDWORD SHCTX "Software\Microsoft\Windows\CurrentVersion\Uninstall\{{bundle_id}}" \
        "EstimatedSize" "$0"

    {{#if install_webview}}
    ; WebView2 installation
    {{webview_install_code}}
    {{/if}}

SectionEnd

{{#if installer_hooks}}
!include "{{installer_hooks}}"
{{/if}}

; Uninstaller section
Section "Uninstall"
    ; Remove files
    RMDir /r "$INSTDIR"

    ; Remove Start Menu items
    RMDir /r "$SMPROGRAMS\{{start_menu_folder}}"

    ; Remove Desktop shortcut
    Delete "$DESKTOP\{{product_name}}.lnk"

    ; Remove registry keys
    DeleteRegKey SHCTX "Software\Microsoft\Windows\CurrentVersion\Uninstall\{{bundle_id}}"
SectionEnd
"#;

#[cfg(test)]
mod tests {
    use super::{WIX_TEMPLATE, render_template};
    use crate::bundler::Arch;
    use serde_json::json;
    use std::collections::BTreeMap;

    #[test]
    fn wix_program_files_folder_matches_architecture() {
        assert_eq!(Arch::X86.wix_program_files_folder(), "ProgramFilesFolder");
        assert_eq!(
            Arch::X86_64.wix_program_files_folder(),
            "ProgramFiles64Folder"
        );
        assert_eq!(
            Arch::AArch64.wix_program_files_folder(),
            "ProgramFiles64Folder"
        );
    }

    #[test]
    fn wix_template_uses_arch_specific_program_files_folder() {
        let data: BTreeMap<String, serde_json::Value> = serde_json::from_value(json!({
            "product_name": "Hotdog",
            "upgrade_code": "00000000-0000-0000-0000-000000000000",
            "version": "0.1.0",
            "publisher": "Dioxus Labs",
            "main_binary_name": "hotdog.exe",
            "main_binary_path": "C:\\staging\\hotdog.exe",
            "short_description": "Hotdog app",
            "allow_downgrades": false,
            "fips_compliant": false,
            "install_tree": "",
            "component_refs_xml": "",
            "component_group_refs": [],
            "component_refs": [],
            "feature_group_refs": [],
            "feature_refs": [],
            "merge_refs": [],
            "wix_program_files_folder": "ProgramFiles64Folder"
        }))
        .unwrap();

        let rendered = render_template(WIX_TEMPLATE, &data).unwrap();

        assert!(rendered.contains("<Directory Id=\"ProgramFiles64Folder\">"));
        assert!(!rendered.contains("<Directory Id=\"ProgramFilesFolder\">"));
        // Verify \{{ substitution works: registry key should contain actual values
        assert!(rendered.contains("Software\\Dioxus Labs\\Hotdog"));
    }
}
