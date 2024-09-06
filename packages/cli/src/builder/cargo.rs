use super::BuildRequest;
use crate::{assets::AssetManifest, link::LINK_OUTPUT_ENV_VAR};
use crate::{builder::Platform, bundler::AppBundle};
use crate::{link::InterceptedArgs, Result};
use anyhow::Context;
use serde::Deserialize;
use std::{path::PathBuf, process::Stdio};
use tokio::{io::AsyncBufReadExt, process::Command};

impl BuildRequest {
    pub(crate) async fn build(self) -> Result<AppBundle> {
        tracing::info!("🚅 Running build command...");

        // Install any tooling that might be required for this build.
        self.verify_tooling().await?;

        // Run the build command with a pretty loader, returning the executable output location
        let executable = self.build_cargo().await?;

        // Extract out the asset manifest from the executable using our linker tricks
        let assets = self.collect_assets().await?;

        // Assemble a bundle from everything
        AppBundle::new(self, assets, executable).await
    }

    pub(crate) async fn verify_tooling(&self) -> Result<()> {
        match self.platform() {
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
    pub(crate) async fn build_cargo(&self) -> anyhow::Result<PathBuf> {
        // Extract the unit count of the crate graph so build_cargo has more accurate data
        let crate_count = self.get_unit_count_estimate().await;

        self.status_starting_build();

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
            .args(&self.build_arguments())
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
        let mut errors = Vec::new();

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
                Message::CompilerMessage(msg) => {
                    let message = msg.message;
                    self.status_build_diagnostic(&message);
                    const WARNING_LEVELS: &[cargo_metadata::diagnostic::DiagnosticLevel] = &[
                        cargo_metadata::diagnostic::DiagnosticLevel::Help,
                        cargo_metadata::diagnostic::DiagnosticLevel::Note,
                        cargo_metadata::diagnostic::DiagnosticLevel::Warning,
                        cargo_metadata::diagnostic::DiagnosticLevel::Error,
                        cargo_metadata::diagnostic::DiagnosticLevel::FailureNote,
                        cargo_metadata::diagnostic::DiagnosticLevel::Ice,
                    ];
                    const FATAL_LEVELS: &[cargo_metadata::diagnostic::DiagnosticLevel] = &[
                        cargo_metadata::diagnostic::DiagnosticLevel::Error,
                        cargo_metadata::diagnostic::DiagnosticLevel::FailureNote,
                        cargo_metadata::diagnostic::DiagnosticLevel::Ice,
                    ];
                    if WARNING_LEVELS.contains(&message.level) {
                        if let Some(rendered) = message.rendered {
                            errors.push(rendered);
                        }
                    }
                    if FATAL_LEVELS.contains(&message.level) {
                        return Err(anyhow::anyhow!(errors.join("\n")));
                    }
                }
                Message::CompilerArtifact(artifact) => {
                    units_compiled += 1;
                    match artifact.executable {
                        Some(executable) => output_location = Some(executable.into()),
                        None => {
                            self.status_build_progress(units_compiled as f64 / crate_count as f64)
                        }
                    }
                }
                Message::BuildFinished(finished) => {
                    if !finished.success {
                        return Err(anyhow::anyhow!("Build failed"));
                    }
                }
                _ => {}
            }
        }

        output_location.context("Build did not return an executable")
    }

    /// Run the linker intercept and then fill in our AssetManifest from the incremental artifacts
    ///
    /// This will execute `dx` with an env var set to force `dx` to operate as a linker, and then
    /// traverse the .o and .rlib files rustc passes that new `dx` instance, collecting the link
    /// tables marked by manganis and parsing them as a ResourceAsset.
    pub(crate) async fn collect_assets(&self) -> anyhow::Result<AssetManifest> {
        // If this is the server build, the client build already copied any assets we need
        if self.platform() == Platform::Server {
            return Ok(AssetManifest::default());
        }

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
            .args(self.build_arguments())
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
    pub(crate) fn build_arguments(&self) -> Vec<String> {
        let mut cargo_args = Vec::new();

        if self.build.release {
            cargo_args.push("--release".to_string());
        }
        if self.build.verbose {
            cargo_args.push("--verbose".to_string());
        } else {
            cargo_args.push("--quiet".to_string());
        }

        if let Some(custom_profile) = &self.build.profile {
            cargo_args.push("--profile".to_string());
            cargo_args.push(custom_profile.to_string());
        }

        if !self.build.target_args.features.is_empty() {
            let features_str = self.build.target_args.features.join(" ");
            cargo_args.push("--features".to_string());
            cargo_args.push(features_str);
        }

        if let Some(target) = self
            .targeting_web()
            .then_some("wasm32-unknown-unknown")
            .or(self.build.target_args.target.as_deref())
        {
            cargo_args.push("--target".to_string());
            cargo_args.push(target.to_string());
        }

        if let Some(ref platform) = self.build.target_args.package {
            cargo_args.push(String::from("-p"));
            cargo_args.push(platform.clone());
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

        cargo_args
    }
}
