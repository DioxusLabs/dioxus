use super::{progress::ProgressTx, BuildArtifacts};
use crate::dioxus_crate::DioxusCrate;
use crate::Result;
use crate::{assets::AssetManifest, TraceSrc};
use crate::{build::BuildArgs, link::LinkAction};
use crate::{AppBundle, Platform};
use anyhow::Context;
use serde::Deserialize;
use std::{path::PathBuf, process::Stdio};
use tokio::{io::AsyncBufReadExt, process::Command};

#[derive(Clone, Debug)]
pub(crate) struct BuildRequest {
    /// The configuration for the crate we are building
    pub(crate) krate: DioxusCrate,

    /// The arguments for the build
    pub(crate) build: BuildArgs,

    /// Status channel to send our progress updates to
    pub(crate) progress: ProgressTx,

    /// The target directory for the build
    pub(crate) custom_target_dir: Option<PathBuf>,
}

impl BuildRequest {
    pub fn new(krate: DioxusCrate, build: BuildArgs, progress: ProgressTx) -> Self {
        Self {
            build,
            krate,
            progress,
            custom_target_dir: None,
        }
    }

    /// Run the build command with a pretty loader, returning the executable output location
    ///
    /// This will also run the fullstack build. Note that fullstack is handled separately within this
    /// code flow rather than outside of it.
    pub(crate) async fn build_all(self) -> Result<AppBundle> {
        tracing::debug!("Running build command...");

        let (app, server) =
            futures_util::future::try_join(self.build_app(), self.build_server()).await?;

        AppBundle::new(self, app, server).await
    }

    pub(crate) async fn build_app(&self) -> Result<BuildArtifacts> {
        tracing::debug!("Building app...");
        let exe = self.build_cargo().await?;
        let assets = self.collect_assets().await?;
        Ok(BuildArtifacts { exe, assets })
    }

    pub(crate) async fn build_server(&self) -> Result<Option<BuildArtifacts>> {
        tracing::debug!("Building server...");

        if !self.build.fullstack {
            return Ok(None);
        }

        let mut cloned = self.clone();
        cloned.build.platform = Some(Platform::Server);
        Ok(Some(cloned.build_app().await?))
    }

    /// Run `cargo`, returning the location of the final executable
    ///
    /// todo: add some stats here, like timing reports, crate-graph optimizations, etc
    pub(crate) async fn build_cargo(&self) -> Result<PathBuf> {
        tracing::debug!("Executing cargo...");

        // Extract the unit count of the crate graph so build_cargo has more accurate data
        let crate_count = self.get_unit_count_estimate().await;

        // Update the status to show that we're starting the build and how many crates we expect to build
        self.status_starting_build(crate_count);

        let mut cmd = Command::new("cargo");

        cmd.arg("rustc")
            .current_dir(self.krate.crate_dir())
            .arg("--message-format")
            .arg("json-diagnostic-rendered-ansi")
            .args(self.build_arguments());
        // .env("RUSTFLAGS", self.rust_flags());

        if let Some(target_dir) = self.custom_target_dir.as_ref() {
            cmd.env("CARGO_TARGET_DIR", target_dir);
        }

        // Android needs a special linker since the linker is actually tied to the android toolchain.
        // For the sake of simplicity, we're going to pass the linker here using ourselves as the linker,
        // but in reality we could simply use the android toolchain's linker as the path.
        //
        // We don't want to overwrite the user's .cargo/config.toml since that gets committed to git
        // and we want everyone's install to be the same.
        if self.build.platform() == Platform::Android {
            cmd.env(
                LinkAction::ENV_VAR_NAME,
                LinkAction::LinkAndroid {
                    linker: "/Users/jonkelley/Library/Android/sdk/ndk/25.2.9519653/toolchains/llvm/prebuilt/darwin-x86_64/bin/aarch64-linux-android24-clang".into(),
                    extra_flags: vec![],
                }
                .to_json(),
            );
        }

        tracing::trace!(dx_src = ?TraceSrc::Build, "Rust cargo args: {:?}", cmd);

        let mut child = cmd
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context("Failed to spawn cargo build")?;

        let stdout = tokio::io::BufReader::new(child.stdout.take().unwrap());
        let stderr = tokio::io::BufReader::new(child.stderr.take().unwrap());
        let mut output_location = None;
        let mut stdout = stdout.lines();
        let mut stderr = stderr.lines();
        let mut units_compiled = 0;

        loop {
            use cargo_metadata::Message;

            let line = tokio::select! {
                Ok(Some(line)) = stdout.next_line() => line,
                Ok(Some(line)) = stderr.next_line() => line,
                else => break,
            };

            let mut deserializer = serde_json::Deserializer::from_str(line.trim());
            deserializer.disable_recursion_limit();

            let message =
                Message::deserialize(&mut deserializer).unwrap_or(Message::TextLine(line));

            match message {
                Message::BuildScriptExecuted(_) => units_compiled += 1,
                Message::TextLine(line) => self.status_build_message(line),
                Message::CompilerMessage(msg) => self.status_build_diagnostic(msg),
                Message::CompilerArtifact(artifact) => {
                    units_compiled += 1;
                    match artifact.executable {
                        Some(executable) => output_location = Some(executable.into()),
                        None => self.status_build_progress(
                            units_compiled,
                            crate_count,
                            artifact.target.name,
                            self.build.platform(),
                        ),
                    }
                }
                Message::BuildFinished(finished) => {
                    if !finished.success {
                        return Err(anyhow::anyhow!(
                            "Cargo build failed, signaled by the compiler"
                        )
                        .into());
                    }
                }
                _ => {}
            }
        }

        if output_location.is_none() {
            tracing::error!("Cargo build failed - no output location");
        }

        let out_location = output_location.context("Build did not return an executable")?;

        tracing::debug!(
            "Build completed successfully - output location: {:?}",
            out_location
        );

        Ok(out_location)
    }

    /// Run the linker intercept and then fill in our AssetManifest from the incremental artifacts
    ///
    /// This will execute `dx` with an env var set to force `dx` to operate as a linker, and then
    /// traverse the .o and .rlib files rustc passes that new `dx` instance, collecting the link
    /// tables marked by manganis and parsing them as a ResourceAsset.
    pub(crate) async fn collect_assets(&self) -> anyhow::Result<AssetManifest> {
        tracing::debug!("Collecting assets ...");

        // If assets are skipped, we don't need to collect them
        if self.build.skip_assets {
            return Ok(AssetManifest::default());
        }

        // Create a temp file to put the output of the args
        // We need to do this since rustc won't actually print the link args to stdout, so we need to
        // give `dx` a file to dump its env::args into
        let tmp_file = tempfile::NamedTempFile::new()?;

        // Run `cargo rustc` again, but this time with a custom linker (dx) and an env var to force
        // `dx` to act as a linker
        //
        // This will force `dx` to look through the incremental cache and find the assets from the previous build
        Command::new("cargo")
            // .env("RUSTFLAGS", self.rust_flags())
            .arg("rustc")
            .args(self.build_arguments())
            .arg("--offline") /* don't use the network, should already be resolved */
            .arg("--")
            .arg(format!(
                "-Clinker={}",
                std::env::current_exe()
                    .unwrap()
                    .canonicalize()
                    .unwrap()
                    .display()
            ))
            .env(
                LinkAction::ENV_VAR_NAME,
                LinkAction::BuildAssetManifest {
                    destination: tmp_file.path().to_path_buf(),
                }
                .to_json(),
            )
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await?;

        // The linker wrote the manifest to the temp file, let's load it!
        AssetManifest::load_from_file(tmp_file.path())
    }

    /// Create a list of arguments for cargo builds
    pub(crate) fn build_arguments(&self) -> Vec<String> {
        let mut cargo_args = Vec::new();

        // Set the target, profile and features that vary between the app and server builds
        if self.build.platform() == Platform::Server {
            cargo_args.push("--profile".to_string());
            match self.build.release {
                true => cargo_args.push("release".to_string()),
                false => cargo_args.push(self.build.server_profile.to_string()),
            };
        } else {
            // Add required profile flags. --release overrides any custom profiles.
            let custom_profile = &self.build.profile.as_ref();
            if custom_profile.is_some() || self.build.release {
                cargo_args.push("--profile".to_string());
                match self.build.release {
                    true => cargo_args.push("release".to_string()),
                    false => {
                        cargo_args.push(
                            custom_profile
                                .expect("custom_profile should have been checked by is_some")
                                .to_string(),
                        );
                    }
                };
            }

            // todo: use the right arch based on the current arch
            let custom_target = match self.build.platform() {
                Platform::Web => Some("wasm32-unknown-unknown"),
                Platform::Ios => match self.build.target_args.device {
                    Some(true) => Some("aarch64-apple-ios"),
                    _ => Some("aarch64-apple-ios-sim"),
                },
                Platform::Android => Some("aarch64-linux-android"),
                Platform::Server => None,
                // we're assuming we're building for the native platform for now... if you're cross-compiling
                // the targets here might be different
                Platform::MacOS => None,
                Platform::Windows => None,
                Platform::Linux => None,
                Platform::Liveview => None,
            };

            if let Some(target) = custom_target.or(self.build.target_args.target.as_deref()) {
                cargo_args.push("--target".to_string());
                cargo_args.push(target.to_string());
            }
        }

        if self.build.verbose {
            cargo_args.push("--verbose".to_string());
        } else {
            cargo_args.push("--quiet".to_string());
        }

        if self.build.target_args.no_default_features {
            cargo_args.push("--no-default-features".to_string());
        }

        let features = self.target_features();

        if !features.is_empty() {
            cargo_args.push("--features".to_string());
            cargo_args.push(features.join(" "));
        }

        if let Some(ref package) = self.build.target_args.package {
            cargo_args.push(String::from("-p"));
            cargo_args.push(package.clone());
        }

        cargo_args.append(&mut self.build.cargo_args.clone());

        match self.krate.executable_type() {
            krates::cm::TargetKind::Bin => cargo_args.push("--bin".to_string()),
            krates::cm::TargetKind::Lib => cargo_args.push("--lib".to_string()),
            krates::cm::TargetKind::Example => cargo_args.push("--example".to_string()),
            _ => {}
        };

        cargo_args.push(self.krate.executable_name().to_string());

        tracing::debug!(dx_src = ?TraceSrc::Build, "cargo args: {:?}", cargo_args);

        cargo_args
    }

    #[allow(dead_code)]
    pub(crate) fn rust_flags(&self) -> String {
        let mut rust_flags = std::env::var("RUSTFLAGS").unwrap_or_default();

        if self.build.platform() == Platform::Android {
            let cur_exe = std::env::current_exe().unwrap();
            rust_flags.push_str(format!(" -Clinker={}", cur_exe.display()).as_str());
            rust_flags.push_str(" -Clink-arg=-landroid");
            rust_flags.push_str(" -Clink-arg=-llog");
            rust_flags.push_str(" -Clink-arg=-lOpenSLES");
            rust_flags.push_str(" -Clink-arg=-Wl,--export-dynamic");
        }

        rust_flags
    }

    /// Create the list of features we need to pass to cargo to build the app by merging together
    /// either the client or server features depending on if we're building a server or not.
    pub(crate) fn target_features(&self) -> Vec<String> {
        let mut features = self.build.target_args.features.clone();

        if self.build.platform() == Platform::Server {
            features.extend(self.build.target_args.server_features.clone());
        } else {
            features.extend(self.build.target_args.client_features.clone());
        }

        features
    }

    pub(crate) fn all_target_features(&self) -> Vec<String> {
        let mut features = self.target_features();

        if !self.build.target_args.no_default_features {
            features.extend(
                self.krate
                    .package()
                    .features
                    .get("default")
                    .cloned()
                    .unwrap_or_default(),
            );
        }

        features.dedup();

        features
    }

    /// Try to get the unit graph for the crate. This is a nightly only feature which may not be available with the current version of rustc the user has installed.
    pub(crate) async fn get_unit_count(&self) -> crate::Result<usize> {
        #[derive(Debug, Deserialize)]
        struct UnitGraph {
            units: Vec<serde_json::Value>,
        }

        let output = tokio::process::Command::new("cargo")
            .arg("+nightly")
            .arg("build")
            .arg("--unit-graph")
            .arg("-Z")
            .arg("unstable-options")
            .args(self.build_arguments())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await?;

        if !output.status.success() {
            return Err(anyhow::anyhow!("Failed to get unit count").into());
        }

        let output_text = String::from_utf8(output.stdout).context("Failed to get unit count")?;
        let graph: UnitGraph =
            serde_json::from_str(&output_text).context("Failed to get unit count")?;

        Ok(graph.units.len())
    }

    /// Get an estimate of the number of units in the crate. If nightly rustc is not available, this will return an estimate of the number of units in the crate based on cargo metadata.
    /// TODO: always use https://doc.rust-lang.org/nightly/cargo/reference/unstable.html#unit-graph once it is stable
    pub(crate) async fn get_unit_count_estimate(&self) -> usize {
        // Try to get it from nightly
        self.get_unit_count().await.unwrap_or_else(|_| {
            // Otherwise, use cargo metadata
            (self
                .krate
                .krates
                .krates_filtered(krates::DepKind::Dev)
                .iter()
                .map(|k| k.targets.len())
                .sum::<usize>() as f64
                / 3.5) as usize
        })
    }
}
