use crate::Result;
use anyhow::Context;
use std::path::PathBuf;
use tokio::process::Command;

#[derive(Debug, Default)]
pub struct RustupShow {
    pub default_host: String,
    pub rustup_home: PathBuf,
    pub installed_toolchains: Vec<String>,
    pub installed_targets: Vec<String>,
    pub active_rustc: String,
    pub active_toolchain: String,
}
impl RustupShow {
    /// Collect the output of `rustup show` and parse it
    pub async fn from_cli() -> Result<RustupShow> {
        let output = Command::new("rustup").args(["show"]).output().await?;
        let stdout =
            String::from_utf8(output.stdout).context("Failed to parse rustup show output")?;

        Ok(RustupShow::from_stdout(stdout))
    }

    /// Parse the output of `rustup show`
    pub fn from_stdout(output: String) -> RustupShow {
        // I apologize for this hand-rolled parser

        let mut result = RustupShow::default();
        let mut current_section = "";

        for line in output.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            if line.starts_with("Default host: ") {
                result.default_host = line.strip_prefix("Default host: ").unwrap().to_string();
            } else if line.starts_with("rustup home: ") {
                result.rustup_home =
                    PathBuf::from(line.strip_prefix("rustup home: ").unwrap().trim());
            } else if line == "installed toolchains" {
                current_section = "toolchains";
            } else if line == "installed targets for active toolchain" {
                current_section = "targets";
            } else if line == "active toolchain" {
                current_section = "active_toolchain";
            } else {
                if line.starts_with("---") || line.is_empty() {
                    continue;
                }
                match current_section {
                    "toolchains" => result
                        .installed_toolchains
                        .push(line.trim_end_matches(" (default)").to_string()),
                    "targets" => result.installed_targets.push(line.to_string()),
                    "active_toolchain" => {
                        if result.active_toolchain.is_empty() {
                            result.active_toolchain = line.to_string();
                        } else if line.starts_with("rustc ") {
                            result.active_rustc = line.to_string();
                        }
                    }
                    _ => {}
                }
            }
        }

        result
    }

    pub fn has_wasm32_unknown_unknown(&self) -> bool {
        self.installed_targets
            .contains(&"wasm32-unknown-unknown".to_string())
    }
}

#[test]
fn parses_rustup_show() {
    let output = r#"
Default host: aarch64-apple-darwin
rustup home:  /Users/jonkelley/.rustup

installed toolchains
--------------------

stable-aarch64-apple-darwin (default)
nightly-2021-07-06-aarch64-apple-darwin
nightly-2021-09-24-aarch64-apple-darwin
nightly-2022-03-10-aarch64-apple-darwin
nightly-2023-03-18-aarch64-apple-darwin
nightly-2024-01-11-aarch64-apple-darwin
nightly-aarch64-apple-darwin
1.58.1-aarch64-apple-darwin
1.60.0-aarch64-apple-darwin
1.68.2-aarch64-apple-darwin
1.69.0-aarch64-apple-darwin
1.71.1-aarch64-apple-darwin
1.72.1-aarch64-apple-darwin
1.73.0-aarch64-apple-darwin
1.74.1-aarch64-apple-darwin
1.77.2-aarch64-apple-darwin
1.78.0-aarch64-apple-darwin
1.79.0-aarch64-apple-darwin
1.49-aarch64-apple-darwin
1.55-aarch64-apple-darwin
1.56-aarch64-apple-darwin
1.57-aarch64-apple-darwin
1.66-aarch64-apple-darwin
1.69-aarch64-apple-darwin
1.70-aarch64-apple-darwin
1.74-aarch64-apple-darwin

installed targets for active toolchain
--------------------------------------

aarch64-apple-darwin
aarch64-apple-ios
aarch64-apple-ios-sim
aarch64-linux-android
aarch64-unknown-linux-gnu
armv7-linux-androideabi
i686-linux-android
thumbv6m-none-eabi
thumbv7em-none-eabihf
wasm32-unknown-unknown
x86_64-apple-darwin
x86_64-apple-ios
x86_64-linux-android
x86_64-pc-windows-msvc
x86_64-unknown-linux-gnu

active toolchain
----------------

stable-aarch64-apple-darwin (default)
rustc 1.79.0 (129f3b996 2024-06-10)
"#;
    let show = RustupShow::from_stdout(output.to_string());
    assert_eq!(show.default_host, "aarch64-apple-darwin");
    assert_eq!(show.rustup_home, PathBuf::from("/Users/jonkelley/.rustup"));
    assert_eq!(
        show.active_toolchain,
        "stable-aarch64-apple-darwin (default)"
    );
    assert_eq!(show.active_rustc, "rustc 1.79.0 (129f3b996 2024-06-10)");
    assert_eq!(show.installed_toolchains.len(), 26);
    assert_eq!(show.installed_targets.len(), 15);
    assert_eq!(
        show.installed_targets,
        vec![
            "aarch64-apple-darwin".to_string(),
            "aarch64-apple-ios".to_string(),
            "aarch64-apple-ios-sim".to_string(),
            "aarch64-linux-android".to_string(),
            "aarch64-unknown-linux-gnu".to_string(),
            "armv7-linux-androideabi".to_string(),
            "i686-linux-android".to_string(),
            "thumbv6m-none-eabi".to_string(),
            "thumbv7em-none-eabihf".to_string(),
            "wasm32-unknown-unknown".to_string(),
            "x86_64-apple-darwin".to_string(),
            "x86_64-apple-ios".to_string(),
            "x86_64-linux-android".to_string(),
            "x86_64-pc-windows-msvc".to_string(),
            "x86_64-unknown-linux-gnu".to_string(),
        ]
    )
}
