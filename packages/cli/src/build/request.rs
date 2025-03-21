use super::{progress::ProgressTx, AndroidTools, BuildArtifacts, PatchData};
use crate::{dioxus_crate::DioxusCrate, TargetArgs};
use crate::{link::LinkAction, BuildArgs};
use crate::{AppBundle, Platform, Result, TraceSrc};
use anyhow::Context;
use dioxus_cli_config::{APP_TITLE_ENV, ASSET_ROOT_ENV};
use dioxus_cli_opt::AssetManifest;
use itertools::Itertools;
use krates::{cm::TargetKind, Utf8PathBuf};
use serde::Deserialize;
use std::{
    path::{Path, PathBuf},
    process::Stdio,
    time::{Instant, SystemTime},
};
use target_lexicon::Triple;
use tokio::{io::AsyncBufReadExt, process::Command};
use uuid::Uuid;

/// This struct is used to plan the build process.
///
/// The point here is to be able to take in the user's config from the CLI without modifying the
/// arguments in place. Creating a buildplan "resolves" their config into a build plan that can be
/// introspected. For example, the users might not specify a "Triple" in the CLI but the triple will
/// be guaranteed to be resolved here.
///
/// Creating a buildplan also lets us introspect build requests and modularize our build process.
/// This will, however, lead to duplicate fields between the CLI and the build engine. This is fine
/// since we have the freedom to evolve the schema internally without breaking the API.
#[derive(Clone, Debug)]
pub(crate) struct BuildRequest {
    ///
    pub(crate) krate: DioxusCrate,

    // /
    pub(crate) fullstack: bool,

    pub(crate) profile: String,

    pub(crate) release: bool,

    ///
    pub(crate) platform: Platform,

    ///
    pub(crate) target: Triple,

    pub(crate) device: bool,

    /// Build for nightly [default: false]
    pub(crate) nightly: bool,

    /// The package to build
    pub(crate) package: Option<String>,

    /// Space separated list of features to activate
    pub(crate) features: Vec<String>,

    /// Extra arguments to pass to cargo
    pub(crate) cargo_args: Vec<String>,

    /// Don't include the default features in the build
    pub(crate) no_default_features: bool,

    /// The target directory for the build
    pub(crate) custom_target_dir: Option<PathBuf>,

    /// How we'll go about building
    pub(crate) mode: BuildMode,

    /// Status channel to send our progress updates to
    pub(crate) progress: ProgressTx,

    pub(crate) cranelift: bool,

    pub(crate) skip_assets: bool,

    pub(crate) ssg: bool,

    pub(crate) wasm_split: bool,

    pub(crate) debug_symbols: bool,

    pub(crate) inject_loading_scripts: bool,

    pub(crate) force_sequential: bool,
}

/// dx can produce different "modes" of a build. A "regular" build is a "base" build. The Fat and Thin
/// modes are used together to achieve binary patching and linking.
#[derive(Clone, Debug, PartialEq)]
pub enum BuildMode {
    /// A normal build generated using `cargo rustc`
    Base,

    /// A "Fat" build generated with cargo rustc and dx as a custom linker without -Wl,-dead-strip
    Fat,

    /// A "thin" build generated with `rustc` directly and dx as a custom linker
    Thin {
        direct_rustc: Vec<String>,
        changed_files: Vec<PathBuf>,
        aslr_reference: u64,
    },
}

impl BuildRequest {
    /// Create a new build request
    ///
    /// This will combine the many inputs here into a single source of truth. Fields will be duplicated
    /// from the inputs since various things might need to be autodetected.
    ///
    /// When creating a new build request we need to take into account
    /// - The user's command line arguments
    /// - The crate's Cargo.toml
    /// - The dioxus.tomkl
    /// - The user's CliSettings
    /// - The workspace
    /// - The host (android tools, installed frameworks, etc)
    /// - The intended platform
    ///
    /// We will attempt to autodetect a number of things if not provided.
    pub fn new(
        args: BuildArgs,
        krate: DioxusCrate,
        progress: ProgressTx,
        mode: BuildMode,
    ) -> Result<Self> {
        let default_platform = krate.default_platform();
        let mut features = vec![];
        let mut no_default_features = false;

        // The user passed --platform XYZ but already has `default = ["ABC"]` in their Cargo.toml
        // We want to strip out the default platform and use the one they passed, setting no-default-features
        if args.args.platform.is_some() && default_platform.is_some() {
            no_default_features = true;
            features.extend(krate.platformless_features());
        }

        // Inherit the platform from the args, or auto-detect it
        let platform = args.args
            .platform
            .map(|p| Some(p))
            .unwrap_or_else(|| krate.autodetect_platform().map(|a| a.0))
            .context("No platform was specified and could not be auto-detected. Please specify a platform with `--platform <platform>` or set a default platform using a cargo feature.")?;

        // Add any features required to turn on the client
        features.push(krate.feature_for_platform(platform));

        // Make sure we set the fullstack platform so we actually build the fullstack variant
        // Users need to enable "fullstack" in their default feature set.
        // todo(jon): fullstack *could* be a feature of the app, but right now we're assuming it's always enabled
        let fullstack = args.fullstack || krate.has_dioxus_feature("fullstack");

        // Set the profile of the build if it's not already set
        // This is mostly used for isolation of builds (preventing thrashing) but also useful to have multiple performance profiles
        // We might want to move some of these profiles into dioxus.toml and make them "virtual".
        let profile = match args.args.profile {
            Some(profile) => profile,
            None if args.args.release => "release".to_string(),
            None => match platform {
                Platform::Android => crate::dioxus_crate::PROFILE_ANDROID.to_string(),
                Platform::Web => crate::dioxus_crate::PROFILE_WASM.to_string(),
                Platform::Server => crate::dioxus_crate::PROFILE_SERVER.to_string(),
                _ => "dev".to_string(),
            },
        };

        let device = args.device.unwrap_or(false);

        // We want a real triple to build with, so we'll autodetect it if it's not provided
        // The triple ends up being a source of truth for us later hence this work to figure it out
        let target = match args.args.target {
            Some(target) => target,
            None => match platform {
                // Generally just use the host's triple for native executables
                Platform::MacOS => target_lexicon::HOST,
                Platform::Windows => target_lexicon::HOST,
                Platform::Linux => target_lexicon::HOST,
                Platform::Server => target_lexicon::HOST,
                Platform::Liveview => target_lexicon::HOST,
                Platform::Web => "wasm32-unknown-unknown".parse().unwrap(),

                // For iOS we should prefer the architecture for the simulator, but in lieu of actually
                // figuring that out, we'll assume aarch64 on m-series and x86_64 otherwise
                Platform::Ios => {
                    // use the host's architecture and sim if --device is passed
                    use target_lexicon::{Architecture, HOST};
                    match HOST.architecture {
                        Architecture::Aarch64(_) if device => "aarch64-apple-ios".parse().unwrap(),
                        Architecture::Aarch64(_) => "aarch64-apple-ios-sim".parse().unwrap(),
                        _ if device => "x86_64-apple-ios".parse().unwrap(),
                        _ => "x86_64-apple-ios-sim".parse().unwrap(),
                    }
                }

                // Same idea with android but we figure out the connected device using adb
                // for now we use
                Platform::Android => {
                    // use the host's architecture and sim if --device is passed
                    // use target_lexicon::{Architecture, HOST};
                    // match HOST.architecture {
                    //     Architecture::Aarch64(_) if device => {
                    //         "aarch64-linux-android".parse().unwrap()
                    //     }
                    //     Architecture::Aarch64(_) => "aarch64-linux-android".parse().unwrap(),
                    //     _ if device => "x86_64-linux-android".parse().unwrap(),
                    //     _ => "x86_64-linux-android".parse().unwrap(),
                    // }
                    //
                    // ... for known we don't know the architecture and we'll discover it during compilation
                    "aarch64-linux-android".parse().unwrap()
                    // "unknown-linux-android".parse().unwrap()
                }
            },
        };

        // Determine arch if android

        // if platform == Platform::Android && args.target_args.target.is_none() {
        //     tracing::debug!("No android arch provided, attempting to auto detect.");

        //     let arch = DioxusCrate::autodetect_android_arch().await;

        //     // Some extra logs
        //     let arch = match arch {
        //         Some(a) => {
        //             tracing::debug!(
        //                 "Autodetected `{}` Android arch.",
        //                 a.android_target_triplet()
        //             );
        //             a.to_owned()
        //         }
        //         None => {
        //             let a = Arch::default();
        //             tracing::debug!(
        //                 "Could not detect Android arch, defaulting to `{}`",
        //                 a.android_target_triplet()
        //             );
        //             a
        //         }
        //     };

        //     self.arch = Some(arch);
        // }

        Ok(Self {
            progress,
            mode,
            platform,
            features,
            no_default_features,
            krate,
            custom_target_dir: None,
            profile,
            fullstack,
            target,
            device,
            nightly: args.args.nightly,
            package: args.args.package,
            release: args.args.release,
            skip_assets: args.skip_assets,
            ssg: args.ssg,
            cranelift: args.cranelift,
            cargo_args: args.args.cargo_args,
            wasm_split: args.wasm_split,
            debug_symbols: args.debug_symbols,
            inject_loading_scripts: args.inject_loading_scripts,
            force_sequential: args.force_sequential,
        })
    }

    /// Run the build command with a pretty loader, returning the executable output location
    ///
    /// This will also run the fullstack build. Note that fullstack is handled separately within this
    /// code flow rather than outside of it.
    pub(crate) async fn build_all(self) -> Result<AppBundle> {
        tracing::debug!(
            "Running build command... {}",
            if self.force_sequential {
                "(sequentially)"
            } else {
                ""
            }
        );

        let (app, server) = match self.force_sequential {
            true => futures_util::future::try_join(self.cargo_build(), self.build_server()).await?,
            false => (self.cargo_build().await?, self.build_server().await?),
        };

        AppBundle::new(self, app, server).await
    }

    pub(crate) async fn build_server(&self) -> Result<Option<BuildArtifacts>> {
        tracing::debug!("Building server...");

        if !self.fullstack {
            return Ok(None);
        }

        let mut cloned = self.clone();
        cloned.platform = Platform::Server;

        Ok(Some(cloned.cargo_build().await?))
    }

    pub(crate) async fn cargo_build(&self) -> Result<BuildArtifacts> {
        let start = SystemTime::now();
        self.prepare_build_dir()?;

        tracing::debug!("Executing cargo...");

        let mut cmd = self.build_command()?;

        tracing::trace!(dx_src = ?TraceSrc::Build, "Rust cargo args: {:#?}", cmd);

        // Extract the unit count of the crate graph so build_cargo has more accurate data
        // "Thin" builds only build the final exe, so we only need to build one crate
        let crate_count = match self.mode {
            BuildMode::Thin { .. } => 1,
            _ => self.get_unit_count_estimate().await,
        };

        // Update the status to show that we're starting the build and how many crates we expect to build
        self.status_starting_build(crate_count);

        let mut child = cmd
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context("Failed to spawn cargo build")?;

        let stdout = tokio::io::BufReader::new(child.stdout.take().unwrap());
        let stderr = tokio::io::BufReader::new(child.stderr.take().unwrap());
        let mut output_location: Option<PathBuf> = None;
        let mut stdout = stdout.lines();
        let mut stderr = stderr.lines();
        let mut units_compiled = 0;
        let mut emitting_error = false;
        let mut direct_rustc = Vec::new();

        loop {
            use cargo_metadata::Message;

            let line = tokio::select! {
                Ok(Some(line)) = stdout.next_line() => line,
                Ok(Some(line)) = stderr.next_line() => line,
                else => break,
            };

            let Some(Ok(message)) = Message::parse_stream(std::io::Cursor::new(line)).next() else {
                continue;
            };

            match message {
                Message::BuildScriptExecuted(_) => units_compiled += 1,
                Message::TextLine(line) => {
                    // Try to extract the direct rustc args from the output
                    if line.trim().starts_with("Running ") {
                        // trim everyting but the contents between the quotes
                        let args = line
                            .trim()
                            .trim_start_matches("Running `")
                            .trim_end_matches('`');

                        // Parse these as shell words so we can get the direct rustc args
                        direct_rustc = shell_words::split(args).unwrap();
                    }

                    #[derive(Debug, Deserialize)]
                    struct RustcArtifact {
                        artifact: PathBuf,
                        emit: String,
                    }

                    if let Ok(artifact) = serde_json::from_str::<RustcArtifact>(&line) {
                        if artifact.emit == "link" {
                            output_location = Some(artifact.artifact);
                        }
                    }

                    // For whatever reason, if there's an error while building, we still receive the TextLine
                    // instead of an "error" message. However, the following messages *also* tend to
                    // be the error message, and don't start with "error:". So we'll check if we've already
                    // emitted an error message and if so, we'll emit all following messages as errors too.
                    if line.trim_start().starts_with("error:") {
                        emitting_error = true;
                    }

                    if emitting_error {
                        self.status_build_error(line);
                    } else {
                        self.status_build_message(line)
                    }
                }
                Message::CompilerMessage(msg) => self.status_build_diagnostic(msg),
                Message::CompilerArtifact(artifact) => {
                    units_compiled += 1;
                    match artifact.executable {
                        Some(executable) => output_location = Some(executable.into()),
                        None => self.status_build_progress(
                            units_compiled,
                            crate_count,
                            artifact.target.name,
                        ),
                    }
                }
                Message::BuildFinished(finished) => {
                    if !finished.success {
                        return Err(anyhow::anyhow!(
                            "Cargo build failed, signaled by the compiler. Toggle tracing mode (press `t`) for more information."
                        )
                        .into());
                    }
                }
                _ => {}
            }
        }

        if output_location.is_none() {
            tracing::error!("Cargo build failed - no output location. Toggle tracing mode (press `t`) for more information.");
        }

        let exe = output_location.context("Build did not return an executable")?;

        tracing::debug!("Build completed successfully - output location: {:?}", exe);

        Ok(BuildArtifacts {
            exe,
            direct_rustc,
            time_start: start,
            time_end: SystemTime::now(),
        })
    }

    #[tracing::instrument(
        skip(self),
        level = "trace",
        fields(dx_src = ?TraceSrc::Build)
    )]
    fn build_command(&self) -> Result<Command> {
        // Prefer using the direct rustc if we have it
        if let BuildMode::Thin { direct_rustc, .. } = &self.mode {
            tracing::debug!("Using direct rustc: {:?}", direct_rustc);
            if !direct_rustc.is_empty() {
                let mut cmd = Command::new(direct_rustc[0].clone());
                cmd.args(direct_rustc[1..].iter());
                cmd.envs(self.env_vars()?);
                cmd.current_dir(self.krate.workspace_dir());
                cmd.arg(format!(
                    "-Clinker={}",
                    dunce::canonicalize(std::env::current_exe().unwrap())
                        .unwrap()
                        .display()
                ));
                return Ok(cmd);
            }
        }

        // Otherwise build up the command using cargo rustc
        let mut cmd = Command::new("cargo");
        cmd.arg("rustc")
            .current_dir(self.krate.crate_dir())
            .arg("--message-format")
            .arg("json-diagnostic-rendered-ansi")
            .args(self.build_arguments())
            .envs(self.env_vars()?);
        Ok(cmd)
    }

    /// Create a list of arguments for cargo builds
    pub(crate) fn build_arguments(&self) -> Vec<String> {
        let mut cargo_args = Vec::new();

        // Add required profile flags. --release overrides any custom profiles.
        cargo_args.push("--profile".to_string());
        cargo_args.push(self.profile.to_string());

        // Pass the appropriate target to cargo. We *always* specify a target which is somewhat helpful for preventing thrashing
        cargo_args.push("--target".to_string());
        cargo_args.push(self.target.to_string());

        // We always run in verbose since the CLI itself is the one doing the presentation
        cargo_args.push("--verbose".to_string());

        if self.no_default_features {
            cargo_args.push("--no-default-features".to_string());
        }

        if !self.features.is_empty() {
            cargo_args.push("--features".to_string());
            cargo_args.push(self.features.join(" "));
        }

        // todo: maybe always set a package to reduce ambiguity?
        if let Some(package) = &self.package {
            cargo_args.push(String::from("-p"));
            cargo_args.push(package.clone());
        }

        match self.krate.executable_type() {
            krates::cm::TargetKind::Bin => cargo_args.push("--bin".to_string()),
            krates::cm::TargetKind::Lib => cargo_args.push("--lib".to_string()),
            krates::cm::TargetKind::Example => cargo_args.push("--example".to_string()),
            _ => {}
        };

        cargo_args.push(self.krate.executable_name().to_string());

        cargo_args.extend(self.cargo_args.clone());

        cargo_args.push("--".to_string());

        // the bundle splitter needs relocation data
        // we'll trim these out if we don't need them during the bundling process
        // todo(jon): for wasm binary patching we might want to leave these on all the time.
        if self.platform == Platform::Web && self.wasm_split {
            cargo_args.push("-Clink-args=--emit-relocs".to_string());
        }

        // dx *always* links android and thin builds
        if self.platform == Platform::Android || matches!(self.mode, BuildMode::Thin { .. }) {
            cargo_args.push(format!(
                "-Clinker={}",
                dunce::canonicalize(std::env::current_exe().unwrap())
                    .unwrap()
                    .display()
            ));
        }

        match self.mode {
            BuildMode::Base => {}
            BuildMode::Thin { .. } => {}
            BuildMode::Fat => {
                // This prevents rust from passing -dead_strip to the linker
                // todo: don't save temps here unless we become the linker for the base app
                cargo_args.extend_from_slice(&[
                    "-Csave-temps=true".to_string(),
                    "-Clink-dead-code".to_string(),
                ]);

                // args.extend([
                //     "-Wl,--whole-archive".to_string(),
                //     "-Wl,--no-gc-sections".to_string(),
                //     "-Wl,--export-all".to_string(),
                // ]);

                match self.platform {
                    // if macos/ios, -Wl,-all_load is required for the linker to work correctly
                    // macos uses ld64 but through the `cc` interface.a
                    Platform::MacOS | Platform::Ios => {
                        cargo_args.push("-Clink-args=-Wl,-all_load".to_string());
                    }

                    Platform::Android => {
                        cargo_args.push("-Clink-args=-Wl,--whole-archive".to_string());
                    }

                    // if linux -Wl,--whole-archive is required for the linker to work correctly
                    Platform::Linux => {
                        cargo_args.push("-Clink-args=-Wl,--whole-archive".to_string());
                    }

                    // if windows -Wl,--whole-archive is required for the linker to work correctly
                    // https://learn.microsoft.com/en-us/cpp/build/reference/wholearchive-include-all-library-object-files?view=msvc-170
                    Platform::Windows => {
                        // --export-dynamic
                        cargo_args.push(
                            "-Clink-args=-Wl,--whole-archive".to_string(),
                            // "-Clink-args=-Wl,--whole-archive,-Wl,--export-dynamic".to_string(),
                        );
                    }

                    // if web, -Wl,--whole-archive is required for the linker to work correctly.
                    // We also use --no-gc-sections and --export-all to push every symbol into the export table.
                    //
                    // rust uses its own wasm-ld linker which can be found here (it's just gcc-ld):
                    // /Users/jonkelley/.rustup/toolchains/stable-aarch64-apple-darwin/lib/rustlib/aarch64-apple-darwin/bin/gcc-ld
                    //
                    // export all should place things like env.memory into the export table so we can access them
                    // when loading the patches
                    Platform::Web => {
                        cargo_args.push(
                            "-Clink-args=-Wl,--whole-archive,-Wl,--no-gc-sections,-Wl,--export-all"
                                .to_string(),
                        );
                    }
                    _ => {}
                }
            }
        }

        tracing::debug!(dx_src = ?TraceSrc::Build, "cargo args: {:?}", cargo_args);

        cargo_args
    }

    pub(crate) fn all_target_features(&self) -> Vec<String> {
        let mut features = self.features.clone();

        if !self.no_default_features {
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
            .envs(self.env_vars()?)
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
        if let Ok(count) = self.get_unit_count().await {
            return count;
        }

        // Otherwise, use cargo metadata
        let units = self
            .krate
            .workspace
            .krates
            .krates_filtered(krates::DepKind::Dev)
            .iter()
            .map(|k| k.targets.len())
            .sum::<usize>();
        (units as f64 / 3.5) as usize
    }

    fn env_vars(&self) -> Result<Vec<(&str, String)>> {
        let mut env_vars = vec![];

        let mut custom_linker = None;

        // Make sure to set all the crazy android flags
        if self.platform == Platform::Android {
            let linker = self.build_android_env(&mut env_vars, true)?;

            // todo(jon): the guide for openssl recommends extending the path to include the tools dir
            //            in practice I couldn't get this to work, but this might eventually become useful.
            //
            // https://github.com/openssl/openssl/blob/master/NOTES-ANDROID.md#configuration
            //
            // They recommend a configuration like this:
            //
            // // export ANDROID_NDK_ROOT=/home/whoever/Android/android-sdk/ndk/20.0.5594570
            // PATH=$ANDROID_NDK_ROOT/toolchains/llvm/prebuilt/linux-x86_64/bin:$ANDROID_NDK_ROOT/toolchains/arm-linux-androideabi-4.9/prebuilt/linux-x86_64/bin:$PATH
            // ./Configure android-arm64 -D__ANDROID_API__=29
            // make
            //
            // let tools_dir = arch.android_tools_dir(&ndk);
            // let extended_path = format!(
            //     "{}:{}",
            //     tools_dir.display(),
            //     std::env::var("PATH").unwrap_or_default()
            // );
            // env_vars.push(("PATH", extended_path));

            // Also make sure to set the linker
            custom_linker = Some(linker);
        };

        match &self.mode {
            // We don't usually employ a custom linker for fat/base builds unless it's android
            // This might change in the future for "zero-linking"
            BuildMode::Base | BuildMode::Fat => {
                if let Some(linker) = custom_linker {
                    tracing::info!("Using custom linker for base link: {linker:?}");
                    env_vars.push((
                        LinkAction::ENV_VAR_NAME,
                        LinkAction::BaseLink {
                            linker,
                            extra_flags: vec![],
                        }
                        .to_json(),
                    ));
                }
            }

            // We use a custom linker here (dx) but it doesn't actually do anything
            BuildMode::Thin { .. } => {
                std::fs::create_dir_all(self.link_args_file().parent().unwrap());
                env_vars.push((
                    LinkAction::ENV_VAR_NAME,
                    LinkAction::ThinLink {
                        triple: self.target.clone(),
                        save_link_args: self.link_args_file(),
                    }
                    .to_json(),
                ))
            }
        }

        if let Some(target_dir) = self.custom_target_dir.as_ref() {
            env_vars.push(("CARGO_TARGET_DIR", target_dir.display().to_string()));
        }

        // If this is a release build, bake the base path and title
        // into the binary with env vars
        if self.release {
            if let Some(base_path) = &self.krate.config.web.app.base_path {
                env_vars.push((ASSET_ROOT_ENV, base_path.clone()));
            }
            env_vars.push((APP_TITLE_ENV, self.krate.config.web.app.title.clone()));
        }

        Ok(env_vars)
    }

    pub fn build_android_env(
        &self,
        env_vars: &mut Vec<(&str, String)>,
        rustf_flags: bool,
    ) -> Result<PathBuf, crate::Error> {
        let tools = crate::build::android_tools().context("Could not determine android tools")?;
        let linker = tools.android_cc(&self.target);
        let min_sdk_version = tools.min_sdk_version();
        let ar_path = tools.ar_path();
        let target_cc = tools.target_cc();
        let target_cxx = tools.target_cxx();
        let java_home = tools.java_home();
        let ndk = tools.ndk.clone();
        tracing::debug!(
            r#"Using android:
            min_sdk_version: {min_sdk_version}
            linker: {linker:?}
            ar_path: {ar_path:?}
            target_cc: {target_cc:?}
            target_cxx: {target_cxx:?}
            java_home: {java_home:?}
            "#
        );
        env_vars.push(("ANDROID_NATIVE_API_LEVEL", min_sdk_version.to_string()));
        env_vars.push(("TARGET_AR", ar_path.display().to_string()));
        env_vars.push(("TARGET_CC", target_cc.display().to_string()));
        env_vars.push(("TARGET_CXX", target_cxx.display().to_string()));
        env_vars.push(("ANDROID_NDK_ROOT", ndk.display().to_string()));
        if let Some(java_home) = java_home {
            tracing::debug!("Setting JAVA_HOME to {java_home:?}");
            env_vars.push(("JAVA_HOME", java_home.display().to_string()));
        }
        env_vars.push(("WRY_ANDROID_PACKAGE", "dev.dioxus.main".to_string()));
        env_vars.push(("WRY_ANDROID_LIBRARY", "dioxusmain".to_string()));
        env_vars.push((
            "WRY_ANDROID_KOTLIN_FILES_OUT_DIR",
            self.wry_android_kotlin_files_out_dir()
                .display()
                .to_string(),
        ));

        if rustf_flags {
            env_vars.push(("RUSTFLAGS", {
                let mut rust_flags = std::env::var("RUSTFLAGS").unwrap_or_default();

                // todo(jon): maybe we can make the symbol aliasing logic here instead of using llvm-objcopy
                if self.platform == Platform::Android {
                    let cur_exe = std::env::current_exe().unwrap();
                    rust_flags.push_str(format!(" -Clinker={}", cur_exe.display()).as_str());
                    rust_flags.push_str(" -Clink-arg=-landroid");
                    rust_flags.push_str(" -Clink-arg=-llog");
                    rust_flags.push_str(" -Clink-arg=-lOpenSLES");
                    rust_flags.push_str(" -Clink-arg=-Wl,--export-dynamic");
                }

                rust_flags
            }));
        }
        Ok(linker)
    }

    /// We only really currently care about:
    ///
    /// - app dir (.app, .exe, .apk, etc)
    /// - assetas dir
    /// - exe dir (.exe, .app, .apk, etc)
    /// - extra scaffolding
    ///
    /// It's not guaranteed that they're different from any other folder
    fn prepare_build_dir(&self) -> Result<()> {
        use once_cell::sync::OnceCell;
        use std::fs::{create_dir_all, remove_dir_all};

        static INITIALIZED: OnceCell<Result<()>> = OnceCell::new();

        let success = INITIALIZED.get_or_init(|| {
            _ = remove_dir_all(self.exe_dir());

            create_dir_all(self.root_dir())?;
            create_dir_all(self.exe_dir())?;
            create_dir_all(self.asset_dir())?;

            tracing::debug!("Initialized Root dir: {:?}", self.root_dir());
            tracing::debug!("Initialized Exe dir: {:?}", self.exe_dir());
            tracing::debug!("Initialized Asset dir: {:?}", self.asset_dir());

            // we could download the templates from somewhere (github?) but after having banged my head against
            // cargo-mobile2 for ages, I give up with that. We're literally just going to hardcode the templates
            // by writing them here.
            if let Platform::Android = self.platform {
                self.build_android_app_dir()?;
            }

            Ok(())
        });

        if let Err(e) = success.as_ref() {
            return Err(format!("Failed to initialize build directory: {e}").into());
        }

        Ok(())
    }

    pub fn incremental_cache_dir(&self) -> PathBuf {
        self.platform_dir().join("incremental-cache")
    }

    pub fn link_args_file(&self) -> PathBuf {
        self.incremental_cache_dir().join("link_args.txt")
    }

    /// The directory in which we'll put the main exe
    ///
    /// Mac, Android, Web are a little weird
    /// - mac wants to be in Contents/MacOS
    /// - android wants to be in jniLibs/arm64-v8a (or others, depending on the platform / architecture)
    /// - web wants to be in wasm (which... we don't really need to, we could just drop the wasm into public and it would work)
    ///
    /// I think all others are just in the root folder
    ///
    /// todo(jon): investigate if we need to put .wasm in `wasm`. It kinda leaks implementation details, which ideally we don't want to do.
    pub fn exe_dir(&self) -> PathBuf {
        match self.platform {
            Platform::MacOS => self.root_dir().join("Contents").join("MacOS"),
            Platform::Web => self.root_dir().join("wasm"),

            // Android has a whole build structure to it
            Platform::Android => self
                .root_dir()
                .join("app")
                .join("src")
                .join("main")
                .join("jniLibs")
                .join(AndroidTools::android_jnilib(&self.target)),

            // these are all the same, I think?
            Platform::Windows
            | Platform::Linux
            | Platform::Ios
            | Platform::Server
            | Platform::Liveview => self.root_dir(),
        }
    }

    /// Get the path to the wasm bindgen temporary output folder
    pub fn wasm_bindgen_out_dir(&self) -> PathBuf {
        self.root_dir().join("wasm")
    }

    /// Get the path to the wasm bindgen javascript output file
    pub fn wasm_bindgen_js_output_file(&self) -> PathBuf {
        self.wasm_bindgen_out_dir()
            .join(self.krate.executable_name())
            .with_extension("js")
    }

    /// Get the path to the wasm bindgen wasm output file
    pub fn wasm_bindgen_wasm_output_file(&self) -> PathBuf {
        self.wasm_bindgen_out_dir()
            .join(format!("{}_bg", self.krate.executable_name()))
            .with_extension("wasm")
    }

    /// returns the path to root build folder. This will be our working directory for the build.
    ///
    /// we only add an extension to the folders where it sorta matters that it's named with the extension.
    /// for example, on mac, the `.app` indicates we can `open` it and it pulls in icons, dylibs, etc.
    ///
    /// for our simulator-based platforms, this is less important since they need to be zipped up anyways
    /// to run in the simulator.
    ///
    /// For windows/linux, it's also not important since we're just running the exe directly out of the folder
    ///
    /// The idea of this folder is that we can run our top-level build command against it and we'll get
    /// a final build output somewhere. Some platforms have basically no build command, and can simply
    /// be ran by executing the exe directly.
    pub(crate) fn root_dir(&self) -> PathBuf {
        let platform_dir = self.platform_dir();

        match self.platform {
            Platform::Web => platform_dir.join("public"),
            Platform::Server => platform_dir.clone(), // ends up *next* to the public folder

            // These might not actually need to be called `.app` but it does let us run these with `open`
            Platform::MacOS => platform_dir.join(format!("{}.app", self.krate.bundled_app_name())),
            Platform::Ios => platform_dir.join(format!("{}.app", self.krate.bundled_app_name())),

            // in theory, these all could end up directly in the root dir
            Platform::Android => platform_dir.join("app"), // .apk (after bundling)
            Platform::Linux => platform_dir.join("app"),   // .appimage (after bundling)
            Platform::Windows => platform_dir.join("app"), // .exe (after bundling)
            Platform::Liveview => platform_dir.join("app"), // .exe (after bundling)
        }
    }

    pub(crate) fn platform_dir(&self) -> PathBuf {
        self.krate.build_dir(self.platform, self.release)
    }

    pub fn asset_dir(&self) -> PathBuf {
        match self.platform {
            Platform::MacOS => self
                .root_dir()
                .join("Contents")
                .join("Resources")
                .join("assets"),

            Platform::Android => self
                .root_dir()
                .join("app")
                .join("src")
                .join("main")
                .join("assets"),

            // everyone else is soooo normal, just app/assets :)
            Platform::Web
            | Platform::Ios
            | Platform::Windows
            | Platform::Linux
            | Platform::Server
            | Platform::Liveview => self.root_dir().join("assets"),
        }
    }

    /// Get the path to the asset optimizer version file
    pub fn asset_optimizer_version_file(&self) -> PathBuf {
        self.platform_dir().join(".cli-version")
    }

    pub fn platform_exe_name(&self) -> String {
        match self.platform {
            Platform::MacOS => self.krate.executable_name().to_string(),
            Platform::Ios => self.krate.executable_name().to_string(),
            Platform::Server => self.krate.executable_name().to_string(),
            Platform::Liveview => self.krate.executable_name().to_string(),
            Platform::Windows => format!("{}.exe", self.krate.executable_name()),

            // from the apk spec, the root exe is a shared library
            // we include the user's rust code as a shared library with a fixed namespacea
            Platform::Android => "libdioxusmain.so".to_string(),

            Platform::Web => unimplemented!("there's no main exe on web"), // this will be wrong, I think, but not important?

            // todo: maybe this should be called AppRun?
            Platform::Linux => self.krate.executable_name().to_string(),
        }
    }

    fn build_android_app_dir(&self) -> Result<()> {
        use std::fs::{create_dir_all, write};
        let root = self.root_dir();

        // gradle
        let wrapper = root.join("gradle").join("wrapper");
        create_dir_all(&wrapper)?;
        tracing::debug!("Initialized Gradle wrapper: {:?}", wrapper);

        // app
        let app = root.join("app");
        let app_main = app.join("src").join("main");
        let app_kotlin = app_main.join("kotlin");
        let app_jnilibs = app_main.join("jniLibs");
        let app_assets = app_main.join("assets");
        let app_kotlin_out = self.wry_android_kotlin_files_out_dir();
        create_dir_all(&app)?;
        create_dir_all(&app_main)?;
        create_dir_all(&app_kotlin)?;
        create_dir_all(&app_jnilibs)?;
        create_dir_all(&app_assets)?;
        create_dir_all(&app_kotlin_out)?;
        tracing::debug!("Initialized app: {:?}", app);
        tracing::debug!("Initialized app/src: {:?}", app_main);
        tracing::debug!("Initialized app/src/kotlin: {:?}", app_kotlin);
        tracing::debug!("Initialized app/src/jniLibs: {:?}", app_jnilibs);
        tracing::debug!("Initialized app/src/assets: {:?}", app_assets);
        tracing::debug!("Initialized app/src/kotlin/main: {:?}", app_kotlin_out);

        // handlerbars
        let hbs = handlebars::Handlebars::new();
        #[derive(serde::Serialize)]
        struct HbsTypes {
            application_id: String,
            app_name: String,
        }
        let hbs_data = HbsTypes {
            application_id: self.krate.full_mobile_app_name(),
            app_name: self.krate.bundled_app_name(),
        };

        // Top-level gradle config
        write(
            root.join("build.gradle.kts"),
            include_bytes!("../../assets/android/gen/build.gradle.kts"),
        )?;
        write(
            root.join("gradle.properties"),
            include_bytes!("../../assets/android/gen/gradle.properties"),
        )?;
        write(
            root.join("gradlew"),
            include_bytes!("../../assets/android/gen/gradlew"),
        )?;
        write(
            root.join("gradlew.bat"),
            include_bytes!("../../assets/android/gen/gradlew.bat"),
        )?;
        write(
            root.join("settings.gradle"),
            include_bytes!("../../assets/android/gen/settings.gradle"),
        )?;

        // Then the wrapper and its properties
        write(
            wrapper.join("gradle-wrapper.properties"),
            include_bytes!("../../assets/android/gen/gradle/wrapper/gradle-wrapper.properties"),
        )?;
        write(
            wrapper.join("gradle-wrapper.jar"),
            include_bytes!("../../assets/android/gen/gradle/wrapper/gradle-wrapper.jar"),
        )?;

        // Now the app directory
        write(
            app.join("build.gradle.kts"),
            hbs.render_template(
                include_str!("../../assets/android/gen/app/build.gradle.kts.hbs"),
                &hbs_data,
            )?,
        )?;
        write(
            app.join("proguard-rules.pro"),
            include_bytes!("../../assets/android/gen/app/proguard-rules.pro"),
        )?;
        write(
            app.join("src").join("main").join("AndroidManifest.xml"),
            hbs.render_template(
                include_str!("../../assets/android/gen/app/src/main/AndroidManifest.xml.hbs"),
                &hbs_data,
            )?,
        )?;

        // Write the main activity manually since tao dropped support for it
        write(
            self.wry_android_kotlin_files_out_dir()
                .join("MainActivity.kt"),
            hbs.render_template(
                include_str!("../../assets/android/MainActivity.kt.hbs"),
                &hbs_data,
            )?,
        )?;

        // Write the res folder
        let res = app_main.join("res");
        create_dir_all(&res)?;
        create_dir_all(res.join("values"))?;
        write(
            res.join("values").join("strings.xml"),
            hbs.render_template(
                include_str!("../../assets/android/gen/app/src/main/res/values/strings.xml.hbs"),
                &hbs_data,
            )?,
        )?;
        write(
            res.join("values").join("colors.xml"),
            include_bytes!("../../assets/android/gen/app/src/main/res/values/colors.xml"),
        )?;
        write(
            res.join("values").join("styles.xml"),
            include_bytes!("../../assets/android/gen/app/src/main/res/values/styles.xml"),
        )?;

        create_dir_all(res.join("drawable"))?;
        write(
            res.join("drawable").join("ic_launcher_background.xml"),
            include_bytes!(
                "../../assets/android/gen/app/src/main/res/drawable/ic_launcher_background.xml"
            ),
        )?;
        create_dir_all(res.join("drawable-v24"))?;
        write(
            res.join("drawable-v24").join("ic_launcher_foreground.xml"),
            include_bytes!(
                "../../assets/android/gen/app/src/main/res/drawable-v24/ic_launcher_foreground.xml"
            ),
        )?;
        create_dir_all(res.join("mipmap-anydpi-v26"))?;
        write(
            res.join("mipmap-anydpi-v26").join("ic_launcher.xml"),
            include_bytes!(
                "../../assets/android/gen/app/src/main/res/mipmap-anydpi-v26/ic_launcher.xml"
            ),
        )?;
        create_dir_all(res.join("mipmap-hdpi"))?;
        write(
            res.join("mipmap-hdpi").join("ic_launcher.webp"),
            include_bytes!(
                "../../assets/android/gen/app/src/main/res/mipmap-hdpi/ic_launcher.webp"
            ),
        )?;
        create_dir_all(res.join("mipmap-mdpi"))?;
        write(
            res.join("mipmap-mdpi").join("ic_launcher.webp"),
            include_bytes!(
                "../../assets/android/gen/app/src/main/res/mipmap-mdpi/ic_launcher.webp"
            ),
        )?;
        create_dir_all(res.join("mipmap-xhdpi"))?;
        write(
            res.join("mipmap-xhdpi").join("ic_launcher.webp"),
            include_bytes!(
                "../../assets/android/gen/app/src/main/res/mipmap-xhdpi/ic_launcher.webp"
            ),
        )?;
        create_dir_all(res.join("mipmap-xxhdpi"))?;
        write(
            res.join("mipmap-xxhdpi").join("ic_launcher.webp"),
            include_bytes!(
                "../../assets/android/gen/app/src/main/res/mipmap-xxhdpi/ic_launcher.webp"
            ),
        )?;
        create_dir_all(res.join("mipmap-xxxhdpi"))?;
        write(
            res.join("mipmap-xxxhdpi").join("ic_launcher.webp"),
            include_bytes!(
                "../../assets/android/gen/app/src/main/res/mipmap-xxxhdpi/ic_launcher.webp"
            ),
        )?;

        Ok(())
    }

    pub(crate) fn wry_android_kotlin_files_out_dir(&self) -> PathBuf {
        let mut kotlin_dir = self
            .root_dir()
            .join("app")
            .join("src")
            .join("main")
            .join("kotlin");

        for segment in "dev.dioxus.main".split('.') {
            kotlin_dir = kotlin_dir.join(segment);
        }

        tracing::debug!("app_kotlin_out: {:?}", kotlin_dir);

        kotlin_dir
    }

    pub(crate) fn is_patch(&self) -> bool {
        matches!(&self.mode, BuildMode::Thin { .. })
    }

    // pub(crate) fn triple(&self) -> Triple {
    //     match self.platform {
    //         Platform::MacOS => Triple::from_str("aarc64-apple-darwin").unwrap(),
    //         Platform::Windows => Triple::from_str("x86_64-pc-windows-msvc").unwrap(),
    //         Platform::Linux => Triple::from_str("x86_64-unknown-linux-gnu").unwrap(),
    //         Platform::Web => Triple::from_str("wasm32-unknown-unknown").unwrap(),
    //         Platform::Ios => Triple::from_str("aarch64-apple-ios-sim").unwrap(),
    //         Platform::Android => Triple::from_str("aarch64-linux-android").unwrap(),
    //         Platform::Server => Triple::from_str("aarc64-apple-darwin").unwrap(),
    //         // Platform::Server => Triple::from_str("x86_64-unknown-linux-gnu").unwrap(),
    //         Platform::Liveview => Triple::from_str("aarc64-apple-darwin").unwrap(),
    //     }
    // }
}

// pub(crate) async fn autodetect_android_arch() -> Option<Triple> {
//     // Try auto detecting arch through adb.
//     static AUTO_ARCH: OnceCell<Option<Triple>> = OnceCell::new();

//     match AUTO_ARCH.get() {
//         Some(a) => *a,
//         None => {
//             // TODO: Wire this up with --device flag. (add `-s serial`` flag before `shell` arg)
//             let output = Command::new("adb")
//                 .arg("shell")
//                 .arg("uname")
//                 .arg("-m")
//                 .output()
//                 .await;

//             let out = match output {
//                 Ok(o) => o,
//                 Err(e) => {
//                     tracing::debug!("ADB command failed: {:?}", e);
//                     return None;
//                 }
//             };

//             // Parse ADB output
//             let Ok(out) = String::from_utf8(out.stdout) else {
//                 tracing::debug!("ADB returned unexpected data.");
//                 return None;
//             };
//             let trimmed = out.trim().to_string();
//             tracing::trace!("ADB Returned: `{trimmed:?}`");

//             // Set the cell
//             let arch = match trimmed.as_str() {
//                 "armv7l" => Ok(Self::Arm),
//                 "aarch64" => Ok(Self::Arm64),
//                 "i386" => Ok(Self::X86),
//                 "x86_64" => Ok(Self::X64),
//                 _ => Err(()),
//             };
//             AUTO_ARCH
//                 .set(arch)
//                 .expect("the cell should have been checked empty by the match condition");

//             arch
//         }
//     }
// }

// impl std::fmt::Display for Arch {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         match self {
//             Arch::Arm => "armv7l",
//             Arch::Arm64 => "aarch64",
//             Arch::X86 => "i386",
//             Arch::X64 => "x86_64",
//         }
//         .fmt(f)
//     }
// }
