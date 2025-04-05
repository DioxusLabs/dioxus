//! ## Web:
//! Create a folder that is somewhat similar to an app-image (exe + asset)
//! The server is dropped into the `web` folder, even if there's no `public` folder.
//! If there's no server (SPA), we still use the `web` folder, but it only contains the
//! public folder.
//! ```
//! web/
//!     server
//!     assets/
//!     public/
//!         index.html
//!         wasm/
//!            app.wasm
//!            glue.js
//!            snippets/
//!                ...
//!         assets/
//!            logo.png
//! ```
//!
//! ## Linux:
//! https://docs.appimage.org/reference/appdir.html#ref-appdir
//! current_exe.join("Assets")
//! ```
//! app.appimage/
//!     AppRun
//!     app.desktop
//!     package.json
//!     assets/
//!         logo.png
//! ```
//!
//! ## Macos
//! We simply use the macos format where binaries are in `Contents/MacOS` and assets are in `Contents/Resources`
//! We put assets in an assets dir such that it generally matches every other platform and we can
//! output `/assets/blah` from manganis.
//! ```
//! App.app/
//!     Contents/
//!         Info.plist
//!         MacOS/
//!             Frameworks/
//!         Resources/
//!             assets/
//!                 blah.icns
//!                 blah.png
//!         CodeResources
//!         _CodeSignature/
//! ```
//!
//! ## iOS
//! Not the same as mac! ios apps are a bit "flattened" in comparison. simpler format, presumably
//! since most ios apps don't ship frameworks/plugins and such.
//!
//! todo(jon): include the signing and entitlements in this format diagram.
//! ```
//! App.app/
//!     main
//!     assets/
//! ```
//!
//! ## Android:
//!
//! Currently we need to generate a `src` type structure, not a pre-packaged apk structure, since
//! we need to compile kotlin and java. This pushes us into using gradle and following a structure
//! similar to that of cargo mobile2. Eventually I'd like to slim this down (drop buildSrc) and
//! drive the kotlin build ourselves. This would let us drop gradle (yay! no plugins!) but requires
//! us to manage dependencies (like kotlinc) ourselves (yuck!).
//!
//! https://github.com/WanghongLin/miscellaneous/blob/master/tools/build-apk-manually.sh
//!
//! Unfortunately, it seems that while we can drop the `android` build plugin, we still will need
//! gradle since kotlin is basically gradle-only.
//!
//! Pre-build:
//! ```
//! app.apk/
//!     .gradle
//!     app/
//!         src/
//!             main/
//!                 assets/
//!                 jniLibs/
//!                 java/
//!                 kotlin/
//!                 res/
//!                 AndroidManifest.xml
//!             build.gradle.kts
//!             proguard-rules.pro
//!         buildSrc/
//!             build.gradle.kts
//!             src/
//!                 main/
//!                     kotlin/
//!                          BuildTask.kt
//!     build.gradle.kts
//!     gradle.properties
//!     gradlew
//!     gradlew.bat
//!     settings.gradle
//! ```
//!
//! Final build:
//! ```
//! app.apk/
//!   AndroidManifest.xml
//!   classes.dex
//!   assets/
//!       logo.png
//!   lib/
//!       armeabi-v7a/
//!           libmyapp.so
//!       arm64-v8a/
//!           libmyapp.so
//! ```
//! Notice that we *could* feasibly build this ourselves :)
//!
//! ## Windows:
//! https://superuser.com/questions/749447/creating-a-single-file-executable-from-a-directory-in-windows
//! Windows does not provide an AppImage format, so instead we're going build the same folder
//! structure as an AppImage, but when distributing, we'll create a .exe that embeds the resources
//! as an embedded .zip file. When the app runs, it will implicitly unzip its resources into the
//! Program Files folder. Any subsequent launches of the parent .exe will simply call the AppRun.exe
//! entrypoint in the associated Program Files folder.
//!
//! This is, in essence, the same as an installer, so we might eventually just support something like msi/msix
//! which functionally do the same thing but with a sleeker UI.
//!
//! This means no installers are required and we can bake an updater into the host exe.
//!
//! ## Handling asset lookups:
//! current_exe.join("assets")
//! ```
//! app.appimage/
//!     main.exe
//!     main.desktop
//!     package.json
//!     assets/
//!         logo.png
//! ```
//!
//! Since we support just a few locations, we could just search for the first that exists
//! - usr
//! - ../Resources
//! - assets
//! - Assets
//! - $cwd/assets
//!
//! ```
//! assets::root() ->
//!     mac -> ../Resources/
//!     ios -> ../Resources/
//!     android -> assets/
//!     server -> assets/
//!     liveview -> assets/
//!     web -> /assets/
//! root().join(bundled)
//! ```
//!
//!
//! Every dioxus app can have an optional server executable which will influence the final bundle.
//! This is built in parallel with the app executable during the `build` phase and the progres/status
//! of the build is aggregated.
//!
//! The server will *always* be dropped into the `web` folder since it is considered "web" in nature,
//! and will likely need to be combined with the public dir to be useful.
//!
//! We do our best to assemble read-to-go bundles here, such that the "bundle" step for each platform
//! can just use the build dir
//!
//! When we write the AppBundle to a folder, it'll contain each bundle for each platform under the app's name:
//! ```
//! dog-app/
//!   build/
//!       web/
//!         server.exe
//!         assets/
//!           some-secret-asset.txt (a server-side asset)
//!         public/
//!           index.html
//!           assets/
//!             logo.png
//!       desktop/
//!          App.app
//!          App.appimage
//!          App.exe
//!          server/
//!              server
//!              assets/
//!                some-secret-asset.txt (a server-side asset)
//!       ios/
//!          App.app
//!          App.ipa
//!       android/
//!          App.apk
//!   bundle/
//!       build.json
//!       Desktop.app
//!       Mobile_x64.ipa
//!       Mobile_arm64.ipa
//!       Mobile_rosetta.ipa
//!       web.appimage
//!       web/
//!         server.exe
//!         assets/
//!             some-secret-asset.txt
//!         public/
//!             index.html
//!             assets/
//!                 logo.png
//!                 style.css
//! ```
//!
//! When deploying, the build.json file will provide all the metadata that dx-deploy will use to
//! push the app to stores, set up infra, manage versions, etc.
//!
//! The format of each build will follow the name plus some metadata such that when distributing you
//! can easily trim off the metadata.
//!
//! The idea here is that we can run any of the programs in the same way that they're deployed.
//!
//!
//! ## Bundle structure links
//! - apple: https://developer.apple.com/documentation/bundleresources/placing_content_in_a_bundle
//! - appimage: https://docs.appimage.org/packaging-guide/manual.html#ref-manual
//!
//! ## Extra links
//! - xbuild: https://github.com/rust-mobile/xbuild/blob/master/xbuild/src/command/build.rs

use super::{AndroidTools, BuildContext, PatchData};
use crate::{
    BuildArgs, DioxusConfig, LinkAction, Platform, ProgressTx, Result, TraceSrc, WasmOptConfig,
    Workspace,
};
use anyhow::Context;
use dioxus_cli_config::{APP_TITLE_ENV, ASSET_ROOT_ENV};
use dioxus_cli_opt::{process_file_to, AssetManifest};
use itertools::Itertools;
use krates::{cm::TargetKind, KrateDetails, Krates, NodeId, Utf8PathBuf};
use manganis::{AssetOptions, JsAssetOptions};
use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};
use serde::Deserialize;
use std::{
    collections::HashSet,
    future::Future,
    io::Write,
    path::{Path, PathBuf},
    pin::Pin,
    process::Stdio,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::{Instant, SystemTime, UNIX_EPOCH},
};
use target_lexicon::{
    Aarch64Architecture, Architecture, ArmArchitecture, BinaryFormat, Environment, OperatingSystem,
    Triple, Vendor, X86_32Architecture,
};
use tokio::{io::AsyncBufReadExt, process::Command};
use toml_edit::Item;
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
///
/// Since we resolve the build request before initializing the CLI, it also serves as a place to store
/// resolved "serve" arguments, which is why it takes ServeArgs instead of BuildArgs. Simply wrap the
/// BuildArgs in a default ServeArgs and pass it in.
///
/// All updates from the build will be sent on a global "BuildProgress" channel.
#[derive(Clone)]
pub(crate) struct BuildRequest {
    pub(crate) workspace: Arc<Workspace>,
    pub(crate) config: DioxusConfig,
    pub(crate) crate_package: NodeId,
    pub(crate) crate_target: krates::cm::Target,

    pub(crate) profile: String,

    pub(crate) release: bool,

    ///
    pub(crate) platform: Platform,

    ///
    pub(crate) triple: Triple,

    pub(crate) device: bool,

    /// Build for nightly [default: false]
    pub(crate) nightly: bool,

    /// The package to build
    pub(crate) cargo_package: String,

    /// Space separated list of features to activate
    pub(crate) features: Vec<String>,

    /// Extra arguments to pass to cargo
    pub(crate) cargo_args: Vec<String>,

    /// Don't include the default features in the build
    pub(crate) no_default_features: bool,

    /// The target directory for the build
    pub(crate) custom_target_dir: Option<PathBuf>,

    pub(crate) cranelift: bool,

    pub(crate) skip_assets: bool,

    pub(crate) wasm_split: bool,

    pub(crate) debug_symbols: bool,

    pub(crate) inject_loading_scripts: bool,
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

/// The end result of a build.
///
/// Contains the final asset manifest, the executable, and metadata about the build.
/// Note that the `exe` might be stale and/or overwritten by the time you read it!
pub struct BuildArtifacts {
    pub(crate) exe: PathBuf,
    pub(crate) direct_rustc: Vec<String>,
    pub(crate) time_start: SystemTime,
    pub(crate) time_end: SystemTime,
    pub(crate) assets: AssetManifest,
    pub(crate) mode: BuildMode,
}

pub(crate) static PROFILE_WASM: &str = "wasm-dev";
pub(crate) static PROFILE_ANDROID: &str = "android-dev";
pub(crate) static PROFILE_SERVER: &str = "server-dev";

impl BuildRequest {
    /// Create a new build request
    ///
    /// This will combine the many inputs here into a single source of truth. Fields will be duplicated
    /// from the inputs since various things might need to be autodetected.
    ///
    /// When creating a new build request we need to take into account
    /// - The user's command line arguments
    /// - The crate's Cargo.toml
    /// - The dioxus.toml
    /// - The user's CliSettings
    /// - The workspace
    /// - The host (android tools, installed frameworks, etc)
    /// - The intended platform
    ///
    /// We will attempt to autodetect a number of things if not provided.
    ///
    /// We intend to not create new BuildRequests very often. Only when the CLI is invoked and then again
    /// if the Cargo.toml's change so such an extent that features are added or removed.
    pub async fn new(args: &BuildArgs) -> Result<Self> {
        let workspace = Workspace::current().await?;

        let crate_package = workspace.find_main_package(args.package.clone())?;

        let config = workspace
            .load_dioxus_config(crate_package)?
            .unwrap_or_default();

        let target_kind = match args.example.is_some() {
            true => TargetKind::Example,
            false => TargetKind::Bin,
        };

        let main_package = &workspace.krates[crate_package];

        let target_name = args
            .example
            .clone()
            .or(args.bin.clone())
            .or_else(|| {
                if let Some(default_run) = &main_package.default_run {
                    return Some(default_run.to_string());
                }

                let bin_count = main_package
                    .targets
                    .iter()
                    .filter(|x| x.kind.contains(&target_kind))
                    .count();

                if bin_count != 1 {
                    return None;
                }

                main_package.targets.iter().find_map(|x| {
                    if x.kind.contains(&target_kind) {
                        Some(x.name.clone())
                    } else {
                        None
                    }
                })
            })
            .unwrap_or(workspace.krates[crate_package].name.clone());

        let crate_target = main_package
            .targets
            .iter()
            .find(|target| {
                target_name == target.name.as_str() && target.kind.contains(&target_kind)
            })
            .with_context(|| {
                let target_of_kind = |kind|-> String {
                    let filtered_packages = main_package
                .targets
                .iter()
                .filter_map(|target| {
                    target.kind.contains(kind).then_some(target.name.as_str())
                }).collect::<Vec<_>>();
                filtered_packages.join(", ")};
                if let Some(example) = &args.example {
                    let examples = target_of_kind(&TargetKind::Example);
                    format!("Failed to find example {example}. \nAvailable examples are:\n{}", examples)
                } else if let Some(bin) = &args.bin {
                    let binaries = target_of_kind(&TargetKind::Bin);
                    format!("Failed to find binary {bin}. \nAvailable binaries are:\n{}", binaries)
                } else {
                    format!("Failed to find target {target_name}. \nIt looks like you are trying to build dioxus in a library crate. \
                    You either need to run dx from inside a binary crate or build a specific example with the `--example` flag. \
                    Available examples are:\n{}", target_of_kind(&TargetKind::Example))
                }
            })?
            .clone();

        let default_platform = Self::default_platform(&main_package);
        let mut features = vec![];
        let mut no_default_features = false;

        // The user passed --platform XYZ but already has `default = ["ABC"]` in their Cargo.toml
        // We want to strip out the default platform and use the one they passed, setting no-default-features
        if args.platform.is_some() && default_platform.is_some() {
            no_default_features = true;
            features.extend(Self::platformless_features(&main_package));
        }

        // Inherit the platform from the args, or auto-detect it
        let platform = args
            .platform
            .map(|p| Some(p))
            .unwrap_or_else(|| Self::autodetect_platform(&workspace, &main_package).map(|a| a.0))
            .context("No platform was specified and could not be auto-detected. Please specify a platform with `--platform <platform>` or set a default platform using a cargo feature.")?;

        // Add any features required to turn on the client
        features.push(Self::feature_for_platform(&main_package, platform));

        // Set the profile of the build if it's not already set
        // This is mostly used for isolation of builds (preventing thrashing) but also useful to have multiple performance profiles
        // We might want to move some of these profiles into dioxus.toml and make them "virtual".
        let profile = match args.profile.clone() {
            Some(profile) => profile,
            None if args.release => "release".to_string(),
            None => match platform {
                Platform::Android => PROFILE_ANDROID.to_string(),
                Platform::Web => PROFILE_WASM.to_string(),
                Platform::Server => PROFILE_SERVER.to_string(),
                _ => "dev".to_string(),
            },
        };

        // Determine the --package we'll pass to cargo.
        // todo: I think this might be wrong - we don't want to use main_package necessarily...a
        let package = args
            .package
            .clone()
            .unwrap_or_else(|| main_package.name.clone());

        // We usually use the simulator unless --device is passed *or* a device is detected by probing.
        // For now, though, since we don't have probing, it just defaults to false
        // Tools like xcrun/adb can detect devices
        let device = args.device.unwrap_or(false);

        // We want a real triple to build with, so we'll autodetect it if it's not provided
        // The triple ends up being a source of truth for us later hence all this work to figure it out
        let target = match args.target.clone() {
            Some(target) => target,
            None => match platform {
                // Generally just use the host's triple for native executables unless specified otherwisea
                Platform::MacOS
                | Platform::Windows
                | Platform::Linux
                | Platform::Server
                | Platform::Liveview => target_lexicon::HOST,

                // We currently assume unknown-unknown for web, but we might want to eventually
                // support emscripten
                Platform::Web => "wasm32-unknown-unknown".parse().unwrap(),

                // For iOS we should prefer the actual architecture for the simulator, but in lieu of actually
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
                Platform::Android => {
                    super::android_tools()
                        .unwrap()
                        .autodetect_android_triple()
                        .await
                }
            },
        };

        Ok(Self {
            platform,
            features,
            no_default_features,
            crate_package,
            crate_target,
            profile,
            triple: target,
            device,
            workspace,
            config,
            custom_target_dir: None,
            cargo_args: args.cargo_args.clone(),
            nightly: args.nightly,
            cargo_package: package,
            release: args.release,
            skip_assets: args.skip_assets,
            cranelift: args.cranelift,
            wasm_split: args.wasm_split,
            debug_symbols: args.debug_symbols,
            inject_loading_scripts: args.inject_loading_scripts,
        })
    }

    pub(crate) async fn build(&self, ctx: &BuildContext) -> Result<BuildArtifacts> {
        // Run the cargo build to produce our artifacts
        let mut artifacts = self.cargo_build(&ctx).await?;

        // Write the build artifacts to the bundle on the disk
        match ctx.mode {
            BuildMode::Thin { aslr_reference, .. } => {
                self.write_patch(aslr_reference, artifacts.time_start)
                    .await?;
            }

            BuildMode::Base | BuildMode::Fat => {
                ctx.status_start_bundle();

                self.write_executable(&ctx, &artifacts.exe, &mut artifacts.assets)
                    .await
                    .context("Failed to write main executable")?;
                self.write_assets(&ctx, &artifacts.assets)
                    .await
                    .context("Failed to write assets")?;
                self.write_metadata().await?;
                self.optimize(&ctx).await?;
                self.assemble(&ctx)
                    .await
                    .context("Failed to assemble app bundle")?;

                tracing::debug!("Bundle created at {}", self.root_dir().display());
            }
        }

        Ok(artifacts)
    }

    async fn cargo_build(&self, ctx: &BuildContext) -> Result<BuildArtifacts> {
        let time_start = SystemTime::now();

        let mut cmd = self.build_command(ctx)?;

        tracing::debug!("Executing cargo...");
        tracing::trace!(dx_src = ?TraceSrc::Build, "Rust cargo args: {:#?}", cmd);

        // Extract the unit count of the crate graph so build_cargo has more accurate data
        // "Thin" builds only build the final exe, so we only need to build one crate
        let crate_count = match ctx.mode {
            BuildMode::Thin { .. } => 1,
            _ => self.get_unit_count_estimate(ctx).await,
        };

        // Update the status to show that we're starting the build and how many crates we expect to build
        ctx.status_starting_build(crate_count);

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

                    #[derive(Deserialize)]
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
                        ctx.status_build_error(line);
                    } else {
                        ctx.status_build_message(line)
                    }
                }
                Message::CompilerMessage(msg) => ctx.status_build_diagnostic(msg),
                Message::CompilerArtifact(artifact) => {
                    units_compiled += 1;
                    match artifact.executable {
                        Some(executable) => output_location = Some(executable.into()),
                        None => ctx.status_build_progress(
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
        let assets = self.collect_assets(&exe)?;
        let time_end = SystemTime::now();
        let mode = ctx.mode.clone();
        tracing::debug!("Build completed successfully - output location: {:?}", exe);

        Ok(BuildArtifacts {
            exe,
            direct_rustc,
            time_start,
            time_end,
            assets,
            mode,
        })
    }

    /// Traverse the target directory and collect all assets from the incremental cache
    ///
    /// This uses "known paths" that have stayed relatively stable during cargo's lifetime.
    /// One day this system might break and we might need to go back to using the linker approach.
    fn collect_assets(&self, exe: &Path) -> Result<AssetManifest> {
        tracing::debug!("Collecting assets ...");

        if self.skip_assets {
            return Ok(AssetManifest::default());
        }

        // walk every file in the incremental cache dir, reading and inserting items into the manifest.
        let mut manifest = AssetManifest::default();

        // And then add from the exe directly, just in case it's LTO compiled and has no incremental cache
        _ = manifest.add_from_object_path(exe);

        Ok(manifest)
    }

    /// Take the output of rustc and make it into the main exe of the bundle
    ///
    /// For wasm, we'll want to run `wasm-bindgen` to make it a wasm binary along with some other optimizations
    /// Other platforms we might do some stripping or other optimizations
    /// Move the executable to the workdir
    async fn write_executable(
        &self,
        ctx: &BuildContext,
        exe: &Path,
        assets: &mut AssetManifest,
    ) -> Result<()> {
        match self.platform {
            // Run wasm-bindgen on the wasm binary and set its output to be in the bundle folder
            // Also run wasm-opt on the wasm binary, and sets the index.html since that's also the "executable".
            //
            // The wasm stuff will be in a folder called "wasm" in the workdir.
            //
            // Final output format:
            // ```
            // dx/
            //     app/
            //         web/
            //             bundle/
            //             build/
            //                 public/
            //                     index.html
            //                     wasm/
            //                        app.wasm
            //                        glue.js
            //                        snippets/
            //                            ...
            //                     assets/
            //                        logo.png
            // ```
            Platform::Web => {
                self.bundle_web(ctx, exe, assets).await?;
            }

            // this will require some extra oomf to get the multi architecture builds...
            // for now, we just copy the exe into the current arch (which, sorry, is hardcoded for my m1)
            // we'll want to do multi-arch builds in the future, so there won't be *one* exe dir to worry about
            // eventually `exe_dir` and `main_exe` will need to take in an arch and return the right exe path
            //
            // todo(jon): maybe just symlink this rather than copy it?
            // we might want to eventually use the objcopy logic to handle this
            //
            // https://github.com/rust-mobile/xbuild/blob/master/xbuild/template/lib.rs
            // https://github.com/rust-mobile/xbuild/blob/master/apk/src/lib.rs#L19
            Platform::Android |

            // These are all super simple, just copy the exe into the folder
            // eventually, perhaps, maybe strip + encrypt the exe?
            Platform::MacOS
            | Platform::Windows
            | Platform::Linux
            | Platform::Ios
            | Platform::Liveview
            | Platform::Server => {
                _ = std::fs::remove_dir_all(self.exe_dir());
                std::fs::create_dir_all(self.exe_dir())?;
                std::fs::copy(&exe, self.main_exe())?;
            }
        }

        Ok(())
    }

    /// Copy the assets out of the manifest and into the target location
    ///
    /// Should be the same on all platforms - just copy over the assets from the manifest into the output directory
    async fn write_assets(&self, ctx: &BuildContext, assets: &AssetManifest) -> Result<()> {
        // Server doesn't need assets - web will provide them
        if self.platform == Platform::Server {
            return Ok(());
        }

        let asset_dir = self.asset_dir();

        // First, clear the asset dir of any files that don't exist in the new manifest
        _ = tokio::fs::create_dir_all(&asset_dir).await;

        // Create a set of all the paths that new files will be bundled to
        let mut keep_bundled_output_paths: HashSet<_> = assets
            .assets
            .values()
            .map(|a| asset_dir.join(a.bundled_path()))
            .collect();

        // The CLI creates a .version file in the asset dir to keep track of what version of the optimizer
        // the asset was processed. If that version doesn't match the CLI version, we need to re-optimize
        // all assets.
        let version_file = self.asset_optimizer_version_file();
        let clear_cache = std::fs::read_to_string(&version_file)
            .ok()
            .filter(|s| s == crate::VERSION.as_str())
            .is_none();
        if clear_cache {
            keep_bundled_output_paths.clear();
        }

        // one possible implementation of walking a directory only visiting files
        fn remove_old_assets<'a>(
            path: &'a Path,
            keep_bundled_output_paths: &'a HashSet<PathBuf>,
        ) -> Pin<Box<dyn Future<Output = std::io::Result<()>> + Send + 'a>> {
            Box::pin(async move {
                // If this asset is in the manifest, we don't need to remove it
                let canon_path = dunce::canonicalize(path)?;
                if keep_bundled_output_paths.contains(canon_path.as_path()) {
                    return Ok(());
                }

                // Otherwise, if it is a directory, we need to walk it and remove child files
                if path.is_dir() {
                    for entry in std::fs::read_dir(path)?.flatten() {
                        let path = entry.path();
                        remove_old_assets(&path, keep_bundled_output_paths).await?;
                    }
                    if path.read_dir()?.next().is_none() {
                        // If the directory is empty, remove it
                        tokio::fs::remove_dir(path).await?;
                    }
                } else {
                    // If it is a file, remove it
                    tokio::fs::remove_file(path).await?;
                }

                Ok(())
            })
        }

        tracing::debug!("Removing old assets");
        tracing::trace!(
            "Keeping bundled output paths: {:#?}",
            keep_bundled_output_paths
        );
        remove_old_assets(&asset_dir, &keep_bundled_output_paths).await?;

        // todo(jon): we also want to eventually include options for each asset's optimization and compression, which we currently aren't
        let mut assets_to_transfer = vec![];

        // Queue the bundled assets
        for (asset, bundled) in &assets.assets {
            let from = asset.clone();
            let to = asset_dir.join(bundled.bundled_path());

            // prefer to log using a shorter path relative to the workspace dir by trimming the workspace dir
            let from_ = from
                .strip_prefix(self.workspace_dir())
                .unwrap_or(from.as_path());
            let to_ = from
                .strip_prefix(self.workspace_dir())
                .unwrap_or(to.as_path());

            tracing::debug!("Copying asset {from_:?} to {to_:?}");
            assets_to_transfer.push((from, to, *bundled.options()));
        }

        // And then queue the legacy assets
        // ideally, one day, we can just check the rsx!{} calls for references to assets
        for from in self.legacy_asset_dir_files() {
            let to = asset_dir.join(from.file_name().unwrap());
            tracing::debug!("Copying legacy asset {from:?} to {to:?}");
            assets_to_transfer.push((from, to, AssetOptions::Unknown));
        }

        let asset_count = assets_to_transfer.len();
        let started_processing = AtomicUsize::new(0);
        let copied = AtomicUsize::new(0);

        // Parallel Copy over the assets and keep track of progress with an atomic counter
        let progress = ctx.tx.clone();
        let ws_dir = self.workspace_dir();
        // Optimizing assets is expensive and blocking, so we do it in a tokio spawn blocking task
        tokio::task::spawn_blocking(move || {
            assets_to_transfer
                .par_iter()
                .try_for_each(|(from, to, options)| {
                    let processing = started_processing.fetch_add(1, Ordering::SeqCst);
                    let from_ = from.strip_prefix(&ws_dir).unwrap_or(from);
                    tracing::trace!(
                        "Starting asset copy {processing}/{asset_count} from {from_:?}"
                    );

                    let res = process_file_to(options, from, to);
                    if let Err(err) = res.as_ref() {
                        tracing::error!("Failed to copy asset {from:?}: {err}");
                    }

                    let finished = copied.fetch_add(1, Ordering::SeqCst);
                    BuildContext::status_copied_asset(
                        &progress,
                        finished,
                        asset_count,
                        from.to_path_buf(),
                    );

                    res.map(|_| ())
                })
        })
        .await
        .map_err(|e| anyhow::anyhow!("A task failed while trying to copy assets: {e}"))??;

        // // Remove the wasm bindgen output directory if it exists
        // _ = std::fs::remove_dir_all(self.wasm_bindgen_out_dir());

        // Write the version file so we know what version of the optimizer we used
        std::fs::write(self.asset_optimizer_version_file(), crate::VERSION.as_str())?;

        Ok(())
    }

    /// libpatch-{time}.(so/dll/dylib) (next to the main exe)
    pub fn patch_exe(&self, time_start: SystemTime) -> PathBuf {
        let path = self.main_exe().with_file_name(format!(
            "libpatch-{}",
            time_start.duration_since(UNIX_EPOCH).unwrap().as_millis(),
        ));

        let extension = match self.triple.operating_system {
            OperatingSystem::Darwin(_) => "dylib",
            OperatingSystem::MacOSX(_) => "dylib",
            OperatingSystem::IOS(_) => "dylib",
            OperatingSystem::Unknown if self.platform == Platform::Web => "wasm",
            OperatingSystem::Windows => "dll",
            OperatingSystem::Linux => "so",
            OperatingSystem::Wasi => "wasm",
            _ => "",
        };

        path.with_extension(extension)
    }

    /// Run our custom linker setup to generate a patch file in the right location
    async fn write_patch(&self, aslr_reference: u64, time_start: SystemTime) -> Result<()> {
        tracing::debug!("Patching existing bundle");

        let raw_args = std::fs::read_to_string(&self.link_args_file())
            .context("Failed to read link args from file")?;

        let args = raw_args.lines().collect::<Vec<_>>();

        let orig_exe = self.main_exe();
        tracing::debug!("writing patch - orig_exe: {:?}", orig_exe);

        let object_files = args
            .iter()
            .filter(|arg| arg.ends_with(".rcgu.o"))
            .sorted()
            .map(|arg| PathBuf::from(arg))
            .collect::<Vec<_>>();

        let resolved_patch_bytes = subsecond_cli_support::resolve_undefined(
            &orig_exe,
            &object_files,
            &self.triple,
            aslr_reference,
        )
        .expect("failed to resolve patch symbols");

        let patch_file = self.main_exe().with_file_name("patch-syms.o");
        std::fs::write(&patch_file, resolved_patch_bytes)?;

        let linker = match self.platform {
            Platform::Web => self.workspace.wasm_ld(),
            Platform::Android => {
                let tools =
                    crate::build::android_tools().context("Could not determine android tools")?;
                tools.android_cc(&self.triple)
            }

            // Note that I think rust uses rust-lld now, so we need to respect its argument profile
            // https://blog.rust-lang.org/2024/05/17/enabling-rust-lld-on-linux.html
            Platform::MacOS
            | Platform::Ios
            | Platform::Linux
            | Platform::Server
            | Platform::Liveview => PathBuf::from("cc"),

            // I think this is right?? does windows use cc?
            Platform::Windows => PathBuf::from("cc"),
        };

        let thin_args = self.thin_link_args(&args, aslr_reference)?;

        // let mut env_vars = vec![];
        // self.build_android_env(&mut env_vars, false)?;

        // todo: we should throw out symbols that we don't need and/or assemble them manually
        // also we should make sure to propagate the right arguments (target, sysroot, etc)
        //
        // also, https://developer.apple.com/forums/thread/773907
        //       -undefined,dynamic_lookup is deprecated for ios but supposedly cpython is using it
        //       we might need to link a new patch file that implements the lookups
        let res = Command::new(linker)
            .args(object_files.iter())
            .arg(patch_file)
            .args(thin_args)
            .arg("-v")
            .arg("-o") // is it "-o" everywhere?
            .arg(&self.patch_exe(time_start))
            .output()
            .await?;

        let errs = String::from_utf8_lossy(&res.stderr);
        if !errs.is_empty() {
            if !self.patch_exe(time_start).exists() {
                tracing::error!("Failed to generate patch: {}", errs.trim());
            } else {
                tracing::debug!("Warnings during thin linking: {}", errs.trim());
            }
        }

        if self.platform == Platform::Web {}

        // // Clean up the temps manually
        // // todo: we might want to keep them around for debugging purposes
        // for file in object_files {
        //     _ = std::fs::remove_file(file);
        // }

        // Also clean up the original fat file since that's causing issues with rtld_global
        // todo: this might not be platform portable
        let link_orig = args
            .iter()
            .position(|arg| *arg == "-o")
            .expect("failed to find -o");
        let link_file: PathBuf = args[link_orig + 1].clone().into();
        _ = std::fs::remove_file(&link_file);

        Ok(())
    }

    fn thin_link_args(&self, original_args: &[&str], aslr_reference: u64) -> Result<Vec<String>> {
        use target_lexicon::OperatingSystem;

        let triple = self.triple.clone();
        let mut args = vec![];

        tracing::debug!("original args:\n{}", original_args.join("\n"));

        match triple.operating_system {
            // wasm32-unknown-unknown
            // use wasm-ld (gnu-lld)
            OperatingSystem::Unknown if self.platform == Platform::Web => {
                const WASM_PAGE_SIZE: u64 = 65536;
                let table_base = 2000 * (aslr_reference + 1);
                let global_base =
                    ((aslr_reference * WASM_PAGE_SIZE * 3) + (WASM_PAGE_SIZE * 32)) as i32;
                tracing::info!(
                    "using aslr of table: {} and global: {}",
                    table_base,
                    global_base
                );

                args.extend([
                    // .arg("-z")
                    // .arg("stack-size=1048576")
                    "--import-memory".to_string(),
                    "--import-table".to_string(),
                    "--growable-table".to_string(),
                    "--export".to_string(),
                    "main".to_string(),
                    "--export-all".to_string(),
                    "--stack-first".to_string(),
                    "--allow-undefined".to_string(),
                    "--no-demangle".to_string(),
                    "--no-entry".to_string(),
                    "--emit-relocs".to_string(),
                    // todo: we need to modify the post-processing code
                    format!("--table-base={}", table_base).to_string(),
                    format!("--global-base={}", global_base).to_string(),
                ]);
            }

            // this uses "cc" and these args need to be ld compatible
            // aarch64-apple-ios
            // aarch64-apple-darwin
            OperatingSystem::IOS(_) | OperatingSystem::MacOSX(_) | OperatingSystem::Darwin(_) => {
                args.extend([
                    "-Wl,-dylib".to_string(),
                    // "-Wl,-export_dynamic".to_string(),
                    // "-Wl,-unexported_symbol,_main".to_string(),
                    // "-Wl,-undefined,dynamic_lookup".to_string(),
                ]);

                match triple.architecture {
                    target_lexicon::Architecture::Aarch64(_) => {
                        args.push("-arch".to_string());
                        args.push("arm64".to_string());
                    }
                    target_lexicon::Architecture::X86_64 => {
                        args.push("-arch".to_string());
                        args.push("x86_64".to_string());
                    }
                    _ => {}
                }
            }

            // android/linux
            // need to be compatible with lld
            OperatingSystem::Linux if triple.environment == Environment::Android => {
                args.extend(
                    [
                        "-shared".to_string(),
                        "-Wl,--eh-frame-hdr".to_string(),
                        "-Wl,-z,noexecstack".to_string(),
                        "-landroid".to_string(),
                        "-llog".to_string(),
                        "-lOpenSLES".to_string(),
                        "-landroid".to_string(),
                        "-ldl".to_string(),
                        "-ldl".to_string(),
                        "-llog".to_string(),
                        "-lunwind".to_string(),
                        "-ldl".to_string(),
                        "-lm".to_string(),
                        "-lc".to_string(),
                        "-Wl,-z,relro,-z,now".to_string(),
                        "-nodefaultlibs".to_string(),
                        "-Wl,-Bdynamic".to_string(),
                    ]
                    .iter()
                    .map(|s| s.to_string()),
                );

                match triple.architecture {
                    target_lexicon::Architecture::Aarch64(_) => {
                        // args.push("-Wl,--target=aarch64-linux-android".to_string());
                    }
                    target_lexicon::Architecture::X86_64 => {
                        // args.push("-Wl,--target=x86_64-linux-android".to_string());
                    }
                    _ => {}
                }
            }

            OperatingSystem::Linux => {
                args.extend([
                    "-Wl,--eh-frame-hdr".to_string(),
                    "-Wl,-z,noexecstack".to_string(),
                    "-Wl,-z,relro,-z,now".to_string(),
                    "-nodefaultlibs".to_string(),
                    "-Wl,-Bdynamic".to_string(),
                ]);
            }

            OperatingSystem::Windows => {}

            _ => return Err(anyhow::anyhow!("Unsupported platform for thin linking").into()),
        }

        let extract_value = |arg: &str| -> Option<String> {
            original_args
                .iter()
                .position(|a| *a == arg)
                .map(|i| original_args[i + 1].to_string())
        };

        if let Some(vale) = extract_value("-target") {
            args.push("-target".to_string());
            args.push(vale);
        }

        if let Some(vale) = extract_value("-isysroot") {
            args.push("-isysroot".to_string());
            args.push(vale);
        }

        tracing::info!("final args:{:#?}", args);

        Ok(args)
    }

    #[tracing::instrument(
        skip(self),
        level = "trace",
        fields(dx_src = ?TraceSrc::Build)
    )]
    fn build_command(&self, ctx: &BuildContext) -> Result<Command> {
        // Prefer using the direct rustc if we have it
        if let BuildMode::Thin { direct_rustc, .. } = &ctx.mode {
            tracing::debug!("Using direct rustc: {:?}", direct_rustc);
            if !direct_rustc.is_empty() {
                let mut cmd = Command::new(direct_rustc[0].clone());
                cmd.args(direct_rustc[1..].iter());
                cmd.envs(self.env_vars(ctx)?);
                cmd.current_dir(self.workspace_dir());
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
            .current_dir(self.crate_dir())
            .arg("--message-format")
            .arg("json-diagnostic-rendered-ansi")
            .args(self.build_arguments(ctx))
            .envs(self.env_vars(ctx)?);

        Ok(cmd)
    }

    /// Create a list of arguments for cargo builds
    fn build_arguments(&self, ctx: &BuildContext) -> Vec<String> {
        let mut cargo_args = Vec::new();

        // Add required profile flags. --release overrides any custom profiles.
        cargo_args.push("--profile".to_string());
        cargo_args.push(self.profile.to_string());

        // Pass the appropriate target to cargo. We *always* specify a target which is somewhat helpful for preventing thrashing
        cargo_args.push("--target".to_string());
        cargo_args.push(self.triple.to_string());

        // We always run in verbose since the CLI itself is the one doing the presentation
        cargo_args.push("--verbose".to_string());

        if self.no_default_features {
            cargo_args.push("--no-default-features".to_string());
        }

        if !self.features.is_empty() {
            cargo_args.push("--features".to_string());
            cargo_args.push(self.features.join(" "));
        }

        // We *always* set the package since that's discovered from cargo metadata
        cargo_args.push(String::from("-p"));
        cargo_args.push(self.cargo_package.clone());

        match self.executable_type() {
            krates::cm::TargetKind::Bin => cargo_args.push("--bin".to_string()),
            krates::cm::TargetKind::Lib => cargo_args.push("--lib".to_string()),
            krates::cm::TargetKind::Example => cargo_args.push("--example".to_string()),
            _ => {}
        };

        cargo_args.push(self.executable_name().to_string());

        cargo_args.extend(self.cargo_args.clone());

        cargo_args.push("--".to_string());

        // the bundle splitter needs relocation data
        // we'll trim these out if we don't need them during the bundling process
        // todo(jon): for wasm binary patching we might want to leave these on all the time.
        if self.platform == Platform::Web && self.wasm_split {
            cargo_args.push("-Clink-args=--emit-relocs".to_string());
        }

        // dx *always* links android and thin builds
        if self.platform == Platform::Android || matches!(ctx.mode, BuildMode::Thin { .. }) {
            cargo_args.push(format!(
                "-Clinker={}",
                dunce::canonicalize(std::env::current_exe().unwrap())
                    .unwrap()
                    .display()
            ));
        }

        match ctx.mode {
            BuildMode::Base => {}
            BuildMode::Thin { .. } => {}
            BuildMode::Fat => {
                // This prevents rust from passing -dead_strip to the linker
                // todo: don't save temps here unless we become the linker for the base app
                cargo_args.extend_from_slice(&[
                    "-Csave-temps=true".to_string(),
                    "-Clink-dead-code".to_string(),
                ]);

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
                        cargo_args.push("-Clink-args=-Wl,--whole-archive".to_string());
                    }

                    // if web, -Wl,--whole-archive is required for the linker to work correctly.
                    // We also use --no-gc-sections and --export-table and --export-memory  to push
                    // said symbols into the export table.
                    //
                    // We use --emit-relocs but scrub those before they make it into the final output.
                    // This is designed for us to build a solid call graph.
                    //
                    // rust uses its own wasm-ld linker which can be found here (it's just gcc-ld with a -target):
                    // /Users/jonkelley/.rustup/toolchains/stable-aarch64-apple-darwin/lib/rustlib/aarch64-apple-darwin/bin/gcc-ld
                    // /Users/jonkelley/.rustup/toolchains/stable-aarch64-apple-darwin/lib/rustlib/aarch64-apple-darwin/bin/gcc-ld/wasm-ld
                    //
                    // export all should place things like env.memory into the export table so we can access them
                    // when loading the patches
                    Platform::Web => {
                        cargo_args.push("-Clink-arg=--no-gc-sections".into());
                        cargo_args.push("-Clink-arg=--growable-table".into());
                        cargo_args.push("-Clink-arg=--whole-archive".into());
                        cargo_args.push("-Clink-arg=--export-table".into());
                        cargo_args.push("-Clink-arg=--export-memory".into());
                        cargo_args.push("-Clink-arg=--emit-relocs".into());
                        cargo_args.push("-Clink-arg=--export=__stack_pointer".into());
                        cargo_args.push("-Clink-arg=--export=__heap_base".into());
                        cargo_args.push("-Clink-arg=--export=__data_end".into());
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
                self.package()
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
    async fn get_unit_count(&self, ctx: &BuildContext) -> crate::Result<usize> {
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
            .args(self.build_arguments(ctx))
            .envs(self.env_vars(ctx)?)
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
    async fn get_unit_count_estimate(&self, ctx: &BuildContext) -> usize {
        // Try to get it from nightly
        if let Ok(count) = self.get_unit_count(ctx).await {
            return count;
        }

        // Otherwise, use cargo metadata
        let units = self
            .workspace
            .krates
            .krates_filtered(krates::DepKind::Dev)
            .iter()
            .map(|k| k.targets.len())
            .sum::<usize>();

        (units as f64 / 3.5) as usize
    }

    fn env_vars(&self, ctx: &BuildContext) -> Result<Vec<(&str, String)>> {
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

        match &ctx.mode {
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
                        triple: self.triple.clone(),
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
            if let Some(base_path) = &self.config.web.app.base_path {
                env_vars.push((ASSET_ROOT_ENV, base_path.clone()));
            }
            env_vars.push((APP_TITLE_ENV, self.config.web.app.title.clone()));
        }

        Ok(env_vars)
    }

    pub fn build_android_env(
        &self,
        env_vars: &mut Vec<(&str, String)>,
        rustf_flags: bool,
    ) -> Result<PathBuf> {
        let tools = crate::build::android_tools().context("Could not determine android tools")?;
        let linker = tools.android_cc(&self.triple);
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
            Platform::MacOS => platform_dir.join(format!("{}.app", self.bundled_app_name())),
            Platform::Ios => platform_dir.join(format!("{}.app", self.bundled_app_name())),

            // in theory, these all could end up directly in the root dir
            Platform::Android => platform_dir.join("app"), // .apk (after bundling)
            Platform::Linux => platform_dir.join("app"),   // .appimage (after bundling)
            Platform::Windows => platform_dir.join("app"), // .exe (after bundling)
            Platform::Liveview => platform_dir.join("app"), // .exe (after bundling)
        }
    }

    pub(crate) fn platform_dir(&self) -> PathBuf {
        self.build_dir(self.platform, self.release)
    }

    pub fn platform_exe_name(&self) -> String {
        match self.platform {
            Platform::MacOS => self.executable_name().to_string(),
            Platform::Ios => self.executable_name().to_string(),
            Platform::Server => self.executable_name().to_string(),
            Platform::Liveview => self.executable_name().to_string(),
            Platform::Windows => format!("{}.exe", self.executable_name()),

            // from the apk spec, the root exe is a shared library
            // we include the user's rust code as a shared library with a fixed namespacea
            Platform::Android => "libdioxusmain.so".to_string(),

            Platform::Web => unimplemented!("there's no main exe on web"), // this will be wrong, I think, but not important?

            // todo: maybe this should be called AppRun?
            Platform::Linux => self.executable_name().to_string(),
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

        // handlebars
        #[derive(serde::Serialize)]
        struct HbsTypes {
            application_id: String,
            app_name: String,
        }
        let hbs_data = HbsTypes {
            application_id: self.full_mobile_app_name(),
            app_name: self.bundled_app_name(),
        };
        let hbs = handlebars::Handlebars::new();

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

    // pub(crate) async fn new(args: &TargetArgs) -> Result<Self> {

    //     Ok(Self {
    //         workspace: workspace.clone(),
    //         package,
    //         config: dioxus_config,
    //         target: Arc::new(target),
    //     })
    // }

    /// The asset dir we used to support before manganis became the default.
    /// This generally was just a folder in your Dioxus.toml called "assets" or "public" where users
    /// would store their assets.
    ///
    /// With manganis you now use `asset!()` and we pick it up automatically.
    pub(crate) fn legacy_asset_dir(&self) -> Option<PathBuf> {
        self.config
            .application
            .asset_dir
            .clone()
            .map(|dir| self.crate_dir().join(dir))
    }

    /// Get the list of files in the "legacy" asset directory
    pub(crate) fn legacy_asset_dir_files(&self) -> Vec<PathBuf> {
        let mut files = vec![];

        let Some(legacy_asset_dir) = self.legacy_asset_dir() else {
            return files;
        };

        let Ok(read_dir) = legacy_asset_dir.read_dir() else {
            return files;
        };

        for entry in read_dir.flatten() {
            files.push(entry.path());
        }

        files
    }

    /// Get the directory where this app can write to for this session that's guaranteed to be stable
    /// for the same app. This is useful for emitting state like window position and size.
    ///
    /// The directory is specific for this app and might be
    pub(crate) fn session_cache_dir(&self) -> PathBuf {
        self.internal_out_dir()
            .join(self.executable_name())
            .join("session-cache")
    }

    /// Get the outdir specified by the Dioxus.toml, relative to the crate directory.
    /// We don't support workspaces yet since that would cause a collision of bundles per project.
    pub(crate) fn crate_out_dir(&self) -> Option<PathBuf> {
        self.config
            .application
            .out_dir
            .as_ref()
            .map(|out_dir| self.crate_dir().join(out_dir))
    }

    /// Compose an out directory. Represents the typical "dist" directory that
    /// is "distributed" after building an application (configurable in the
    /// `Dioxus.toml`).
    fn internal_out_dir(&self) -> PathBuf {
        let dir = self.workspace_dir().join("target").join("dx");
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// Create a workdir for the given platform
    /// This can be used as a temporary directory for the build, but in an observable way such that
    /// you can see the files in the directory via `target`
    ///
    /// target/dx/build/app/web/
    /// target/dx/build/app/web/public/
    /// target/dx/build/app/web/server.exe
    pub(crate) fn build_dir(&self, platform: Platform, release: bool) -> PathBuf {
        self.internal_out_dir()
            .join(self.executable_name())
            .join(if release { "release" } else { "debug" })
            .join(platform.build_folder_name())
    }

    /// target/dx/bundle/app/
    /// target/dx/bundle/app/blah.app
    /// target/dx/bundle/app/blah.exe
    /// target/dx/bundle/app/public/
    pub(crate) fn bundle_dir(&self, platform: Platform) -> PathBuf {
        self.internal_out_dir()
            .join(self.executable_name())
            .join("bundle")
            .join(platform.build_folder_name())
    }

    /// Get the workspace directory for the crate
    pub(crate) fn workspace_dir(&self) -> PathBuf {
        self.workspace
            .krates
            .workspace_root()
            .as_std_path()
            .to_path_buf()
    }

    /// Get the directory of the crate
    pub(crate) fn crate_dir(&self) -> PathBuf {
        self.package()
            .manifest_path
            .parent()
            .unwrap()
            .as_std_path()
            .to_path_buf()
    }

    /// Get the main source file of the target
    pub(crate) fn main_source_file(&self) -> PathBuf {
        self.crate_target.src_path.as_std_path().to_path_buf()
    }

    /// Get the package we are currently in
    pub(crate) fn package(&self) -> &krates::cm::Package {
        &self.workspace.krates[self.crate_package]
    }

    /// Get the name of the package we are compiling
    pub(crate) fn executable_name(&self) -> &str {
        &self.crate_target.name
    }

    /// Get the type of executable we are compiling
    pub(crate) fn executable_type(&self) -> krates::cm::TargetKind {
        self.crate_target.kind[0].clone()
    }

    /// Try to autodetect the platform from the package by reading its features
    ///
    /// Read the default-features list and/or the features list on dioxus to see if we can autodetect the platform
    fn autodetect_platform(
        ws: &Workspace,
        package: &krates::cm::Package,
    ) -> Option<(Platform, String)> {
        let krate = ws.krates.krates_by_name("dioxus").next()?;

        // We're going to accumulate the platforms that are enabled
        // This will let us create a better warning if multiple platforms are enabled
        let manually_enabled_platforms = ws
            .krates
            .get_enabled_features(krate.kid)?
            .iter()
            .flat_map(|feature| {
                tracing::trace!("Autodetecting platform from feature {feature}");
                Platform::autodetect_from_cargo_feature(feature).map(|f| (f, feature.to_string()))
            })
            .collect::<Vec<_>>();

        if manually_enabled_platforms.len() > 1 {
            tracing::error!("Multiple platforms are enabled. Please specify a platform with `--platform <platform>` or set a single default platform using a cargo feature.");
            for platform in manually_enabled_platforms {
                tracing::error!("  - {platform:?}");
            }
            return None;
        }

        if manually_enabled_platforms.len() == 1 {
            return manually_enabled_platforms.first().cloned();
        }

        // Let's try and find the list of platforms from the feature list
        // This lets apps that specify web + server to work without specifying the platform.
        // This is because we treat `server` as a binary thing rather than a dedicated platform, so at least we can disambiguate it
        let possible_platforms = package
            .features
            .iter()
            .filter_map(|(feature, _features)| {
                match Platform::autodetect_from_cargo_feature(feature) {
                    Some(platform) => Some((platform, feature.to_string())),
                    None => {
                        let auto_implicit = _features
                            .iter()
                            .filter_map(|f| {
                                if !f.starts_with("dioxus?/") && !f.starts_with("dioxus/") {
                                    return None;
                                }

                                let rest = f
                                    .trim_start_matches("dioxus/")
                                    .trim_start_matches("dioxus?/");

                                Platform::autodetect_from_cargo_feature(rest)
                            })
                            .collect::<Vec<_>>();

                        if auto_implicit.len() == 1 {
                            Some((auto_implicit.first().copied().unwrap(), feature.to_string()))
                        } else {
                            None
                        }
                    }
                }
            })
            .filter(|platform| platform.0 != Platform::Server)
            .collect::<Vec<_>>();

        if possible_platforms.len() == 1 {
            return possible_platforms.first().cloned();
        }

        None
    }

    /// Get the features required to build for the given platform
    fn feature_for_platform(package: &krates::cm::Package, platform: Platform) -> String {
        // Try to find the feature that activates the dioxus feature for the given platform
        let dioxus_feature = platform.feature_name();

        let res = package.features.iter().find_map(|(key, features)| {
            // if the feature is just the name of the platform, we use that
            if key == dioxus_feature {
                return Some(key.clone());
            }

            // Otherwise look for the feature that starts with dioxus/ or dioxus?/ and matches the platform
            for feature in features {
                if let Some((_, after_dioxus)) = feature.split_once("dioxus") {
                    if let Some(dioxus_feature_enabled) =
                        after_dioxus.trim_start_matches('?').strip_prefix('/')
                    {
                        // If that enables the feature we are looking for, return that feature
                        if dioxus_feature_enabled == dioxus_feature {
                            return Some(key.clone());
                        }
                    }
                }
            }

            None
        });

        res.unwrap_or_else(|| {
            let fallback = format!("dioxus/{}", platform.feature_name()) ;
            tracing::debug!(
                "Could not find explicit feature for platform {platform}, passing `fallback` instead"
            );
            fallback
        })
    }

    /// Check if assets should be pre_compressed. This will only be true in release mode if the user
    /// has enabled pre_compress in the web config.
    pub(crate) fn should_pre_compress_web_assets(&self, release: bool) -> bool {
        self.config.web.pre_compress && release
    }

    // The `opt-level=1` increases build times, but can noticeably decrease time
    // between saving changes and being able to interact with an app (for wasm/web). The "overall"
    // time difference (between having and not having the optimization) can be
    // almost imperceptible (~1 s) but also can be very noticeable (~6 s) — depends
    // on setup (hardware, OS, browser, idle load).
    //
    // Find or create the client and server profiles in the top-level Cargo.toml file
    // todo(jon): we should/could make these optional by placing some defaults somewhere
    pub(crate) fn initialize_profiles(&self) -> crate::Result<()> {
        let config_path = self.workspace_dir().join("Cargo.toml");
        let mut config = match std::fs::read_to_string(&config_path) {
            Ok(config) => config.parse::<toml_edit::DocumentMut>().map_err(|e| {
                crate::Error::Other(anyhow::anyhow!("Failed to parse Cargo.toml: {}", e))
            })?,
            Err(_) => Default::default(),
        };

        if let Item::Table(table) = config
            .as_table_mut()
            .entry("profile")
            .or_insert(Item::Table(Default::default()))
        {
            if let toml_edit::Entry::Vacant(entry) = table.entry(PROFILE_WASM) {
                let mut client = toml_edit::Table::new();
                client.insert("inherits", Item::Value("dev".into()));
                client.insert("opt-level", Item::Value(1.into()));
                entry.insert(Item::Table(client));
            }

            if let toml_edit::Entry::Vacant(entry) = table.entry(PROFILE_SERVER) {
                let mut server = toml_edit::Table::new();
                server.insert("inherits", Item::Value("dev".into()));
                entry.insert(Item::Table(server));
            }

            if let toml_edit::Entry::Vacant(entry) = table.entry(PROFILE_ANDROID) {
                let mut android = toml_edit::Table::new();
                android.insert("inherits", Item::Value("dev".into()));
                entry.insert(Item::Table(android));
            }
        }

        std::fs::write(config_path, config.to_string())
            .context("Failed to write profiles to Cargo.toml")?;

        Ok(())
    }

    /// Return the version of the wasm-bindgen crate if it exists
    pub fn wasm_bindgen_version(&self) -> Option<String> {
        self.workspace
            .krates
            .krates_by_name("wasm-bindgen")
            .next()
            .map(|krate| krate.krate.version.to_string())
    }

    pub(crate) fn default_platform(package: &krates::cm::Package) -> Option<Platform> {
        let default = package.features.get("default")?;

        // we only trace features 1 level deep..
        for feature in default.iter() {
            // If the user directly specified a platform we can just use that.
            if feature.starts_with("dioxus/") {
                let dx_feature = feature.trim_start_matches("dioxus/");
                let auto = Platform::autodetect_from_cargo_feature(dx_feature);
                if auto.is_some() {
                    return auto;
                }
            }

            // If the user is specifying an internal feature that points to a platform, we can use that
            let internal_feature = package.features.get(feature);
            if let Some(internal_feature) = internal_feature {
                for feature in internal_feature {
                    if feature.starts_with("dioxus/") {
                        let dx_feature = feature.trim_start_matches("dioxus/");
                        let auto = Platform::autodetect_from_cargo_feature(dx_feature);
                        if auto.is_some() {
                            return auto;
                        }
                    }
                }
            }
        }

        None
    }

    /// Gather the features that are enabled for the package
    fn platformless_features(package: &krates::cm::Package) -> Vec<String> {
        let default = package.features.get("default").unwrap();
        let mut kept_features = vec![];

        // Only keep the top-level features in the default list that don't point to a platform directly
        // IE we want to drop `web` if default = ["web"]
        'top: for feature in default {
            // Don't keep features that point to a platform via dioxus/blah
            if feature.starts_with("dioxus/") {
                let dx_feature = feature.trim_start_matches("dioxus/");
                if Platform::autodetect_from_cargo_feature(dx_feature).is_some() {
                    continue 'top;
                }
            }

            // Don't keep features that point to a platform via an internal feature
            if let Some(internal_feature) = package.features.get(feature) {
                for feature in internal_feature {
                    if feature.starts_with("dioxus/") {
                        let dx_feature = feature.trim_start_matches("dioxus/");
                        if Platform::autodetect_from_cargo_feature(dx_feature).is_some() {
                            continue 'top;
                        }
                    }
                }
            }

            // Otherwise we can keep it
            kept_features.push(feature.to_string());
        }

        kept_features
    }

    pub(crate) fn mobile_org(&self) -> String {
        let identifier = self.bundle_identifier();
        let mut split = identifier.splitn(3, '.');
        let sub = split
            .next()
            .expect("Identifier to have at least 3 periods like `com.example.app`");
        let tld = split
            .next()
            .expect("Identifier to have at least 3 periods like `com.example.app`");
        format!("{}.{}", sub, tld)
    }

    pub(crate) fn bundled_app_name(&self) -> String {
        use convert_case::{Case, Casing};
        self.executable_name().to_case(Case::Pascal)
    }

    pub(crate) fn full_mobile_app_name(&self) -> String {
        format!("{}.{}", self.mobile_org(), self.bundled_app_name())
    }

    pub(crate) fn bundle_identifier(&self) -> String {
        if let Some(identifier) = self.config.bundle.identifier.clone() {
            return identifier.clone();
        }

        format!("com.example.{}", self.bundled_app_name())
    }

    /// The item that we'll try to run directly if we need to.
    ///
    /// todo(jon): we should name the app properly instead of making up the exe name. It's kinda okay for dev mode, but def not okay for prod
    pub fn main_exe(&self) -> PathBuf {
        self.exe_dir().join(self.platform_exe_name())
    }

    /// Does the app specify:
    ///
    /// - Dioxus with "fullstack" enabled? (access to serverfns, etc)
    /// - An explicit "fullstack" feature that enables said feature?
    ///
    /// Note that we don't detect if dependencies enable it transitively since we want to be explicit about it.
    ///
    /// The intention here is to detect if "fullstack" is enabled in the target's features list:
    /// ```toml
    /// [dependencies]
    /// dioxus = { version = "0.4", features = ["fullstack"] }
    /// ```
    ///
    /// or as an explicit feature in default:
    /// ```toml
    /// [features]
    /// default = ["dioxus/fullstack"]
    /// ```
    ///
    /// or as a default feature that enables the dioxus feature:
    /// ```toml
    /// [features]
    /// default = ["fullstack"]
    /// fullstack = ["dioxus/fullstack"]
    /// ```
    ///
    /// or as an explicit feature (that enables the dioxus feature):
    /// ```
    /// dx serve app --features "fullstack"
    /// ```
    pub fn fullstack_feature_enabled(&self) -> bool {
        let dioxus_dep = self
            .package()
            .dependencies
            .iter()
            .find(|dep| dep.name == "dioxus");

        // If we don't have a dioxus dependency, we can't be fullstack. This shouldn't impact non-dioxus projects
        let Some(dioxus_dep) = dioxus_dep else {
            return false;
        };

        // Check if the dioxus dependency has the "fullstack" feature enabled
        if dioxus_dep.features.iter().any(|f| f == "fullstack") {
            return true;
        }

        // Check if any of the features enables the "fullstack" feature
        // todo!("check if any of the features enables the fullstack feature");

        false
    }

    // /// We always put the server in the `web` folder!
    // /// Only the `web` target will generate a `public` folder though
    // async fn write_server_executable(&self) -> Result<()> {
    //     if let Some(server) = &self.server {
    //         let to = self
    //             .server_exe()
    //             .expect("server should be set if we're building a server");

    //         std::fs::create_dir_all(self.server_exe().unwrap().parent().unwrap())?;

    //         tracing::debug!("Copying server executable to: {to:?} {server:#?}");

    //         // Remove the old server executable if it exists, since copying might corrupt it :(
    //         // todo(jon): do this in more places, I think
    //         _ = std::fs::remove_file(&to);
    //         std::fs::copy(&server.exe, to)?;
    //     }

    //     Ok(())
    // }

    /// todo(jon): use handlebars templates instead of these prebaked templates
    async fn write_metadata(&self) -> Result<()> {
        // write the Info.plist file
        match self.platform {
            Platform::MacOS => {
                let dest = self.root_dir().join("Contents").join("Info.plist");
                let plist = self.info_plist_contents(self.platform)?;
                std::fs::write(dest, plist)?;
            }

            Platform::Ios => {
                let dest = self.root_dir().join("Info.plist");
                let plist = self.info_plist_contents(self.platform)?;
                std::fs::write(dest, plist)?;
            }

            // AndroidManifest.xml
            // er.... maybe even all the kotlin/java/gradle stuff?
            Platform::Android => {}

            // Probably some custom format or a plist file (haha)
            // When we do the proper bundle, we'll need to do something with wix templates, I think?
            Platform::Windows => {}

            // eventually we'll create the .appimage file, I guess?
            Platform::Linux => {}

            // These are served as folders, not appimages, so we don't need to do anything special (I think?)
            // Eventually maybe write some secrets/.env files for the server?
            // We could also distribute them as a deb/rpm for linux and msi for windows
            Platform::Web => {}
            Platform::Server => {}
            Platform::Liveview => {}
        }

        Ok(())
    }

    /// Run the optimizers, obfuscators, minimizers, signers, etc
    pub(crate) async fn optimize(&self, ctx: &BuildContext) -> Result<()> {
        match self.platform {
            Platform::Web => {
                // Compress the asset dir
                // If pre-compressing is enabled, we can pre_compress the wasm-bindgen output
                let pre_compress = self.should_pre_compress_web_assets(self.release);

                ctx.status_compressing_assets();
                let asset_dir = self.asset_dir();
                tokio::task::spawn_blocking(move || {
                    crate::fastfs::pre_compress_folder(&asset_dir, pre_compress)
                })
                .await
                .unwrap()?;
            }
            Platform::MacOS => {}
            Platform::Windows => {}
            Platform::Linux => {}
            Platform::Ios => {}
            Platform::Android => {}
            Platform::Server => {}
            Platform::Liveview => {}
        }

        Ok(())
    }

    // pub(crate) fn server_exe(&self) -> Option<PathBuf> {
    //     if let Some(_server) = &self.server {
    //         let mut path = self.build_dir(Platform::Server, self.release);

    //         if cfg!(windows) {
    //             path.push("server.exe");
    //         } else {
    //             path.push("server");
    //         }

    //         return Some(path);
    //     }

    //     None
    // }

    /// Bundle the web app
    /// - Run wasm-bindgen
    /// - Bundle split
    /// - Run wasm-opt
    /// - Register the .wasm and .js files with the asset system
    async fn bundle_web(
        &self,
        ctx: &BuildContext,
        exe: &Path,
        assets: &mut AssetManifest,
    ) -> Result<()> {
        use crate::{wasm_bindgen::WasmBindgen, wasm_opt};
        use std::fmt::Write;

        // Locate the output of the build files and the bindgen output
        // We'll fill these in a second if they don't already exist
        let bindgen_outdir = self.wasm_bindgen_out_dir();
        let prebindgen = exe.clone();
        let post_bindgen_wasm = self.wasm_bindgen_wasm_output_file();
        let should_bundle_split: bool = self.wasm_split;
        let rustc_exe = exe.with_extension("wasm");
        let bindgen_version = self
            .wasm_bindgen_version()
            .expect("this should have been checked by tool verification");

        // Prepare any work dirs
        std::fs::create_dir_all(&bindgen_outdir)?;

        // Prepare our configuration
        //
        // we turn off debug symbols in dev mode but leave them on in release mode (weird!) since
        // wasm-opt and wasm-split need them to do better optimizations.
        //
        // We leave demangling to false since it's faster and these tools seem to prefer the raw symbols.
        // todo(jon): investigate if the chrome extension needs them demangled or demangles them automatically.
        let will_wasm_opt = (self.release || self.wasm_split) && self.workspace.wasm_opt.is_some();
        let keep_debug = self.config.web.wasm_opt.debug
            || self.debug_symbols
            || self.wasm_split
            || !self.release
            || will_wasm_opt;
        let demangle = false;
        let wasm_opt_options = WasmOptConfig {
            memory_packing: self.wasm_split,
            debug: self.debug_symbols,
            ..self.config.web.wasm_opt.clone()
        };

        // Run wasm-bindgen. Some of the options are not "optimal" but will be fixed up by wasm-opt
        //
        // There's performance implications here. Running with --debug is slower than without
        // We're keeping around lld sections and names but wasm-opt will fix them
        // todo(jon): investigate a good balance of wiping debug symbols during dev (or doing a double build?)
        ctx.status_wasm_bindgen_start();
        tracing::debug!(dx_src = ?TraceSrc::Bundle, "Running wasm-bindgen");
        let start = std::time::Instant::now();
        WasmBindgen::new(&bindgen_version)
            .input_path(&rustc_exe)
            .target("web")
            .debug(keep_debug)
            .demangle(demangle)
            .keep_debug(keep_debug)
            .keep_lld_sections(true)
            .out_name(self.executable_name())
            .out_dir(&bindgen_outdir)
            .remove_name_section(!will_wasm_opt)
            .remove_producers_section(!will_wasm_opt)
            .run()
            .await
            .context("Failed to generate wasm-bindgen bindings")?;
        tracing::debug!(dx_src = ?TraceSrc::Bundle, "wasm-bindgen complete in {:?}", start.elapsed());

        // Run bundle splitting if the user has requested it
        // It's pretty expensive but because of rayon should be running separate threads, hopefully
        // not blocking this thread. Dunno if that's true
        if should_bundle_split {
            ctx.status_splitting_bundle();

            if !will_wasm_opt {
                return Err(anyhow::anyhow!(
                    "Bundle splitting requires wasm-opt to be installed or the CLI to be built with `--features optimizations`. Please install wasm-opt and try again."
                )
                .into());
            }

            // Load the contents of these binaries since we need both of them
            // We're going to use the default makeLoad glue from wasm-split
            let original = std::fs::read(&prebindgen)?;
            let bindgened = std::fs::read(&post_bindgen_wasm)?;
            let mut glue = wasm_split_cli::MAKE_LOAD_JS.to_string();

            // Run the emitter
            let splitter = wasm_split_cli::Splitter::new(&original, &bindgened);
            let modules = splitter
                .context("Failed to parse wasm for splitter")?
                .emit()
                .context("Failed to emit wasm split modules")?;

            // Write the chunks that contain shared imports
            // These will be in the format of chunk_0_modulename.wasm - this is hardcoded in wasm-split
            tracing::debug!("Writing split chunks to disk");
            for (idx, chunk) in modules.chunks.iter().enumerate() {
                let path = bindgen_outdir.join(format!("chunk_{}_{}.wasm", idx, chunk.module_name));
                wasm_opt::write_wasm(&chunk.bytes, &path, &wasm_opt_options).await?;
                writeln!(
                    glue, "export const __wasm_split_load_chunk_{idx} = makeLoad(\"/assets/{url}\", [], fusedImports);",
                    url = assets
                        .register_asset(&path, AssetOptions::Unknown)?.bundled_path(),
                )?;
            }

            // Write the modules that contain the entrypoints
            tracing::debug!("Writing split modules to disk");
            for (idx, module) in modules.modules.iter().enumerate() {
                let comp_name = module
                    .component_name
                    .as_ref()
                    .context("generated bindgen module has no name?")?;

                let path = bindgen_outdir.join(format!("module_{}_{}.wasm", idx, comp_name));
                wasm_opt::write_wasm(&module.bytes, &path, &wasm_opt_options).await?;

                let hash_id = module.hash_id.as_ref().unwrap();

                writeln!(
                    glue,
                    "export const __wasm_split_load_{module}_{hash_id}_{comp_name} = makeLoad(\"/assets/{url}\", [{deps}], fusedImports);",
                    module = module.module_name,


                    // Again, register this wasm with the asset system
                    url = assets
                        .register_asset(&path, AssetOptions::Unknown)?.bundled_path(),

                    // This time, make sure to write the dependencies of this chunk
                    // The names here are again, hardcoded in wasm-split - fix this eventually.
                    deps = module
                        .relies_on_chunks
                        .iter()
                        .map(|idx| format!("__wasm_split_load_chunk_{idx}"))
                        .collect::<Vec<_>>()
                        .join(", ")
                )?;
            }

            // Write the js binding
            // It's not registered as an asset since it will get included in the main.js file
            let js_output_path = bindgen_outdir.join("__wasm_split.js");
            std::fs::write(&js_output_path, &glue)?;

            // Make sure to write some entropy to the main.js file so it gets a new hash
            // If we don't do this, the main.js file will be cached and never pick up the chunk names
            let uuid = uuid::Uuid::new_v5(&uuid::Uuid::NAMESPACE_URL, glue.as_bytes());
            std::fs::OpenOptions::new()
                .append(true)
                .open(self.wasm_bindgen_js_output_file())
                .context("Failed to open main.js file")?
                .write_all(format!("/*{uuid}*/").as_bytes())?;

            // Write the main wasm_bindgen file and register it with the asset system
            // This will overwrite the file in place
            // We will wasm-opt it in just a second...
            std::fs::write(&post_bindgen_wasm, modules.main.bytes)?;
        }

        // Make sure to optimize the main wasm file if requested or if bundle splitting
        if should_bundle_split || self.release {
            ctx.status_optimizing_wasm();
            wasm_opt::optimize(&post_bindgen_wasm, &post_bindgen_wasm, &wasm_opt_options).await?;
        }

        // Make sure to register the main wasm file with the asset system
        assets.register_asset(&post_bindgen_wasm, AssetOptions::Unknown)?;

        // Register the main.js with the asset system so it bundles in the snippets and optimizes
        assets.register_asset(
            &self.wasm_bindgen_js_output_file(),
            AssetOptions::Js(JsAssetOptions::new().with_minify(true).with_preload(true)),
        )?;

        // Write the index.html file with the pre-configured contents we got from pre-rendering
        std::fs::write(
            self.root_dir().join("index.html"),
            self.prepare_html(&assets)?,
        )?;

        Ok(())
    }

    fn info_plist_contents(&self, platform: Platform) -> Result<String> {
        #[derive(serde::Serialize)]
        pub struct InfoPlistData {
            pub display_name: String,
            pub bundle_name: String,
            pub bundle_identifier: String,
            pub executable_name: String,
        }

        match platform {
            Platform::MacOS => handlebars::Handlebars::new()
                .render_template(
                    include_str!("../../assets/macos/mac.plist.hbs"),
                    &InfoPlistData {
                        display_name: self.bundled_app_name(),
                        bundle_name: self.bundled_app_name(),
                        executable_name: self.platform_exe_name(),
                        bundle_identifier: self.bundle_identifier(),
                    },
                )
                .map_err(|e| e.into()),
            Platform::Ios => handlebars::Handlebars::new()
                .render_template(
                    include_str!("../../assets/ios/ios.plist.hbs"),
                    &InfoPlistData {
                        display_name: self.bundled_app_name(),
                        bundle_name: self.bundled_app_name(),
                        executable_name: self.platform_exe_name(),
                        bundle_identifier: self.bundle_identifier(),
                    },
                )
                .map_err(|e| e.into()),
            _ => Err(anyhow::anyhow!("Unsupported platform for Info.plist").into()),
        }
    }

    /// Run any final tools to produce apks or other artifacts we might need.
    ///
    /// This might include codesigning, zipping, creating an appimage, etc
    async fn assemble(&self, ctx: &BuildContext) -> Result<()> {
        if let Platform::Android = self.platform {
            ctx.status_running_gradle();

            let output = Command::new(self.gradle_exe()?)
                .arg("assembleDebug")
                .current_dir(self.root_dir())
                .output()
                .await?;

            if !output.status.success() {
                return Err(anyhow::anyhow!("Failed to assemble apk: {output:?}").into());
            }
        }

        Ok(())
    }

    /// Run bundleRelease and return the path to the `.aab` file
    ///
    /// https://stackoverflow.com/questions/57072558/whats-the-difference-between-gradlewassemblerelease-gradlewinstallrelease-and
    pub(crate) async fn android_gradle_bundle(&self) -> Result<PathBuf> {
        let output = Command::new(self.gradle_exe()?)
            .arg("bundleRelease")
            .current_dir(self.root_dir())
            .output()
            .await
            .context("Failed to run gradle bundleRelease")?;

        if !output.status.success() {
            return Err(anyhow::anyhow!("Failed to bundleRelease: {output:?}").into());
        }

        let app_release = self
            .root_dir()
            .join("app")
            .join("build")
            .join("outputs")
            .join("bundle")
            .join("release");

        // Rename it to Name-arch.aab
        let from = app_release.join("app-release.aab");
        let to = app_release.join(format!("{}-{}.aab", self.bundled_app_name(), self.triple));

        std::fs::rename(from, &to).context("Failed to rename aab")?;

        Ok(to)
    }

    fn gradle_exe(&self) -> Result<PathBuf> {
        // make sure we can execute the gradlew script
        #[cfg(unix)]
        {
            use std::os::unix::prelude::PermissionsExt;
            std::fs::set_permissions(
                self.root_dir().join("gradlew"),
                std::fs::Permissions::from_mode(0o755),
            )?;
        }

        let gradle_exec_name = match cfg!(windows) {
            true => "gradlew.bat",
            false => "gradlew",
        };

        Ok(self.root_dir().join(gradle_exec_name))
    }

    pub(crate) fn apk_path(&self) -> PathBuf {
        self.root_dir()
            .join("app")
            .join("build")
            .join("outputs")
            .join("apk")
            .join("debug")
            .join("app-debug.apk")
    }

    /// We only really currently care about:
    ///
    /// - app dir (.app, .exe, .apk, etc)
    /// - assetas dir
    /// - exe dir (.exe, .app, .apk, etc)
    /// - extra scaffolding
    ///
    /// It's not guaranteed that they're different from any other folder
    pub fn prepare_build_dir(&self) -> Result<()> {
        use once_cell::sync::OnceCell;
        use std::fs::{create_dir_all, remove_dir_all};

        static INITIALIZED: OnceCell<Result<()>> = OnceCell::new();

        let success = INITIALIZED.get_or_init(|| {
            _ = remove_dir_all(self.exe_dir());

            self.flush_session_cache();

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
                .join(AndroidTools::android_jnilib(&self.triple)),

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
            .join(self.executable_name())
            .with_extension("js")
    }

    /// Get the path to the wasm bindgen wasm output file
    pub fn wasm_bindgen_wasm_output_file(&self) -> PathBuf {
        self.wasm_bindgen_out_dir()
            .join(format!("{}_bg", self.executable_name()))
            .with_extension("wasm")
    }

    /// Get the path to the asset optimizer version file
    pub fn asset_optimizer_version_file(&self) -> PathBuf {
        self.platform_dir().join(".cli-version")
    }

    pub fn flush_session_cache(&self) {
        let cache_dir = self.session_cache_dir();
        _ = std::fs::remove_dir_all(&cache_dir);
        _ = std::fs::create_dir_all(&cache_dir);
    }
}

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
