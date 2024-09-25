use super::progress::ProgressTx;
use crate::build::BuildArgs;
use crate::bundler::AppBundle;
use crate::dioxus_crate::DioxusCrate;
use crate::Platform;
use crate::{assets::AssetManifest, link::LINK_OUTPUT_ENV_VAR, TraceSrc};
use crate::{link::InterceptedArgs, Result};
use anyhow::Context;
use serde::Deserialize;
use std::{path::PathBuf, process::Stdio};
use tokio::{io::AsyncBufReadExt, process::Command};

/// An app that's built, bundled, processed, and a handle to its running app, if it exists
///
/// As the build progresses, we'll fill in fields like assets, executable, entitlements, etc
///
/// This combines both the app and its potential server in one go, since we do end up bundling them
/// together in the end anyways. We can also track progress for both in one spot, which is better
/// than trying to aggregate them after the builds have finished.
///
/// If the app needs to be bundled, we'll add the bundle info here too
#[derive(Clone, Debug)]
pub(crate) struct BuildRequest {
    /// The configuration for the crate we are building
    pub(crate) krate: DioxusCrate,

    /// The arguments for the build
    pub(crate) build: BuildArgs,

    /// Status channel to send our progress updates to
    pub(crate) progress: ProgressTx,

    /// The rustc flags to pass to the build
    pub(crate) rust_flags: Vec<String>,

    /// The target directory for the build
    pub(crate) custom_target_dir: Option<PathBuf>,
}

impl BuildRequest {
    pub fn new(krate: DioxusCrate, build: BuildArgs, progress: ProgressTx) -> Self {
        Self {
            build,
            krate,
            progress,
            rust_flags: Default::default(),
            custom_target_dir: None,
        }
    }

    /// The final output name of the app, primarly to be used when bundled
    ///
    /// Needs to be very disambiguated
    /// Eg: my-app-web-macos-x86_64.app
    /// {app_name}-{platform}-{arch}
    ///
    /// Does not include the extension
    pub(crate) fn app_name(&self) -> String {
        match self.build.platform() {
            Platform::Web => "web".to_string(),
            Platform::Server => "server".to_string(),
            Platform::Desktop => todo!(),
            Platform::Ios => todo!(),
            Platform::Android => todo!(),
            Platform::Liveview => todo!(),
        }
    }

    /// Run the build command with a pretty loader, returning the executable output location
    ///
    /// This will also run the fullstack build. Note that fullstack is handled separately within this
    /// code flow rather than outside of it.
    pub(crate) async fn build(self) -> Result<AppBundle> {
        tracing::debug!("Running build command...");

        // Install any tooling that might be required for this build.
        self.verify_tooling().await?;

        // Run both the app and server builds in parallel
        // We currently don't create a manifest for the server, so all assets belong to the client.
        //
        // Run the build command with a pretty loader, returning the executable output location
        // Then, extract out the asset manifest from the executable using our linker tricks
        //
        // todo(jon): we should probably create a manifest for the server too
        let ((app_exe, app_assets), server_exe) = futures_util::future::try_join(
            async {
                let app_exe = self.build_cargo(false).await?;
                let app_assets = self.collect_assets().await?;
                Ok((app_exe, app_assets))
            },
            async {
                if self.build.fullstack {
                    self.build_cargo(true).await.map(Some)
                } else {
                    Ok(None)
                }
            },
        )
        .await?;

        // Assemble a bundle from everything
        AppBundle::new(self, app_assets, app_exe, server_exe).await
    }

    pub(crate) async fn verify_tooling(&self) -> Result<()> {
        tracing::debug!("Verifying tooling...");

        self.krate.initialize_profiles()?;

        match self.build.platform() {
            // If this is a web, build make sure we have the web build tooling set up
            Platform::Web => {}

            // Make sure we have mobile tooling if need be
            Platform::Ios => {}
            Platform::Android => {}

            // Make sure we have the required deps for desktop. More important for linux
            Platform::Desktop => {}

            // Generally nothing for the server, pretty simple
            Platform::Server => {}
            Platform::Liveview => {}
        }

        Ok(())
    }

    /// Run `cargo`, returning the location of the final exectuable
    ///
    /// todo: add some stats here, like timing reports, crate-graph optimizations, etc
    pub(crate) async fn build_cargo(&self, server: bool) -> Result<PathBuf> {
        tracing::debug!("Executing cargo...");

        // Extract the unit count of the crate graph so build_cargo has more accurate data
        let crate_count = self.get_unit_count_estimate(server).await;

        // Update the status to show that we're starting the build and how many crates we expect to build
        self.status_starting_build(server, crate_count);

        let mut child = Command::new("cargo")
            .arg("rustc")
            .envs(
                self.custom_target_dir
                    .as_ref()
                    .map(|dir| ("CARGO_TARGET_DIR", dir)),
            )
            .current_dir(self.krate.crate_dir())
            .arg("--message-format")
            .arg("json-diagnostic-rendered-ansi")
            .args(&self.build_arguments(server))
            .arg("--")
            .args(self.rust_flags.clone())
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
                            server,
                        ),
                    }
                }
                Message::BuildFinished(finished) => {
                    if !finished.success {
                        return Err(anyhow::anyhow!("Cargo build failed.").into());
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
        // Pass in the tmp_file as the env var itself
        //
        // NOTE: that -Csave-temps=y is needed to prevent rustc from deleting the incremental cache...
        // This might not be a "stable" way of keeping artifacts around, but it's in stable rustc, so we use it
        Command::new("cargo")
            .arg("rustc")
            .args(self.build_arguments(false))
            .arg("--offline") /* don't use the network, should already be resolved */
            .arg("--")
            .arg(format!("-Clinker={}", std::env::current_exe().unwrap().display())) /* pass ourselves in */
            .env(LINK_OUTPUT_ENV_VAR, tmp_file.path()) /* but with the env var pointing to the temp file */
            .arg("-Csave-temps=y") /* don't delete the incremental cache */
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await?;

        // Read the contents of the temp file
        let args =
            std::fs::read_to_string(tmp_file.path()).context("Failed to read linker output")?;

        // Parse them as a Vec<String> which is just our informal format for link args in the cli
        // Todo: this might be wrong-ish on windows? The format is weird
        let args = serde_json::from_str::<InterceptedArgs>(&args)
            .context("Failed to parse linker output")?;

        Ok(AssetManifest::new_from_linker_intercept(args))
    }

    /// Create a list of arguments for cargo builds
    pub(crate) fn build_arguments(&self, server: bool) -> Vec<String> {
        let mut cargo_args = Vec::new();

        // Set the target, profile and features that vary between the app and server builds
        if server {
            if let Some(custom_profile) = &self.build.server_profile {
                cargo_args.push("--profile".to_string());
                cargo_args.push(custom_profile.to_string());
            }
        } else {
            if let Some(custom_profile) = &self.build.profile {
                cargo_args.push("--profile".to_string());
                cargo_args.push(custom_profile.to_string());
            }

            if let Some(target) = self
                .targeting_web()
                .then_some("wasm32-unknown-unknown")
                .or(self.build.target_args.target.as_deref())
            {
                cargo_args.push("--target".to_string());
                cargo_args.push(target.to_string());
            }
        }

        if self.build.release {
            cargo_args.push("--release".to_string());
        }

        if self.build.verbose {
            cargo_args.push("--verbose".to_string());
        } else {
            cargo_args.push("--quiet".to_string());
        }

        let features = self.target_features(server);

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
            krates::cm::TargetKind::Bin => {
                cargo_args.push("--bin".to_string());
            }
            krates::cm::TargetKind::Lib => {
                cargo_args.push("--lib".to_string());
            }
            krates::cm::TargetKind::Example => {
                cargo_args.push("--example".to_string());
            }
            _ => {}
        };

        cargo_args.push(self.krate.executable_name().to_string());

        tracing::debug!(dx_src = ?TraceSrc::Build, "cargo args: {:?}", cargo_args);

        cargo_args
    }

    pub(crate) fn target_features(&self, server: bool) -> Vec<String> {
        let mut features = self.build.target_args.features.clone();

        if server {
            features.extend(self.build.target_args.server_features.clone());
        } else {
            features.extend(self.build.target_args.client_features.clone());
        }
        features
    }
}
