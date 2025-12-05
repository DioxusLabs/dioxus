use super::*;
use crate::{Result, Workspace};
use anyhow::{bail, Context};
use itertools::Itertools;

/// Perform a system analysis to verify the system install is working correctly.
#[derive(Clone, Debug, Parser)]
pub(crate) struct Doctor {}

impl Doctor {
    pub async fn doctor(self) -> Result<StructuredOutput> {
        let mut rustc_version = "not found".to_string();
        let mut rustc_sysroot = "not found".to_string();
        let mut rustlib = PathBuf::from(".");
        if let Ok(r) = Workspace::get_rustc_sysroot().await {
            rustlib = PathBuf::from(r.as_str()).join("lib").join("rustlib");
            rustc_sysroot = r;
        }
        if let Ok(r) = Workspace::get_rustc_version().await {
            rustc_version = r;
        }

        // wasm-opt
        let wasm_opt_location = crate::wasm_opt::installed_location();
        let wasm_opt_message = match wasm_opt_location.clone() {
            Some(path) => path.to_string_lossy().to_string(),
            None => "not installed".into(),
        };

        // wasm-bindgen
        let mut wbg_version_msg = "automatically managed".to_string();
        let mut wasm_bindgen_location = "automatically managed".to_string();
        if let Ok(workspace) = Workspace::current().await {
            let wbg_version = workspace.wasm_bindgen_version();
            if let Some(vers) = &wbg_version {
                wbg_version_msg = vers.to_string();

                wasm_bindgen_location =
                    match crate::wasm_bindgen::WasmBindgen::new(vers).get_binary_path() {
                        Ok(path) => path.to_string_lossy().to_string(),
                        Err(err) => err.to_string().lines().join(""),
                    };
            }
        }

        // extensions
        fn has_dioxus_ext(editor_dir: &str) -> anyhow::Result<PathBuf> {
            let home = dirs::home_dir().context("no home dir")?;
            let exts = home.join(editor_dir).join("extensions");
            for dir in exts.read_dir()?.flatten() {
                if dir
                    .file_name()
                    .to_string_lossy()
                    .contains("dioxuslabs.dioxus-")
                {
                    return Ok(dir.path());
                }
            }

            bail!("not found")
        }

        // Editors
        let vscode_ext = has_dioxus_ext(".vscode");
        let vscode_ext_msg = match vscode_ext.as_ref() {
            Ok(path) => path.to_string_lossy().to_string(),
            Err(_) => "not found".to_string(),
        };
        let vscode_insiders_ext = has_dioxus_ext(".vscode-insiders");
        let vscode_insiders_ext_msg = match vscode_insiders_ext.as_ref() {
            Ok(path) => path.to_string_lossy().to_string(),
            Err(_) => "not found".to_string(),
        };
        let cursor_ext = has_dioxus_ext(".cursor");
        let cursor_ext_msg = match cursor_ext.as_ref() {
            Ok(path) => path.to_string_lossy().to_string(),
            Err(_) => "not found".to_string(),
        };

        // Tailwind
        let mut tailwindcss = "not found".to_string();
        if let Ok(path) = crate::tailwind::TailwindCli::v3().get_binary_path() {
            tailwindcss = path.display().to_string();
        }
        if let Ok(path) = crate::tailwind::TailwindCli::v4().get_binary_path() {
            tailwindcss = path.display().to_string();
        }

        let mut adb = "not found".to_string();
        let mut ndk = "not found".to_string();
        let mut sdk = "not found".to_string();
        let mut java_home = "not found".to_string();
        let mut emulator = "not found".to_string();
        if let Some(rf) = crate::build::get_android_tools() {
            if rf.adb.exists() {
                adb = rf.adb.display().to_string();
            }
            if rf.ndk.exists() {
                ndk = rf.ndk.display().to_string();
            }
            if let Some(jh) = rf.java_home.as_ref() {
                java_home = jh.display().to_string();
            }
            if rf.sdk().exists() {
                sdk = rf.sdk().display().to_string();
            }
            if let Some(jh) = rf.java_home.as_ref() {
                java_home = jh.display().to_string();
            }
            if rf.emulator().exists() {
                emulator = rf.emulator().display().to_string();
            }
        };

        let mut simulator_location = "not found".to_string();
        let mut xcode_install = "not found".to_string();
        if let Some(xcode) = Workspace::get_xcode_path().await {
            let sim_location = xcode.join("Applications").join("Simulator.app");
            if sim_location.exists() {
                simulator_location = sim_location.display().to_string();
            }
            if xcode.exists() {
                xcode_install = xcode.display().to_string();
            }
        }

        let mut security_cli_path = "not found".to_string();
        let mut codesign_path = "not found".to_string();
        let mut xcode_select_path = "not found".to_string();
        let mut xcrun_path = "not found".to_string();
        let mut ranlib_path = "not found".to_string();
        if let Ok(path) = which::which("security") {
            security_cli_path = path.display().to_string();
        }
        if let Ok(path) = which::which("codesign") {
            codesign_path = path.display().to_string();
        }
        if let Ok(path) = which::which("xcode-select") {
            xcode_select_path = path.display().to_string();
        }
        if let Ok(path) = which::which("xcrun") {
            xcrun_path = path.display().to_string();
        }
        if let Some(path) = Workspace::select_ranlib() {
            ranlib_path = path.display().to_string();
        }

        // toolchains
        let mut has_wasm32_unknown_unknown = "❌";
        let mut has_aarch64_linux_android = "❌";
        let mut has_i686_linux_android = "❌";
        let mut has_armv7_linux_androideabi = "❌";
        let mut has_x86_64_linux_android = "❌";
        let mut has_x86_64_apple_ios = "❌";
        let mut has_aarch64_apple_ios = "❌";
        let mut has_aarch64_apple_ios_sim = "❌";
        let mut has_aarch64_apple_darwin = "❌";
        if rustlib.join("wasm32-unknown-unknown").exists() {
            has_wasm32_unknown_unknown = "✅";
        }
        if rustlib.join("aarch64-linux-android").exists() {
            has_aarch64_linux_android = "✅";
        }
        if rustlib.join("i686-linux-android").exists() {
            has_i686_linux_android = "✅";
        }
        if rustlib.join("armv7-linux-androideabi").exists() {
            has_armv7_linux_androideabi = "✅";
        }
        if rustlib.join("x86_64-linux-android").exists() {
            has_x86_64_linux_android = "✅";
        }
        if rustlib.join("x86_64-apple-ios").exists() {
            has_x86_64_apple_ios = "✅";
        }
        if rustlib.join("aarch64-apple-ios").exists() {
            has_aarch64_apple_ios = "✅";
        }
        if rustlib.join("aarch64-apple-ios-sim").exists() {
            has_aarch64_apple_ios_sim = "✅";
        }
        if rustlib.join("aarch64-apple-darwin").exists() {
            has_aarch64_apple_darwin = "✅";
        }

        // Rust tool paths
        let mut rustc_path = "not found".to_string();
        let mut cargo_path = "not found".to_string();
        let mut cc_path = "not found".to_string();
        if let Ok(path) = which::which("rustc") {
            rustc_path = path.display().to_string();
        }
        if let Ok(path) = which::which("cargo") {
            cargo_path = path.display().to_string();
        }
        if let Ok(path) = which::which("cc") {
            cc_path = path.display().to_string();
        }

        // Things to know
        // - current rust version and rust-related things
        // - installed toolchains
        // -
        use crate::styles::*;
        println!(
            r#"{LINK_STYLE}Setup{LINK_STYLE:#}
 {GLOW_STYLE}Web{GLOW_STYLE:#}: wasm-bindgen, wasm-opt, and TailwindCSS are downloaded automatically
 {GLOW_STYLE}iOS{GLOW_STYLE:#}: Install iOS SDK and developer tools and through XCode
 {GLOW_STYLE}Android{GLOW_STYLE:#}: Install Android Studio, NDK, and then set ANDROID_HOME and ANDROID_NDK_HOME
 {GLOW_STYLE}macOS{GLOW_STYLE:#}: all tools should be installed by default
 {GLOW_STYLE}Windows{GLOW_STYLE:#}: install the webview2 binary
 {GLOW_STYLE}Linux{GLOW_STYLE:#}: Install libwebkit2gtk-4.1-dev libgtk-3-dev libasound2-dev libudev-dev libayatana-appindicator3-dev libxdo-dev libglib2.0-dev
 {GLOW_STYLE}nix{GLOW_STYLE:#}: Make sure all tools are in your path (codesign, ld, etc.)

{LINK_STYLE}Rust{LINK_STYLE:#}
 Rustc version: {HINT_STYLE}{rustc_version}{HINT_STYLE:#}
 Rustc sysroot: {HINT_STYLE}{rustc_sysroot}{HINT_STYLE:#}
 Rustc path: {HINT_STYLE}{rustc_path}{HINT_STYLE:#}
 Cargo path: {HINT_STYLE}{cargo_path}{HINT_STYLE:#}
 cc path: {HINT_STYLE}{cc_path}{HINT_STYLE:#}

{LINK_STYLE}Devtools{LINK_STYLE:#}
 VSCode Extension: {HINT_STYLE}{vscode_ext_msg}{HINT_STYLE:#}
 VSCode-Insiders Extension: {HINT_STYLE}{vscode_insiders_ext_msg}{HINT_STYLE:#}
 Cursor Extension: {HINT_STYLE}{cursor_ext_msg}{HINT_STYLE:#}
 TailwindCSS: {HINT_STYLE}{tailwindcss}{HINT_STYLE:#}

{LINK_STYLE}Web{LINK_STYLE:#}
 wasm-opt: {HINT_STYLE}{wasm_opt_message}{HINT_STYLE:#}
 wasm-bindgen: {HINT_STYLE}{wasm_bindgen_location}{HINT_STYLE:#}
 wasm-bindgen version: {HINT_STYLE}{wbg_version_msg}{HINT_STYLE:#}

{LINK_STYLE}iOS/macOS{LINK_STYLE:#}
 XCode: {HINT_STYLE}{xcode_install}{HINT_STYLE:#}
 Simulator: {HINT_STYLE}{simulator_location}{HINT_STYLE:#}
 Security CLI: {HINT_STYLE}{security_cli_path}{HINT_STYLE:#}
 Codesign CII: {HINT_STYLE}{codesign_path}{HINT_STYLE:#}
 xcode-select: {HINT_STYLE}{xcode_select_path}{HINT_STYLE:#}
 xcrun: {HINT_STYLE}{xcrun_path}{HINT_STYLE:#}
 ranlib: {HINT_STYLE}{ranlib_path}{HINT_STYLE:#}

{LINK_STYLE}Android{LINK_STYLE:#}
 sdk: {HINT_STYLE}{sdk}{HINT_STYLE:#}
 ndk: {HINT_STYLE}{ndk}{HINT_STYLE:#}
 adb: {HINT_STYLE}{adb}{HINT_STYLE:#}
 emulator: {HINT_STYLE}{emulator}{HINT_STYLE:#}
 java_home: {HINT_STYLE}{java_home}{HINT_STYLE:#}

{LINK_STYLE}Toolchains{LINK_STYLE:#}
 {HINT_STYLE}{has_wasm32_unknown_unknown}{HINT_STYLE:#} wasm32-unknown-unknown {HINT_STYLE}(web){HINT_STYLE:#}
 {HINT_STYLE}{has_aarch64_linux_android}{HINT_STYLE:#} aarch64-linux-android {HINT_STYLE}(android){HINT_STYLE:#}
 {HINT_STYLE}{has_i686_linux_android}{HINT_STYLE:#} i686-linux-android {HINT_STYLE}(android){HINT_STYLE:#}
 {HINT_STYLE}{has_armv7_linux_androideabi}{HINT_STYLE:#} armv7-linux-androideabi {HINT_STYLE}(android){HINT_STYLE:#}
 {HINT_STYLE}{has_x86_64_linux_android}{HINT_STYLE:#} x86_64-linux-android {HINT_STYLE}(android){HINT_STYLE:#}
 {HINT_STYLE}{has_x86_64_apple_ios}{HINT_STYLE:#} x86_64-apple-ios {HINT_STYLE}(iOS){HINT_STYLE:#}
 {HINT_STYLE}{has_aarch64_apple_ios}{HINT_STYLE:#} aarch64-apple-ios {HINT_STYLE}(iOS){HINT_STYLE:#}
 {HINT_STYLE}{has_aarch64_apple_ios_sim}{HINT_STYLE:#} aarch64-apple-ios-sim {HINT_STYLE}(iOS){HINT_STYLE:#}
 {HINT_STYLE}{has_aarch64_apple_darwin}{HINT_STYLE:#} aarch64-apple-darwin {HINT_STYLE}(iOS){HINT_STYLE:#}

Get help: {LINK_STYLE}https://discord.gg/XgGxMSkvUM{LINK_STYLE:#}
More info: {LINK_STYLE}https://dioxuslabs.com/learn/0.7/{LINK_STYLE:#}
"#
        );

        Ok(StructuredOutput::Success)
    }
}
