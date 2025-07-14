use super::*;
use crate::{Result, Workspace};
use anyhow::{bail, Context};
use itertools::Itertools;

/// Perform a system analysis to verify the system install is working correctly.
#[derive(Clone, Debug, Parser)]
pub(crate) struct Doctor {}

impl Doctor {
    pub async fn doctor(self) -> Result<StructuredOutput> {
        let Ok(workspace) = Workspace::current().await else {
            bail!("dx doctor must be run within a cargo workspace!")
        };

        let rustc_version = &workspace.rustc_version;
        let rustc_sysroot = &workspace.sysroot.to_string_lossy();
        let rustlib = workspace.sysroot.join("lib").join("rustlib");

        // wasm-opt
        let wasm_opt_location = crate::wasm_opt::installed_location();
        let wasm_opt_message = match wasm_opt_location.clone() {
            Some(path) => path.to_string_lossy().to_string(),
            None => "not installed".into(),
        };

        // wasm-bindgen
        let wbg_version = workspace.wasm_bindgen_version();
        let wbg_version_msg = match wbg_version.clone() {
            Some(msg) => msg,
            None => "not required".to_string(),
        };
        let wasm_bindgen_location = match wbg_version {
            Some(vers) => match crate::wasm_bindgen::WasmBindgen::new(&vers).get_binary_path() {
                Ok(path) => path.to_string_lossy().to_string(),
                Err(err) => err.to_string().lines().join(""),
            },
            None => "not required".to_string(),
        };

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
        if let Some(rf) = workspace.android_tools.as_deref() {
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
        if let Some(xcode) = workspace.xcode.as_ref() {
            let sim_location = xcode.join("Applications").join("Simulator.app");
            if sim_location.exists() {
                simulator_location = sim_location.display().to_string();
            }
            if xcode.exists() {
                xcode_install = xcode.display().to_string();
            }
        }

        // toolchains
        let mut has_wasm32_unknown_unknown = "❌";
        let mut has_aarch64_android_linux = "❌";
        let mut has_i686_linux_android = "❌";
        let mut has_armv7_linux_androideabi = "❌";
        let mut has_x86_64_android_linux = "❌";
        let mut has_x86_64_apple_ios = "❌";
        let mut has_aarch64_apple_ios = "❌";
        let mut has_aarch64_apple_ios_sim = "❌";
        let mut has_aarch64_apple_darwin = "❌";
        if rustlib.join("wasm32-unknown-unknown").exists() {
            has_wasm32_unknown_unknown = "✅";
        }
        if rustlib.join("aarch64-linux-android").exists() {
            has_aarch64_android_linux = "✅";
        }
        if rustlib.join("i686-linux-android").exists() {
            has_i686_linux_android = "✅";
        }
        if rustlib.join("armv7-linux-androideabi").exists() {
            has_armv7_linux_androideabi = "✅";
        }
        if rustlib.join("x86_64-linux-android").exists() {
            has_x86_64_android_linux = "✅";
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

{LINK_STYLE}Rust{LINK_STYLE:#}
 Rustc version: {HINT_STYLE}{rustc_version}{HINT_STYLE:#}
 Rustc sysroot: {HINT_STYLE}{rustc_sysroot}{HINT_STYLE:#}

{LINK_STYLE}Devtools{LINK_STYLE:#}
 VSCode Extension: {HINT_STYLE}{vscode_ext_msg}{HINT_STYLE:#}
 Cursor Extension: {HINT_STYLE}{cursor_ext_msg}{HINT_STYLE:#}
 TailwindCSS: {HINT_STYLE}{tailwindcss}{HINT_STYLE:#}

{LINK_STYLE}Web{LINK_STYLE:#}
 wasm-opt: {HINT_STYLE}{wasm_opt_message}{HINT_STYLE:#}
 wasm-bindgen: {HINT_STYLE}{wasm_bindgen_location}{HINT_STYLE:#}
 wasm-bindgen version: {HINT_STYLE}{wbg_version_msg}{HINT_STYLE:#}

{LINK_STYLE}iOS{LINK_STYLE:#}
 XCode: {HINT_STYLE}{xcode_install}{HINT_STYLE:#}
 Simulator: {HINT_STYLE}{simulator_location}{HINT_STYLE:#}

{LINK_STYLE}Android{LINK_STYLE:#}
 sdk: {HINT_STYLE}{sdk}{HINT_STYLE:#}
 ndk: {HINT_STYLE}{ndk}{HINT_STYLE:#}
 adb: {HINT_STYLE}{adb}{HINT_STYLE:#}
 emulator: {HINT_STYLE}{emulator}{HINT_STYLE:#}
 java_home: {HINT_STYLE}{java_home}{HINT_STYLE:#}

{LINK_STYLE}Toolchains{LINK_STYLE:#}
 {HINT_STYLE}{has_wasm32_unknown_unknown}{HINT_STYLE:#} wasm32-unknown-unknown {HINT_STYLE}(web){HINT_STYLE:#}
 {HINT_STYLE}{has_aarch64_android_linux}{HINT_STYLE:#} aarch64-android-linux {HINT_STYLE}(android){HINT_STYLE:#}
 {HINT_STYLE}{has_i686_linux_android}{HINT_STYLE:#} i686-linux-android {HINT_STYLE}(android){HINT_STYLE:#}
 {HINT_STYLE}{has_armv7_linux_androideabi}{HINT_STYLE:#} armv7-linux-androideabi {HINT_STYLE}(android){HINT_STYLE:#}
 {HINT_STYLE}{has_x86_64_android_linux}{HINT_STYLE:#} x86_64-android-linux {HINT_STYLE}(android){HINT_STYLE:#}
 {HINT_STYLE}{has_x86_64_apple_ios}{HINT_STYLE:#} x86_64-apple-ios {HINT_STYLE}(iOS){HINT_STYLE:#}
 {HINT_STYLE}{has_aarch64_apple_ios}{HINT_STYLE:#} aarch64-apple-ios {HINT_STYLE}(iOS){HINT_STYLE:#}
 {HINT_STYLE}{has_aarch64_apple_ios_sim}{HINT_STYLE:#} aarch64-apple-ios-sim {HINT_STYLE}(iOS){HINT_STYLE:#}
 {HINT_STYLE}{has_aarch64_apple_darwin}{HINT_STYLE:#} aarch64-apple-darwin {HINT_STYLE}(iOS){HINT_STYLE:#}

Get help: {LINK_STYLE}https://discord.gg/XgGxMSkvUM{LINK_STYLE:#}
More info: {LINK_STYLE}https://dioxuslabs.com/learn/0.6/{LINK_STYLE:#}
"#
        );

        Ok(StructuredOutput::Success)
    }
}
