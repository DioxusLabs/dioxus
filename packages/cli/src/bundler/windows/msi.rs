//! WiX MSI installer creation for Windows.
//!
//! Generates a WiX source (.wxs) file and compiles it into an MSI installer
//! using the WiX toolset (candle.exe + light.exe).
//!
//! This module is Windows-only since WiX only runs on Windows.

#[cfg(target_os = "windows")]
mod inner {
    use super::super::sign;
    use super::super::util;
    use crate::bundler::context::Arch;
    use crate::bundler::tools::ensure_wix;
    use crate::bundler::BundleContext;
    use anyhow::{bail, Context, Result};
    use handlebars::Handlebars;
    use std::collections::BTreeMap;
    use std::path::PathBuf;
    use uuid::Uuid;

    /// The embedded WiX template.
    ///
    /// This is a simplified WiX XML source that handles core MSI installation.
    /// For advanced use cases, users can provide a custom template via WixSettings.
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
            Description="{{short_description}}"
            Manufacturer="{{publisher}}"
            InstalledScope="perMachine"
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

        <UIRef Id="WixUI_InstallDir" />
        <Property Id="WIXUI_INSTALLDIR" Value="INSTALLDIR" />

        <Directory Id="TARGETDIR" Name="SourceDir">
            <Directory Id="ProgramFilesFolder">
                <Directory Id="INSTALLDIR" Name="{{product_name}}">
                    <Component Id="MainExecutable" Guid="*">
                        <File
                            Id="MainExe"
                            Name="{{main_binary_name}}"
                            Source="{{main_binary_path}}"
                            KeyPath="yes" />
                    </Component>
                    {{#each resource_components}}
                    <Component Id="Resource_{{this.id}}" Guid="*">
                        <File
                            Id="ResourceFile_{{this.id}}"
                            Name="{{this.name}}"
                            Source="{{this.source}}"
                            KeyPath="yes" />
                    </Component>
                    {{/each}}
                </Directory>
            </Directory>

            <Directory Id="ProgramMenuFolder">
                <Directory Id="ProgramMenuSubfolder" Name="{{product_name}}">
                    <Component Id="StartMenuShortcut" Guid="*">
                        <Shortcut
                            Id="ApplicationShortcut"
                            Name="{{product_name}}"
                            Description="{{short_description}}"
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
                        Description="{{short_description}}"
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
            {{#each resource_components}}
            <ComponentRef Id="Resource_{{this.id}}" />
            {{/each}}
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

    /// Bundle the project as a WiX MSI installer.
    ///
    /// Returns the path(s) to the generated MSI file(s).
    pub(crate) fn bundle_project(ctx: &BundleContext) -> Result<Vec<PathBuf>> {
        let output_dir = ctx
            .project_out_directory()
            .join("bundle")
            .join(util::WIX_OUTPUT_FOLDER_NAME);
        std::fs::create_dir_all(&output_dir).context("Failed to create MSI output directory")?;

        let wix_settings = ctx.windows().wix.unwrap_or_default();
        let windows_settings = ctx.windows();

        // Ensure WiX toolchain is available
        let wix_dir = ensure_wix(&ctx.tools_dir())?;
        let candle = wix_dir.join("candle.exe");
        let light = wix_dir.join("light.exe");

        let arch = ctx.binary_arch();
        let arch_str = util::arch_to_windows_string(&arch);
        let wix_arch = match arch {
            Arch::X86_64 => "x64",
            Arch::X86 => "x86",
            Arch::AArch64 => "arm64",
            _ => "x64",
        };

        let product_name = ctx.product_name();
        let version = wix_version(
            &wix_settings.version.clone().unwrap_or_else(|| ctx.version_string()),
        )?;

        let msi_name = format!("{product_name}_{version}_{arch_str}.msi");
        let output_path = output_dir.join(&msi_name);

        // Generate upgrade code from product name if not provided
        let upgrade_code = wix_settings.upgrade_code.unwrap_or_else(|| {
            let input = format!("{}.exe.app.{}", product_name, arch_str);
            Uuid::new_v5(&Uuid::NAMESPACE_DNS, input.as_bytes())
        });

        // Copy main binary to staging area
        let staging_dir = output_dir.join("_staging");
        if staging_dir.exists() {
            std::fs::remove_dir_all(&staging_dir)?;
        }
        std::fs::create_dir_all(&staging_dir)?;

        let main_binary_src = ctx.main_binary_path();
        let main_binary_name = format!("{}.exe", ctx.main_binary_name());
        let main_binary_dest = staging_dir.join(&main_binary_name);
        std::fs::copy(&main_binary_src, &main_binary_dest).with_context(|| {
            format!(
                "Failed to copy main binary from {} to {}",
                main_binary_src.display(),
                main_binary_dest.display()
            )
        })?;

        // Copy resources
        let resources_dir = staging_dir.join("resources");
        std::fs::create_dir_all(&resources_dir)?;
        ctx.copy_resources(&resources_dir)?;
        ctx.copy_external_binaries(&staging_dir)?;

        // Build template data
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
            "main_binary_name".to_string(),
            serde_json::Value::String(main_binary_name.clone()),
        );
        data.insert(
            "main_binary_path".to_string(),
            serde_json::Value::String(
                main_binary_dest.to_string_lossy().replace('/', "\\"),
            ),
        );
        data.insert(
            "short_description".to_string(),
            serde_json::Value::String(ctx.short_description()),
        );
        data.insert(
            "bundle_id".to_string(),
            serde_json::Value::String(ctx.bundle_identifier()),
        );

        // Publisher
        let publisher = ctx
            .publisher()
            .map(|s| s.to_string())
            .or_else(|| ctx.authors_comma_separated())
            .unwrap_or_else(|| product_name.clone());
        data.insert(
            "publisher".to_string(),
            serde_json::Value::String(publisher),
        );

        // Allow downgrades
        data.insert(
            "allow_downgrades".to_string(),
            serde_json::Value::Bool(windows_settings.allow_downgrades),
        );

        // FIPS compliant
        data.insert(
            "fips_compliant".to_string(),
            serde_json::Value::Bool(wix_settings.fips_compliant),
        );

        // Icon
        if let Some(icon_path) = &windows_settings.icon_path {
            let icon = ctx.crate_dir().join(icon_path);
            data.insert(
                "icon_path".to_string(),
                serde_json::Value::String(icon.to_string_lossy().replace('/', "\\")),
            );
        }

        // License
        if let Some(license) = &wix_settings.license {
            let license_path = ctx.crate_dir().join(license);
            data.insert(
                "license".to_string(),
                serde_json::Value::String(license_path.to_string_lossy().replace('/', "\\")),
            );
        }

        // Banner and dialog images
        if let Some(banner) = &wix_settings.banner_path {
            let banner_path = ctx.crate_dir().join(banner);
            data.insert(
                "banner_path".to_string(),
                serde_json::Value::String(banner_path.to_string_lossy().replace('/', "\\")),
            );
        }
        if let Some(dialog) = &wix_settings.dialog_image_path {
            let dialog_path = ctx.crate_dir().join(dialog);
            data.insert(
                "dialog_image_path".to_string(),
                serde_json::Value::String(dialog_path.to_string_lossy().replace('/', "\\")),
            );
        }

        // Component/feature refs from settings
        let to_json_array = |v: &[String]| -> serde_json::Value {
            serde_json::Value::Array(v.iter().map(|s| serde_json::Value::String(s.clone())).collect())
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

        // Resource components (simplified - empty for now)
        // TODO: Enumerate resources and create component entries
        data.insert(
            "resource_components".to_string(),
            serde_json::Value::Array(Vec::new()),
        );

        // Render the WiX source
        let wxs_content = if let Some(custom_template) = &wix_settings.template {
            let template_path = ctx.crate_dir().join(custom_template);
            let template_str = std::fs::read_to_string(&template_path).with_context(|| {
                format!(
                    "Failed to read custom WiX template: {}",
                    template_path.display()
                )
            })?;
            render_template(&template_str, &data)?
        } else {
            render_template(WIX_TEMPLATE, &data)?
        };

        // Write the .wxs file
        let wxs_path = output_dir.join(format!("{product_name}.wxs"));
        std::fs::write(&wxs_path, &wxs_content).context("Failed to write WiX source file")?;

        // Include any fragment files
        let mut fragment_wxs_paths = Vec::new();
        for fragment in &wix_settings.fragment_paths {
            let frag_path = ctx.crate_dir().join(fragment);
            if !frag_path.exists() {
                bail!("WiX fragment file not found: {}", frag_path.display());
            }
            fragment_wxs_paths.push(frag_path);
        }

        // Step 1: Run candle.exe to compile .wxs to .wixobj
        tracing::info!("Running candle.exe to compile WiX source...");
        let wixobj_path = output_dir.join(format!("{product_name}.wixobj"));

        let mut candle_cmd = std::process::Command::new(&candle);
        candle_cmd
            .arg("-arch")
            .arg(wix_arch)
            .arg("-o")
            .arg(&wixobj_path)
            .arg(&wxs_path);

        // Add extension for UI
        candle_cmd.arg("-ext").arg("WixUIExtension");

        tracing::debug!("candle command: {:?}", candle_cmd);

        let candle_output = candle_cmd
            .output()
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

        // Also compile fragment files
        let mut all_wixobj_paths = vec![wixobj_path];
        for frag_wxs in &fragment_wxs_paths {
            let frag_name = frag_wxs
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy();
            let frag_wixobj = output_dir.join(format!("{frag_name}.wixobj"));

            let mut frag_candle = std::process::Command::new(&candle);
            frag_candle
                .arg("-arch")
                .arg(wix_arch)
                .arg("-o")
                .arg(&frag_wixobj)
                .arg(frag_wxs);

            let frag_output = frag_candle
                .output()
                .with_context(|| format!("Failed to compile WiX fragment: {}", frag_wxs.display()))?;

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

        // Step 2: Run light.exe to link .wixobj to .msi
        tracing::info!("Running light.exe to link MSI...");

        let mut light_cmd = std::process::Command::new(&light);
        light_cmd
            .arg("-o")
            .arg(&output_path)
            .arg("-ext")
            .arg("WixUIExtension");

        // Add all wixobj files
        for wixobj in &all_wixobj_paths {
            light_cmd.arg(wixobj);
        }

        tracing::debug!("light command: {:?}", light_cmd);

        let light_output = light_cmd
            .output()
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

        // Sign the MSI if configured
        if sign::can_sign(&windows_settings) {
            sign::try_sign(&output_path, ctx)?;
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

    /// Render a Handlebars template with the given data.
    fn render_template(
        template: &str,
        data: &BTreeMap<String, serde_json::Value>,
    ) -> Result<String> {
        let mut hbs = Handlebars::new();
        hbs.set_strict_mode(false);
        // Disable HTML escaping since we're generating XML, not HTML
        hbs.register_escape_fn(|s: &str| s.to_string());
        hbs.register_template_string("wix", template)
            .context("Failed to parse WiX template")?;
        hbs.render("wix", data)
            .context("Failed to render WiX template")
    }

    /// Convert a semver version string to a WiX-compatible version.
    ///
    /// WiX requires versions in the format `major.minor.patch[.build]` where
    /// major/minor are 0-255 and patch/build are 0-65535.
    /// Pre-release suffixes are stripped.
    fn wix_version(version: &str) -> Result<String> {
        // Strip any pre-release suffix (e.g., "-beta.1")
        let version = version.split('-').next().unwrap_or(version);
        let parts: Vec<&str> = version.split('.').collect();

        if parts.len() < 2 || parts.len() > 4 {
            bail!(
                "Invalid version for MSI: '{}'. Expected format: major.minor.patch[.build]",
                version
            );
        }

        // Validate each part is a valid number within range
        for (i, part) in parts.iter().enumerate() {
            let num: u64 = part
                .parse()
                .with_context(|| format!("Invalid version component: '{part}'"))?;
            match i {
                0 | 1 => {
                    if num > 255 {
                        bail!("Version component {part} exceeds maximum value of 255");
                    }
                }
                2 | 3 => {
                    if num > 65535 {
                        bail!("Version component {part} exceeds maximum value of 65535");
                    }
                }
                _ => {}
            }
        }

        // Ensure at least 3 components
        let version_str = if parts.len() == 2 {
            format!("{}.{}.0", parts[0], parts[1])
        } else {
            parts.join(".")
        };

        Ok(version_str)
    }
}

#[cfg(target_os = "windows")]
pub(crate) use inner::bundle_project;

#[cfg(not(target_os = "windows"))]
pub(crate) fn bundle_project(
    _ctx: &crate::bundler::BundleContext,
) -> anyhow::Result<Vec<std::path::PathBuf>> {
    anyhow::bail!("MSI bundling is only supported on Windows")
}
