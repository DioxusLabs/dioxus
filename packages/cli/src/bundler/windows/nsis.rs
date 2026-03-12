//! NSIS installer creation for Windows.
//!
//! Generates an NSIS script and runs makensis to produce a Windows installer.
//! NSIS can be run on non-Windows platforms (via native builds or wine),
//! so this module is not cfg-gated to Windows only.

use super::sign;
use super::util;
use crate::bundler::BundleContext;
use crate::{NSISInstallerMode, WebviewInstallMode};
use anyhow::{bail, Context, Result};
use handlebars::Handlebars;
use serde_json::Value as JsonValue;
use std::collections::BTreeMap;
use std::path::PathBuf;
use tokio::process::Command;

/// Bundle the project as an NSIS installer.
///
/// NSIS packaging renders an `.nsi` script (embedded template or user template), stages
/// application files, and executes `makensis` to produce an installer executable.
///
/// Required external tooling:
/// - NSIS `makensis` binary resolved by bundler tools setup
///
/// Bundle staging and output layout:
/// - `_staging/` contains main binary, resources, and external binaries
/// - `<product>.nsi` script is generated in the output directory
/// - final installer is emitted as `<product>_<version>_<arch>-setup.exe`
///
/// Returns the path(s) to generated installer artifacts.
pub(crate) async fn bundle_project(ctx: &BundleContext<'_>) -> Result<Vec<PathBuf>> {
    let output_dir = ctx
        .project_out_directory()
        .join("bundle")
        .join(util::NSIS_OUTPUT_FOLDER_NAME);
    std::fs::create_dir_all(&output_dir).context("Failed to create NSIS output directory")?;

    let nsis_settings = ctx.windows().nsis.unwrap_or_default();
    let windows_settings = ctx.windows();

    // Get pre-resolved NSIS directory
    let nsis_dir = ctx
        .tools
        .nsis_dir
        .as_ref()
        .context("NSIS tools were not resolved. This is a bug.")?;
    let makensis = if cfg!(target_os = "windows") {
        nsis_dir.join("makensis.exe")
    } else {
        nsis_dir.join("makensis")
    };

    let arch = ctx.binary_arch();
    let arch_str = util::arch_to_windows_string(&arch);

    let product_name = ctx.product_name();
    let version = ctx.version_string();
    let installer_name = format!("{product_name}_{version}_{arch_str}-setup.exe");
    let output_path = output_dir.join(&installer_name);

    // Prepare the staging directory with all files to install
    let staging_dir = output_dir.join("_staging");
    if staging_dir.exists() {
        std::fs::remove_dir_all(&staging_dir)?;
    }
    std::fs::create_dir_all(&staging_dir)?;

    // Copy main binary to staging
    let main_binary_src = ctx.main_binary_path();
    let main_binary_name = if cfg!(target_os = "windows") || ctx.target().contains("windows") {
        format!("{}.exe", ctx.main_binary_name())
    } else {
        ctx.main_binary_name().to_string()
    };
    let main_binary_dest = staging_dir.join(&main_binary_name);
    std::fs::copy(&main_binary_src, &main_binary_dest).with_context(|| {
        format!(
            "Failed to copy main binary from {} to {}",
            main_binary_src.display(),
            main_binary_dest.display()
        )
    })?;

    // Copy resources to staging
    let resources_dir = staging_dir.join("resources");
    std::fs::create_dir_all(&resources_dir)?;
    ctx.copy_resources(&resources_dir)?;

    // Copy external binaries
    ctx.copy_external_binaries(&staging_dir)?;

    // Handle WebView2 installation mode
    let (install_webview, webview_install_code) =
        generate_webview_install_code(&windows_settings.webview_install_mode, ctx)?;

    // Build the template data
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
        JsonValue::String(ctx.short_description()),
    );
    data.insert(
        "bundle_id".to_string(),
        JsonValue::String(ctx.bundle_identifier()),
    );

    // Publisher
    let publisher = ctx
        .publisher()
        .map(|s| s.to_string())
        .or_else(|| ctx.authors_comma_separated())
        .unwrap_or_else(|| product_name.clone());
    data.insert("publisher".to_string(), JsonValue::String(publisher));

    // Copyright
    if let Some(copyright) = ctx.copyright_string() {
        data.insert(
            "copyright".to_string(),
            JsonValue::String(copyright.to_string()),
        );
    }

    // Install mode
    let install_mode = &nsis_settings.install_mode;
    data.insert(
        "install_mode_per_machine".to_string(),
        JsonValue::Bool(matches!(install_mode, NSISInstallerMode::PerMachine)),
    );
    data.insert(
        "install_mode_both".to_string(),
        JsonValue::Bool(matches!(install_mode, NSISInstallerMode::Both)),
    );

    // Start menu folder
    let start_menu_folder = nsis_settings
        .start_menu_folder
        .clone()
        .unwrap_or_else(|| product_name.clone());
    data.insert(
        "start_menu_folder".to_string(),
        JsonValue::String(start_menu_folder),
    );

    // Icons
    if let Some(icon) = &nsis_settings.installer_icon {
        let icon_path = ctx.crate_dir().join(icon);
        data.insert(
            "installer_icon".to_string(),
            JsonValue::String(icon_path.to_string_lossy().replace('/', "\\")),
        );
    }

    if let Some(header) = &nsis_settings.header_image {
        let header_path = ctx.crate_dir().join(header);
        data.insert(
            "header_image".to_string(),
            JsonValue::String(header_path.to_string_lossy().replace('/', "\\")),
        );
    }

    if let Some(sidebar) = &nsis_settings.sidebar_image {
        let sidebar_path = ctx.crate_dir().join(sidebar);
        data.insert(
            "sidebar_image".to_string(),
            JsonValue::String(sidebar_path.to_string_lossy().replace('/', "\\")),
        );
    }

    // License
    if let Some(license) = &nsis_settings.license {
        let license_path = ctx.crate_dir().join(license);
        data.insert(
            "license".to_string(),
            JsonValue::String(license_path.to_string_lossy().replace('/', "\\")),
        );
    }

    // Installer hooks
    if let Some(hooks) = &nsis_settings.installer_hooks {
        let hooks_path = ctx.crate_dir().join(hooks);
        data.insert(
            "installer_hooks".to_string(),
            JsonValue::String(hooks_path.to_string_lossy().replace('/', "\\")),
        );
    }

    // Additional languages
    if let Some(languages) = &nsis_settings.languages {
        let lang_values: Vec<JsonValue> = languages
            .iter()
            .filter(|l| l.as_str() != "English") // English is always included
            .map(|l| JsonValue::String(l.clone()))
            .collect();
        data.insert(
            "additional_languages".to_string(),
            JsonValue::Array(lang_values),
        );
    }

    // WebView2 installation
    data.insert(
        "install_webview".to_string(),
        JsonValue::Bool(install_webview),
    );
    data.insert(
        "webview_install_code".to_string(),
        JsonValue::String(webview_install_code),
    );

    // Resources list (simplified - just note that resources are in the resources subdir)
    data.insert("resources".to_string(), JsonValue::Array(Vec::new()));

    // Render the NSIS script
    let nsi_content = if let Some(custom_template) = &nsis_settings.template {
        let template_path = ctx.crate_dir().join(custom_template);
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

    // Write the .nsi script
    let nsi_path = output_dir.join(format!("{product_name}.nsi"));
    std::fs::write(&nsi_path, &nsi_content).context("Failed to write NSIS script")?;

    // Run makensis
    tracing::info!("Running makensis to build NSIS installer...");
    let mut cmd = Command::new(&makensis);

    // Use the NSIS Plugins directory from the downloaded NSIS
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

    // Sign the installer if configured
    if sign::can_sign(&windows_settings) {
        sign::try_sign(&output_path, ctx).await?;
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

/// Render a Handlebars template with the given data.
fn render_template(template: &str, data: &BTreeMap<String, JsonValue>) -> Result<String> {
    let mut hbs = Handlebars::new();
    hbs.set_strict_mode(false);

    // Disable HTML escaping since we're generating NSIS scripts, not HTML
    hbs.register_escape_fn(|s: &str| s.to_string());
    hbs.register_template_string("nsis", template)
        .context("Failed to parse NSIS template")?;
    hbs.render("nsis", data)
        .context("Failed to render NSIS template")
}

/// Generate the NSIS code snippet for WebView2 installation based on the install mode.
///
/// Returns (should_install, nsis_code).
fn generate_webview_install_code(
    mode: &WebviewInstallMode,
    ctx: &BundleContext,
) -> Result<(bool, String)> {
    match mode {
        WebviewInstallMode::Skip | WebviewInstallMode::FixedRuntime { .. } => {
            Ok((false, String::new()))
        }

        WebviewInstallMode::DownloadBootstrapper { silent }
        | WebviewInstallMode::EmbedBootstrapper { silent } => {
            let installer_path = ctx
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
            let installer_path = ctx
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

/// The embedded NSIS template script.
///
/// This is a simplified template that handles the core installation flow.
/// For advanced use cases, users can provide a custom template via NsisSettings.
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
    {{#each resources}}
    SetOutPath "$INSTDIR\{{this.target_dir}}"
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
