//! # BuildRequest - the core of the build process
//!
//! The BuildRequest object is the core of the build process. It contains all the resolved arguments
//! flowing in from the CLI, dioxus.toml, env vars, and the workspace.
//!
//! Every BuildRequest is tied to a given workspace and BuildArgs. For simplicity's sake, the BuildArgs
//! struct is used to represent the CLI arguments and all other configuration is basically just
//! extra CLI arguments, but in a configuration format.
//!
//! When [`BuildRequest::build`] is called, it will prepare its work directory in the target folder
//! and then start running the build process. A [`BuildContext`] is required to customize this
//! build process, containing a channel for progress updates and the build mode.
//!
//! The [`BuildMode`] is extremely important since it influences how the build is performed. Most
//! "normal" builds just use [`BuildMode::Base`], but we also support [`BuildMode::Fat`] and
//! [`BuildMode::Thin`]. These builds are used together to power the hot-patching and fast-linking
//! engine.
//!
//! The BuildRequest is also responsible for writing the final build artifacts to disk. This includes
//!
//! - Writing the executable
//! - Processing assets from the artifact
//! - Writing any metadata or configuration files (Info.plist, AndroidManifest.xml)
//! - Bundle splitting (for wasm) and wasm-bindgen
//!
//! In some cases, the BuildRequest also handles the linking of the final executable. Specifically,
//! - For Android, we use `dx` as an opaque linker to dynamically find the true android linker
//! - For hotpatching, the CLI manually links the final executable with a stub file
//!
//! ## Build formats:
//!
//! We support building for the most popular platforms:
//! - Web via wasm-bindgen
//! - macOS via app-bundle
//! - iOS via app-bundle
//! - Android via gradle
//! - Linux via app-image
//! - Windows via exe, msi/msix
//!
//! Note that we are missing some setups that we *should* support:
//! - PWAs, WebWorkers, ServiceWorkers
//! - Web Extensions
//! - Linux via flatpak/snap
//!
//! There are some less popular formats that we might want to support eventually:
//! - TVOS, watchOS
//! - OpenHarmony
//!
//! Also, some deploy platforms have their own bespoke formats:
//! - Cloudflare workers
//! - AWS Lambda
//!
//! Currently, we defer most of our deploy-based bundling to Tauri bundle, though we should migrate
//! to just bundling everything ourselves. This would require us to implement code-signing which
//! is a bit of a pain, but fortunately a solved process (https://github.com/rust-mobile/xbuild).
//!
//! ## Build Structure
//!
//! Builds generally follow the same structure everywhere:
//! - A main executable
//! - Sidecars (alternate entrypoints, framewrok plugins, etc)
//! - Assets (images, fonts, etc)
//! - Metadata (Info.plist, AndroidManifest.xml)
//! - Glue code (java, kotlin, javascript etc)
//! - Entitlements for code-signing and verification
//!
//! We need to be careful to not try and put a "round peg in a square hole," but most platforms follow
//! the same pattern.
//!
//! As such, we try to assemble a build directory that's somewhat sensible:
//! - A main "staging" dir for a given app
//! - Per-profile dirs (debug/release)
//! - A platform dir (ie web/desktop/android/ios)
//! - The "bundle" dir which is basically the `.app` format or `wwww` dir.
//! - The "executable" dir where the main exe is housed
//! - The "assets" dir where the assets are housed
//! - The "meta" dir where stuff like Info.plist, AndroidManifest.xml, etc are housed
//!
//! There's also some "quirky" folders that need to be stable between builds but don't influence the
//! bundle itself:
//! - session_cache_dir which stores stuff like window position
//!
//! ### Web:
//!
//! Create a folder that is somewhat similar to an app-image (exe + asset)
//! The server is dropped into the `web` folder, even if there's no `public` folder.
//! If there's no server (SPA), we still use the `web` folder, but it only contains the
//! public folder.
//!
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
//! ### Linux:
//!
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
//! ### Macos
//!
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
//! ### iOS
//!
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
//! ### Android:
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
//! ### Windows:
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
//! ## Bundle structure links
//! - apple: https://developer.apple.com/documentation/bundleresources/placing_content_in_a_bundle
//! - appimage: https://docs.appimage.org/packaging-guide/manual.html#ref-manual
//!
//! ## Extra links
//! - xbuild: https://github.com/rust-mobile/xbuild/blob/master/xbuild/src/command/build.rs

use super::{android_tools, AndroidTools, BuildContext, BuildId};
use crate::{
    rustcwrapper::{RustcArgs, DX_RUSTC_WRAPPER_ENV_VAR},
    wasm_bindgen::WasmBindgen,
    BuildArgs, DioxusConfig, Error, LinkAction, Platform, ProgressTx, Result, TraceSrc,
    WasmOptConfig, Workspace,
};
use anyhow::Context;
use dioxus_cli_config::{APP_TITLE_ENV, ASSET_ROOT_ENV};
use dioxus_cli_opt::{process_file_to, AssetManifest};
use itertools::Itertools;
use krates::{cm::TargetKind, KrateDetails, Krates, NodeId, Utf8PathBuf};
use manganis::{AssetOptions, JsAssetOptions};
use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    future::Future,
    io::Write,
    path::{Path, PathBuf},
    pin::Pin,
    process::Stdio,
    str::FromStr,
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
use tempfile::NamedTempFile;
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
    pub(crate) extra_cargo_args: Vec<String>,

    /// Extra arguments to pass to rustc
    pub(crate) extra_rustc_args: Vec<String>,

    /// Don't include the default features in the build
    pub(crate) no_default_features: bool,

    /// The target directory for the build
    pub(crate) custom_target_dir: Option<PathBuf>,

    pub(crate) cranelift: bool,

    pub(crate) skip_assets: bool,

    pub(crate) wasm_split: bool,

    pub(crate) debug_symbols: bool,

    pub(crate) inject_loading_scripts: bool,

    pub(crate) custom_linker: Option<PathBuf>,

    pub(crate) link_args_file: Arc<NamedTempFile>,

    pub(crate) link_err_file: Arc<NamedTempFile>,

    pub(crate) rustc_wrapper_args_file: Arc<NamedTempFile>,
}

/// dx can produce different "modes" of a build. A "regular" build is a "base" build. The Fat and Thin
/// modes are used together to achieve binary patching and linking.
///
/// Guide:
/// ----------
/// - Base: A normal build generated using `cargo rustc`, intended for production use cases
///
/// - Fat: A "fat" build with -Wl,-all_load and no_dead_strip, keeping *every* symbol in the binary.
///        Intended for development for larger up-front builds with faster link times and the ability
///        to binary patch the final binary. On WASM, this also forces wasm-bindgen to generate all
///        JS-WASM bindings, saving us the need to re-wasmbindgen the final binary.
///
/// - Thin: A "thin" build that dynamically links against the dependencies produced by the "fat" build.
///         This is generated by calling rustc *directly* and might be more fragile to construct, but
///         generates *much* faster than a regular base or fat build.
#[derive(Clone, Debug, PartialEq)]
pub enum BuildMode {
    /// A normal build generated using `cargo rustc`
    Base,

    /// A "Fat" build generated with cargo rustc and dx as a custom linker without -Wl,-dead-strip
    Fat,

    /// A "thin" build generated with `rustc` directly and dx as a custom linker
    Thin {
        rustc_args: RustcArgs,
        changed_files: Vec<PathBuf>,
        aslr_reference: Option<u64>,
    },
}

/// The end result of a build.
///
/// Contains the final asset manifest, the executable, and metadata about the build.
/// Note that the `exe` might be stale and/or overwritten by the time you read it!
#[derive(Clone, Debug)]
pub struct BuildArtifacts {
    pub(crate) platform: Platform,
    pub(crate) exe: PathBuf,
    pub(crate) direct_rustc: RustcArgs,
    pub(crate) time_start: SystemTime,
    pub(crate) time_end: SystemTime,
    pub(crate) assets: AssetManifest,
    pub(crate) mode: BuildMode,
}

pub(crate) static PROFILE_WASM: &str = "wasm-dev";
pub(crate) static PROFILE_ANDROID: &str = "android-dev";
pub(crate) static PROFILE_SERVER: &str = "server-dev";

impl BuildRequest {
    /// Create a new build request.
    ///
    /// This method consolidates various inputs into a single source of truth. It combines:
    /// - Command-line arguments provided by the user.
    /// - The crate's `Cargo.toml`.
    /// - The `dioxus.toml` configuration file.
    /// - User-specific CLI settings.
    /// - The workspace metadata.
    /// - Host-specific details (e.g., Android tools, installed frameworks).
    /// - The intended target platform.
    ///
    /// Fields may be duplicated from the inputs to allow for autodetection and resolution.
    ///
    /// Autodetection is performed for unspecified fields where possible.
    ///
    /// Note: Build requests are typically created only when the CLI is invoked or when significant
    /// changes are detected in the `Cargo.toml` (e.g., features added or removed).
    pub(crate) async fn new(args: &BuildArgs, workspace: Arc<Workspace>) -> Result<Self> {
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
        // todo: I think this might be wrong - we don't want to use main_package necessarily...
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
        let triple = match args.target.clone() {
            Some(target) => target,
            None => match platform {
                // Generally just use the host's triple for native executables unless specified otherwise
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
                        .autodetect_android_device_triple()
                        .await
                }
            },
        };

        let custom_linker = if platform == Platform::Android {
            let tools = android_tools().context("Failed to find your Android NDK setup")?;
            Some(tools.android_cc(&triple))
        } else {
            None
        };

        // Set up some tempfiles so we can do some IPC between us and the linker/rustc wrapper (which is occasionally us!)
        let link_args_file = Arc::new(
            NamedTempFile::new().context("Failed to create temporary file for linker args")?,
        );
        let link_err_file = Arc::new(
            NamedTempFile::new().context("Failed to create temporary file for linker args")?,
        );
        let rustc_wrapper_args_file = Arc::new(
            NamedTempFile::new()
                .context("Failed to create temporary file for rustc wrapper args")?,
        );

        let extra_rustc_args = shell_words::split(&args.rustc_args.clone().unwrap_or_default())
            .context("Failed to parse rustc args")?;

        Ok(Self {
            platform,
            features,
            no_default_features,
            crate_package,
            crate_target,
            profile,
            triple,
            device,
            workspace,
            config,
            custom_target_dir: None,
            custom_linker,
            link_args_file,
            link_err_file,
            rustc_wrapper_args_file,
            extra_rustc_args,
            extra_cargo_args: args.cargo_args.clone(),
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
        // If we forget to do this, then we won't get the linker args since rust skips the full build
        // We need to make sure to not react to this though, so the filemap must cache it
        _ = self.bust_fingerprint(&ctx);

        // Run the cargo build to produce our artifacts
        let mut artifacts = self.cargo_build(&ctx).await?;

        // Write the build artifacts to the bundle on the disk
        match ctx.mode {
            BuildMode::Thin { aslr_reference, .. } => {
                self.write_patch(ctx, aslr_reference, &mut artifacts)
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

    /// Run the cargo build by assembling the build command and executing it.
    ///
    /// This method needs to be very careful with processing output since errors being swallowed will
    /// be very confusing to the user.
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
                Message::CompilerMessage(msg) => ctx.status_build_diagnostic(msg),
                Message::TextLine(line) => {
                    // Handle the case where we're getting lines directly from rustc.
                    // These are in a different format than the normal cargo output, though I imagine
                    // this parsing code is quite fragile/sensitive to changes in cargo, cargo_metadta, rustc, etc.
                    #[derive(Deserialize)]
                    struct RustcArtifact {
                        artifact: PathBuf,
                        emit: String,
                    }

                    // These outputs look something like:
                    //
                    // { "artifact":"target/debug/deps/libdioxus_core-4f2a0b3c1e5f8b7c.rlib", "emit":"link" }
                    //
                    // There are other outputs like depinfo that we might be interested in in the future.
                    if let Ok(artifact) = serde_json::from_str::<RustcArtifact>(&line) {
                        if artifact.emit == "link" {
                            output_location = Some(artifact.artifact);
                        }
                    }

                    // For whatever reason, if there's an error while building, we still receive the TextLine
                    // instead of an "error" message. However, the following messages *also* tend to
                    // be the error message, and don't start with "error:". So we'll check if we've already
                    // emitted an error message and if so, we'll emit all following messages as errors too.
                    //
                    // todo: This can lead to some really ugly output though, so we might want to look
                    // into a more reliable way to detect errors propagating out of the compiler. If
                    // we always wrapped rustc, then we could store this data somewhere in a much more
                    // reliable format.
                    if line.trim_start().starts_with("error:") {
                        emitting_error = true;
                    }

                    // Note that previous text lines might have set emitting_error to true
                    match emitting_error {
                        true => ctx.status_build_error(line),
                        false => ctx.status_build_message(line),
                    }
                }
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
                // todo: this can occasionally swallow errors, so we should figure out what exactly is going wrong
                //       since that is a really bad user experience.
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
        let platform = self.platform;
        tracing::debug!("Build completed successfully - output location: {:?}", exe);

        // Accumulate the rustc args from the wrapper, if they exist and can be parsed.
        let mut direct_rustc = RustcArgs::default();
        if let Ok(res) = std::fs::read_to_string(self.rustc_wrapper_args_file.path()) {
            if let Ok(res) = serde_json::from_str(&res) {
                direct_rustc = res;
            }
        }

        Ok(BuildArtifacts {
            platform,
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
        tracing::debug!("Collecting assets from exe at {} ...", exe.display());

        // walk every file in the incremental cache dir, reading and inserting items into the manifest.
        let mut manifest = AssetManifest::default();

        // And then add from the exe directly, just in case it's LTO compiled and has no incremental cache
        if !self.skip_assets {
            _ = manifest.add_from_object_path(exe);
        }

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
            //                 server.exe
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
                // We wipe away the dir completely, which is not great behavior :/
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

    /// Patches are stored in the same directory as the main executable, but with a name based on the
    /// time the patch started compiling.
    ///
    /// - lib{name}-patch-{time}.(so/dll/dylib) (next to the main exe)
    ///
    /// Note that weirdly enough, the name of dylibs can actually matter. In some environments, libs
    /// can override each other with symbol interposition.
    ///
    /// Also, on Android - and some Linux, we *need* to start the lib name with `lib` for the dynamic
    /// loader to consider it a shared library.
    ///
    /// todo: the time format might actually be problematic if two platforms share the same build folder.
    pub(crate) fn patch_exe(&self, time_start: SystemTime) -> PathBuf {
        let path = self.main_exe().with_file_name(format!(
            "lib{}-patch-{}",
            self.executable_name(),
            time_start.duration_since(UNIX_EPOCH).unwrap().as_millis(),
        ));

        let extension = match self.triple.operating_system {
            OperatingSystem::Darwin(_) => "dylib",
            OperatingSystem::MacOSX(_) => "dylib",
            OperatingSystem::IOS(_) => "dylib",
            OperatingSystem::Windows => "dll",
            OperatingSystem::Linux => "so",
            OperatingSystem::Wasi => "wasm",
            OperatingSystem::Unknown if self.platform == Platform::Web => "wasm",
            _ => "",
        };

        path.with_extension(extension)
    }

    /// Run our custom linker setup to generate a patch file in the right location
    ///
    /// This should be the only case where the cargo output is a "dummy" file and requires us to
    /// manually do any linking.
    ///
    /// We also run some post processing steps here, like extracting out any new assets.
    async fn write_patch(
        &self,
        ctx: &BuildContext,
        aslr_reference: Option<u64>,
        artifacts: &mut BuildArtifacts,
    ) -> Result<()> {
        tracing::debug!("Patching existing bundle");
        ctx.status_hotpatching();

        let raw_args = std::fs::read_to_string(&self.link_args_file.path())
            .context("Failed to read link args from file")?;
        let args = raw_args.lines().collect::<Vec<_>>();

        // Extract out the incremental object files.
        //
        // This is sadly somewhat of a hack, but it might be a moderately reliable hack.
        //
        // When rustc links your project, it passes the args as how a linker would expect, but with
        // a somehwat reliable ordering. These are all internal details to cargo/rustc, so we can't
        // rely on them *too* much, but the *are* fundamental to how rust compiles your projects, and
        // linker interfaces probably won't change drastically for another 40 years.
        //
        // We need to tear apart this command and only pass the args that are relevant to our thin link.
        // Mainly, we don't want any rlibs to be linked. Occasionally some libraries like objc_exception
        // export a folder with their artifacts - unsure if we actually need to include them. Generally
        // you can err on the side that most *libraries* don't need to be linked here since dlopen
        // satisfies those symbols anyways when the binary is loaded.
        //
        // Many args are passed twice, too, which can be confusing, but generally don't have any real
        // effect. Note that on macos/ios, there's a special macho header that needs to be set, otherwise
        // dyld will complain.a
        //
        // Also, some flags in darwin land might become deprecated, need to be super conservative:
        // - https://developer.apple.com/forums/thread/773907
        //
        // The format of this command roughly follows:
        // ```
        // clang
        //     /dioxus/target/debug/subsecond-cli
        //     /var/folders/zs/gvrfkj8x33d39cvw2p06yc700000gn/T/rustcAqQ4p2/symbols.o
        //     /dioxus/target/subsecond-dev/deps/subsecond_harness-acfb69cb29ffb8fa.05stnb4bovskp7a00wyyf7l9s.rcgu.o
        //     /dioxus/target/subsecond-dev/deps/subsecond_harness-acfb69cb29ffb8fa.08rgcutgrtj2mxoogjg3ufs0g.rcgu.o
        //     /dioxus/target/subsecond-dev/deps/subsecond_harness-acfb69cb29ffb8fa.0941bd8fa2bydcv9hfmgzzne9.rcgu.o
        //     /dioxus/target/subsecond-dev/deps/libbincode-c215feeb7886f81b.rlib
        //     /dioxus/target/subsecond-dev/deps/libanyhow-e69ac15c094daba6.rlib
        //     /dioxus/target/subsecond-dev/deps/libratatui-c3364579b86a1dfc.rlib
        //     /.rustup/toolchains/stable-aarch64-apple-darwin/lib/rustlib/aarch64-apple-darwin/lib/libstd-019f0f6ae6e6562b.rlib
        //     /.rustup/toolchains/stable-aarch64-apple-darwin/lib/rustlib/aarch64-apple-darwin/lib/libpanic_unwind-7387d38173a2eb37.rlib
        //     /.rustup/toolchains/stable-aarch64-apple-darwin/lib/rustlib/aarch64-apple-darwin/lib/libobject-2b03cf6ece171d21.rlib
        //     -framework AppKit
        //     -lc
        //     -framework Foundation
        //     -framework Carbon
        //     -lSystem
        //     -framework CoreFoundation
        //     -lobjc
        //     -liconv
        //     -lm
        //     -arch arm64
        //     -mmacosx-version-min=11.0.0
        //     -L /dioxus/target/subsecond-dev/build/objc_exception-dc226cad0480ea65/out
        //     -o /dioxus/target/subsecond-dev/deps/subsecond_harness-acfb69cb29ffb8fa
        //     -nodefaultlibs
        //     -Wl,-all_load
        // ```
        let mut object_files = args
            .iter()
            .filter(|arg| arg.ends_with(".rcgu.o"))
            .sorted()
            .map(|arg| PathBuf::from(arg))
            .collect::<Vec<_>>();

        // On non-wasm platforms, we generate a special shim object file which converts symbols from
        // fat binary into direct addresses from the running process.
        //
        // Our wasm approach is quite specific to wasm. We don't need to resolve any missing symbols
        // there since wasm is relocatable, but there is considerable pre and post processing work to
        // satisfy undefined symbols that we do by munging the binary directly.
        //
        // todo: can we adjust our wasm approach to also use a similar system?
        // todo: don't require the aslr reference and just patch the got when loading.
        //
        // Requiring the ASLR offset here is necessary but unfortunately might be flakey in practice.
        // Android apps can take a long time to open, and a hot patch might've been issued in the interim,
        // making this hotpatch a failure.
        if self.platform != Platform::Web {
            let stub_bytes = crate::build::resolve_undefined(
                &self.main_exe(),
                &object_files,
                &self.triple,
                aslr_reference.context("ASLR reference not found - is the client connected?")?,
            )
            .expect("failed to resolve patch symbols");

            // Currently we're dropping stub.o in the exe dir, but should probably just move to a tempfile?
            let patch_file = self.main_exe().with_file_name("stub.o");
            std::fs::write(&patch_file, stub_bytes)?;
            object_files.push(patch_file);
        }

        let cc = match self.platform {
            // todo: we're using wasm-ld directly, but I think we can drive it with rust-lld and -flavor wasm
            Platform::Web => self.workspace.wasm_ld(),

            // The android clang linker is *special* and has some android-specific flags that we need
            //
            // Note that this is *clang*, not `lld`.
            Platform::Android => android_tools()
                .context("Could not determine android tools")?
                .android_cc(&self.triple),

            // The rest of the platforms use `cc` as the linker which should be available in your path,
            // provided you have build-tools setup. On mac/linux this is the default, but on Windows
            // it requires msvc or gnu downloaded, which is a requirement to use rust anyways.
            //
            // The default linker might actually be slow though, so we could consider using lld or rust-lld
            // since those are shipping by default on linux as of 1.86. Window's linker is the really slow one.
            //
            // https://blog.rust-lang.org/2024/05/17/enabling-rust-lld-on-linux.html
            //
            // Note that "cc" is *not* a linker. It's a compiler! The arguments we pass need to be in
            // the form of `-Wl,<args>` for them to make it to the linker. This matches how rust does it
            // which is confusing.
            Platform::MacOS
            | Platform::Ios
            | Platform::Linux
            | Platform::Server
            | Platform::Liveview
            | Platform::Windows => PathBuf::from("cc"),
        };

        // Run the linker directly!
        //
        // We dump its output directly into the patch exe location which is different than how rustc
        // does it since it uses llvm-objcopy into the `target/debug/` folder.
        let res = Command::new(cc)
            .args(object_files.iter())
            .args(self.thin_link_args(&args)?)
            .arg("-o")
            .arg(&self.patch_exe(artifacts.time_start))
            .output()
            .await?;

        if !res.stderr.is_empty() {
            let errs = String::from_utf8_lossy(&res.stderr);
            if !self.patch_exe(artifacts.time_start).exists() {
                tracing::error!("Failed to generate patch: {}", errs.trim());
            } else {
                tracing::debug!("Warnings during thin linking: {}", errs.trim());
            }
        }

        // For some really weird reason that I think is because of dlopen caching, future loads of the
        // jump library will fail if we don't remove the original fat file. I think this could be
        // because of library versioning and namespaces, but really unsure.
        //
        // The errors if you forget to do this are *extremely* cryptic - missing symbols that never existed.
        //
        // Fortunately, this binary exists in two places - the deps dir and the target out dir. We
        // can just remove the one in the deps dir and the problem goes away.
        if let Some(idx) = args.iter().position(|arg| *arg == "-o") {
            _ = std::fs::remove_file(&PathBuf::from(args[idx + 1]));
        }

        // Also clean up the temp artifacts
        // // Clean up the temps manually
        // // todo: we might want to keep them around for debugging purposes
        // for file in object_files {
        //     _ = std::fs::remove_file(file);
        // }

        Ok(())
    }

    /// Take the original args passed to the "fat" build and then create the "thin" variant.
    ///
    /// This is basically just stripping away the rlibs and other libraries that will be satisfied
    /// by our stub step.
    fn thin_link_args(&self, original_args: &[&str]) -> Result<Vec<String>> {
        use target_lexicon::OperatingSystem;

        let triple = self.triple.clone();
        let mut out_args = vec![];

        tracing::trace!("original args:\n{}", original_args.join(" "));

        match triple.operating_system {
            // wasm32-unknown-unknown -> use wasm-ld (gnu-lld)
            //
            // We need to import a few things - namely the memory and ifunc table.
            //
            // We can safely export everything, I believe, though that led to issues with the "fat"
            // binaries that also might lead to issues here too. wasm-bindgen chokes on some symbols
            // and the resulting JS has issues.
            //
            // We turn on both --pie and --experimental-pic but I think we only need --pie.
            OperatingSystem::Unknown if self.platform == Platform::Web => {
                out_args.extend([
                    "--import-memory".to_string(),
                    "--import-table".to_string(),
                    "--growable-table".to_string(),
                    "--export-all".to_string(),
                    "--allow-undefined".to_string(),
                    "--no-demangle".to_string(),
                    "--no-entry".to_string(),
                    "--pie".to_string(),
                    "--experimental-pic".to_string(),
                ]);
            }

            // This uses "cc" and these args need to be ld compatible
            //
            // Most importantly, we want to pass `-dylib` to both CC and the linker to indicate that
            // we want to generate the shared library instead of an executable.
            OperatingSystem::IOS(_) | OperatingSystem::MacOSX(_) | OperatingSystem::Darwin(_) => {
                out_args.extend(["-Wl,-dylib".to_string()]);

                match triple.architecture {
                    target_lexicon::Architecture::Aarch64(_) => {
                        out_args.push("-arch".to_string());
                        out_args.push("arm64".to_string());
                    }
                    target_lexicon::Architecture::X86_64 => {
                        out_args.push("-arch".to_string());
                        out_args.push("x86_64".to_string());
                    }
                    _ => {}
                }
            }

            // android/linux need to be compatible with lld
            //
            // android currently drags along its own libraries and other zany flags
            OperatingSystem::Linux if triple.environment == Environment::Android => {
                out_args.extend(
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
                out_args.extend([
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
            out_args.push("-target".to_string());
            out_args.push(vale);
        }

        if let Some(vale) = extract_value("-isysroot") {
            out_args.push("-isysroot".to_string());
            out_args.push(vale);
        }

        Ok(out_args)
    }

    /// Assemble the `cargo rustc` / `rustc` command
    ///
    /// When building fat/base binaries, we use `cargo rustc`.
    /// When building thin binaries, we use `rustc` directly.
    ///
    /// When processing the output of this command, you need to make sure to handle both cases which
    /// both have different formats (but with json output for both).
    fn build_command(&self, ctx: &BuildContext) -> Result<Command> {
        match &ctx.mode {
            // We're assembling rustc directly, so we need to be *very* careful. Cargo sets rustc's
            // env up very particularly, and we want to match it 1:1 but with some changes.
            //
            // To do this, we reset the env completely, and then pass every env var that the original
            // rustc process had 1:1.
            //
            // We need to unset a few things, like the RUSTC wrappers and then our special env var
            // indicating that dx itself is the compiler. If we forget to do this, then the compiler
            // ends up doing some recursive nonsense and dx is trying to link instead of compiling.
            //
            // todo: maybe rustc needs to be found on the FS instead of using the one in the path?
            BuildMode::Thin { rustc_args, .. } => {
                let mut cmd = Command::new("rustc");
                cmd.current_dir(self.workspace_dir());
                cmd.env_clear();
                cmd.args(rustc_args.args[1..].iter());
                cmd.envs(rustc_args.envs.iter());
                cmd.env_remove("RUSTC_WORKSPACE_WRAPPER");
                cmd.env_remove("RUSTC_WRAPPER");
                cmd.env_remove(DX_RUSTC_WRAPPER_ENV_VAR);
                cmd.envs(self.cargo_build_env_vars(ctx)?);
                cmd.arg(format!(
                    "-Clinker={}",
                    dunce::canonicalize(std::env::current_exe().unwrap())
                        .unwrap()
                        .display()
                ));
                Ok(cmd)
            }

            // For Base and Fat builds, we use a regular cargo setup, but we might need to intercept
            // rustc itself in case we're hot-patching and need a reliable rustc environment to
            // continuously recompile the top-level crate with.
            //
            // In the future, when we support hot-patching *all* workspace crates, we will need to
            // make use of the RUSTC_WORKSPACE_WRAPPER environment variable instead of RUSTC_WRAPPER
            // and then keep track of env and args on a per-crate basis.
            //
            // We've also had a number of issues with incorrect canonicalization when passing paths
            // through envs on windows, hence the frequent use of dunce::canonicalize.
            _ => {
                let mut cmd = Command::new("cargo");

                cmd.arg("rustc")
                    .current_dir(self.crate_dir())
                    .arg("--message-format")
                    .arg("json-diagnostic-rendered-ansi")
                    .args(self.cargo_build_arguments(ctx))
                    .envs(self.cargo_build_env_vars(ctx)?);

                if ctx.mode == BuildMode::Fat {
                    cmd.env(
                        DX_RUSTC_WRAPPER_ENV_VAR,
                        dunce::canonicalize(self.rustc_wrapper_args_file.path())
                            .unwrap()
                            .display()
                            .to_string(),
                    );
                    cmd.env(
                        "RUSTC_WRAPPER",
                        dunce::canonicalize(std::env::current_exe().unwrap())
                            .unwrap()
                            .display()
                            .to_string(),
                    );
                }

                Ok(cmd)
            }
        }
    }

    /// Create a list of arguments for cargo builds
    ///
    /// We always use `cargo rustc` *or* `rustc` directly. This means we can pass extra flags like
    /// `-C` arguments directly to the compiler.
    fn cargo_build_arguments(&self, ctx: &BuildContext) -> Vec<String> {
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

        // Set the executable
        match self.executable_type() {
            TargetKind::Bin => cargo_args.push("--bin".to_string()),
            TargetKind::Lib => cargo_args.push("--lib".to_string()),
            TargetKind::Example => cargo_args.push("--example".to_string()),
            _ => {}
        };
        cargo_args.push(self.executable_name().to_string());

        // Merge in extra args. Order shouldn't really matter.
        cargo_args.extend(self.extra_cargo_args.clone());
        cargo_args.push("--".to_string());
        cargo_args.extend(self.extra_rustc_args.clone());

        // The bundle splitter needs relocation data to create a call-graph.
        // This will automatically be erased by wasm-opt during the optimization step.
        if self.platform == Platform::Web && self.wasm_split {
            cargo_args.push("-Clink-args=--emit-relocs".to_string());
        }

        // dx *always* links android and thin builds
        if self.custom_linker.is_some() || matches!(ctx.mode, BuildMode::Thin { .. }) {
            cargo_args.push(format!(
                "-Clinker={}",
                dunce::canonicalize(std::env::current_exe().unwrap())
                    .unwrap()
                    .display()
            ));
        }

        // Our fancy hot-patching engine needs a lot of customization to work properly.
        //
        // These args are mostly intended to be passed when *fat* linking but are generally fine to
        // pass for both fat and thin linking.
        //
        // We need save-temps and no-dead-strip in both cases though. When we run `cargo rustc` with
        // these args, they will be captured and re-ran for the fast compiles in the future, so whatever
        // we set here will be set for all future hot patches too.
        if matches!(ctx.mode, BuildMode::Thin { .. } | BuildMode::Fat) {
            // rustc gives us some portable flags required:
            // - link-dead-code: prevents rust from passing -dead_strip to the linker since that's the default.
            // - save-temps=true: keeps the incremental object files around, which we need for manually linking.
            cargo_args.extend_from_slice(&[
                "-Csave-temps=true".to_string(),
                "-Clink-dead-code".to_string(),
            ]);

            // We need to set some extra args that ensure all symbols make it into the final output
            // and that the linker doesn't strip them out.
            //
            // This basically amounts of -all_load or --whole-archive, depending on the linker.
            // We just assume an ld-like interface on macos and a gnu-ld interface elsewhere.
            match self.triple.operating_system {
                // macOS/iOS use ld64 but through the `cc` interface.
                OperatingSystem::Darwin(_) | OperatingSystem::MacOSX(_) => {
                    cargo_args.push("-Clink-args=-Wl,-all_load".to_string());
                }

                // Linux and Android fit under this umbrella, both with the same clang-like entrypoint
                // and the gnu-ld interface.
                OperatingSystem::Linux => {
                    cargo_args.push("-Clink-args=-Wl,--whole-archive".to_string());
                }

                // If windows -Wl,--whole-archive is required since it follows gnu-ld convention.
                // There might be other flags on windows - we haven't tested windows thoroughly.
                //
                // https://learn.microsoft.com/en-us/cpp/build/reference/wholearchive-include-all-library-object-files?view=msvc-170
                OperatingSystem::Windows => {
                    cargo_args.push("-Clink-args=-Wl,--whole-archive".to_string());
                }

                // if web, -Wl,--whole-archive is required since it follows gnu-ld convention.
                //
                // We also use --no-gc-sections and --export-table and --export-memory  to push
                // said symbols into the export table.
                //
                // We use --emit-relocs to build up a solid call graph.
                //
                // rust uses its own wasm-ld linker which can be found here (it's just gcc-ld with a `-target wasm` flag):
                // - ~/.rustup/toolchains/stable-aarch64-apple-darwin/lib/rustlib/aarch64-apple-darwin/bin/gcc-ld
                // - ~/.rustup/toolchains/stable-aarch64-apple-darwin/lib/rustlib/aarch64-apple-darwin/bin/gcc-ld/wasm-ld
                //
                // Note that we can't use --export-all, unfortunately, since some symbols are internal
                // to wasm-bindgen and exporting them causes the JS generation to fail.
                //
                // We are basically replicating what emscripten does here with its dynamic linking
                // approach where the MAIN_MODULE is very "fat" and exports the necessary arguments
                // for the side modules to be linked in. This guide is really helpful:
                //
                // https://github.com/WebAssembly/tool-conventions/blob/main/DynamicLinking.md
                //
                // The trickiest one here is -Crelocation-model=pic, which forces data symbols
                // into a GOT, making it possible to import them from the main module.
                //
                // I think we can make relocation-model=pic work for non-wasm platforms, enabling
                // fully relocatable modules with no host coordination in lieu of sending out
                // the aslr slide at runtime.
                OperatingSystem::Wasi | OperatingSystem::Unknown
                    if self.platform == Platform::Web =>
                {
                    cargo_args.push("-Clink-arg=--no-gc-sections".into());
                    cargo_args.push("-Clink-arg=--growable-table".into());
                    cargo_args.push("-Clink-arg=--whole-archive".into());
                    cargo_args.push("-Clink-arg=--export-table".into());
                    cargo_args.push("-Clink-arg=--export-memory".into());
                    cargo_args.push("-Clink-arg=--emit-relocs".into());
                    cargo_args.push("-Clink-arg=--export=__stack_pointer".into());
                    cargo_args.push("-Clink-arg=--export=__heap_base".into());
                    cargo_args.push("-Clink-arg=--export=__data_end".into());
                    cargo_args.push("-Crelocation-model=pic".into());
                }

                _ => {
                    tracing::error!("Thin linking is not supported on this platform - hot patching might not work properly.");
                }
            }
        }

        cargo_args
    }

    fn cargo_build_env_vars(&self, ctx: &BuildContext) -> Result<Vec<(&'static str, String)>> {
        let mut env_vars = vec![];

        // Make sure to set all the crazy android flags. Cross-compiling is hard, man.
        if self.platform == Platform::Android {
            env_vars.extend(self.android_env_vars()?);
        };

        // If we're either zero-linking or using a custom linker, make `dx` itself do the linking.
        if self.custom_linker.is_some() || matches!(ctx.mode, BuildMode::Thin { .. }) {
            env_vars.push((
                LinkAction::ENV_VAR_NAME,
                LinkAction {
                    linker: self.custom_linker.clone(),
                    link_err_file: self.link_err_file.path().to_path_buf(),
                    link_args_file: self.link_args_file.path().to_path_buf(),
                    triple: self.triple.clone(),
                }
                .to_json(),
            ));
        }

        if let Some(target_dir) = self.custom_target_dir.as_ref() {
            env_vars.push(("CARGO_TARGET_DIR", target_dir.display().to_string()));
        }

        // If this is a release build, bake the base path and title into the binary with env vars.
        // todo: should we even be doing this? might be better being a build.rs or something else.
        if self.release {
            if let Some(base_path) = &self.config.web.app.base_path {
                env_vars.push((ASSET_ROOT_ENV, base_path.clone()));
            }
            env_vars.push((APP_TITLE_ENV, self.config.web.app.title.clone()));
        }

        Ok(env_vars)
    }

    fn android_env_vars(&self) -> Result<Vec<(&'static str, String)>> {
        let mut env_vars = vec![];

        let tools = android_tools().context("Could not determine android tools")?;
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

        // Set the wry env vars - this is where wry will dump its kotlin files.
        // Their setup is really annyoing and requires us to hardcode `dx` to specific versions of tao/wry.
        env_vars.push(("WRY_ANDROID_PACKAGE", "dev.dioxus.main".to_string()));
        env_vars.push(("WRY_ANDROID_LIBRARY", "dioxusmain".to_string()));
        env_vars.push((
            "WRY_ANDROID_KOTLIN_FILES_OUT_DIR",
            self.wry_android_kotlin_files_out_dir()
                .display()
                .to_string(),
        ));

        // Set the rust flags for android which get passed to *every* crate in the graph.
        // todo: I don't think we should be passing --export-dynamic here, but it works.
        //       At least for production, we shouldn't.
        env_vars.push(("RUSTFLAGS", {
            let mut rust_flags = std::env::var("RUSTFLAGS").unwrap_or_default();
            rust_flags.push_str(" -Clink-arg=-landroid");
            rust_flags.push_str(" -Clink-arg=-llog");
            rust_flags.push_str(" -Clink-arg=-lOpenSLES");
            rust_flags.push_str(" -Clink-arg=-Wl,--export-dynamic");
            rust_flags
        }));

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

        Ok(env_vars)
    }

    /// Get an estimate of the number of units in the crate. If nightly rustc is not available, this
    /// will return an estimate of the number of units in the crate based on cargo metadata.
    ///
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

    /// Try to get the unit graph for the crate. This is a nightly only feature which may not be
    /// available with the current version of rustc the user has installed.
    ///
    /// It also might not be super reliable - I think in practice it occasionally returns 2x the units.
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
            .args(self.cargo_build_arguments(ctx))
            .envs(self.cargo_build_env_vars(ctx)?)
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

    fn platform_dir(&self) -> PathBuf {
        self.build_dir(self.platform, self.release)
    }

    fn platform_exe_name(&self) -> String {
        match self.platform {
            Platform::MacOS => self.executable_name().to_string(),
            Platform::Ios => self.executable_name().to_string(),
            Platform::Server => self.executable_name().to_string(),
            Platform::Liveview => self.executable_name().to_string(),
            Platform::Windows => format!("{}.exe", self.executable_name()),

            // from the apk spec, the root exe is a shared library
            // we include the user's rust code as a shared library with a fixed namespace
            Platform::Android => "libdioxusmain.so".to_string(),

            // this will be wrong, I think, but not important?
            Platform::Web => format!("{}_bg.wasm", self.executable_name()),

            // todo: maybe this should be called AppRun?
            Platform::Linux => self.executable_name().to_string(),
        }
    }

    /// Assemble the android app dir.
    ///
    /// This is a bit of a mess since we need to create a lot of directories and files. Other approaches
    /// would be to unpack some zip folder or something stored via `include_dir!()`. However, we do
    /// need to customize the whole setup a bit, so it's just simpler (though messier) to do it this way.
    fn build_android_app_dir(&self) -> Result<()> {
        use std::fs::{create_dir_all, write};
        let root = self.root_dir();

        // gradle
        let wrapper = root.join("gradle").join("wrapper");
        create_dir_all(&wrapper)?;

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

        tracing::debug!(
            r#"Initialized android dirs:
- gradle:              {wrapper:?}
- app/                 {app:?}
- app/src:             {app_main:?}
- app/src/kotlin:      {app_kotlin:?}
- app/src/jniLibs:     {app_jnilibs:?}
- app/src/assets:      {app_assets:?}
- app/src/kotlin/main: {app_kotlin_out:?}
"#
        );

        // handlebars
        #[derive(Serialize)]
        struct AndroidHandlebarsObjects {
            application_id: String,
            app_name: String,
        }
        let hbs_data = AndroidHandlebarsObjects {
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

        // Write the res folder, containing stuff like default icons, colors, and menubars.
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

    fn wry_android_kotlin_files_out_dir(&self) -> PathBuf {
        let mut kotlin_dir = self
            .root_dir()
            .join("app")
            .join("src")
            .join("main")
            .join("kotlin");

        for segment in "dev.dioxus.main".split('.') {
            kotlin_dir = kotlin_dir.join(segment);
        }

        kotlin_dir
    }

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

    /// Get the package we are currently in
    pub(crate) fn package(&self) -> &krates::cm::Package {
        &self.workspace.krates[self.crate_package]
    }

    /// Get the name of the package we are compiling
    pub(crate) fn executable_name(&self) -> &str {
        &self.crate_target.name
    }

    /// Get the type of executable we are compiling
    pub(crate) fn executable_type(&self) -> TargetKind {
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
                Platform::autodetect_from_cargo_feature(feature)
                    .filter(|platform| *platform != Platform::Server)
                    .map(|f| (f, feature.to_string()))
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
    fn wasm_bindgen_version(&self) -> Option<String> {
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

    // pub(crate) fn default_platforms(&self) -> Vec<Platform> {
    //     let Some(default) = self.package().features.get("default") else {
    //         return Vec::new();
    //     };
    //     let mut platforms = vec![];

    //     // we only trace features 1 level deep..
    //     for feature in default.iter() {
    //         // If the user directly specified a platform we can just use that.
    //         if feature.starts_with("dioxus/") {
    //             let dx_feature = feature.trim_start_matches("dioxus/");
    //             let auto = Platform::autodetect_from_cargo_feature(dx_feature);
    //             if let Some(auto) = auto {
    //                 platforms.push(auto);
    //             }
    //         }

    //         // If the user is specifying an internal feature that points to a platform, we can use that
    //         let internal_feature = self.package().features.get(feature);
    //         if let Some(internal_feature) = internal_feature {
    //             for feature in internal_feature {
    //                 if feature.starts_with("dioxus/") {
    //                     let dx_feature = feature.trim_start_matches("dioxus/");
    //                     let auto = Platform::autodetect_from_cargo_feature(dx_feature);
    //                     if let Some(auto) = auto {
    //                         platforms.push(auto);
    //                     }
    //                 }
    //             }
    //         }
    //     }

    //     platforms.sort();
    //     platforms.dedup();
    //     platforms
    // }

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
    pub(crate) fn main_exe(&self) -> PathBuf {
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
    pub(crate) fn fullstack_feature_enabled(&self) -> bool {
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
    async fn optimize(&self, ctx: &BuildContext) -> Result<()> {
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

    /// Check if assets should be pre_compressed. This will only be true in release mode if the user
    /// has enabled pre_compress in the web config.
    fn should_pre_compress_web_assets(&self, release: bool) -> bool {
        self.config.web.pre_compress && release
    }

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
        let prebindgen = exe;
        let post_bindgen_wasm = self.wasm_bindgen_wasm_output_file();
        let should_bundle_split: bool = self.wasm_split;
        let rustc_exe = exe.with_extension("wasm");
        let bindgen_version = self
            .wasm_bindgen_version()
            .expect("this should have been checked by tool verification");

        // Prepare any work dirs
        std::fs::create_dir_all(&bindgen_outdir)?;

        // Lift the internal functions to exports
        if ctx.mode == BuildMode::Fat {
            let unprocessed = std::fs::read(&prebindgen)?;
            let all_exported_bytes = crate::build::prepare_wasm_base_module(&unprocessed).unwrap();
            std::fs::write(&rustc_exe, all_exported_bytes)?;
        }

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
            || will_wasm_opt
            || ctx.mode == BuildMode::Fat;
        let keep_names = will_wasm_opt || ctx.mode == BuildMode::Fat;
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
            .remove_name_section(!keep_names)
            .remove_producers_section(!keep_names)
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
        #[derive(Serialize)]
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

    pub(crate) fn debug_apk_path(&self) -> PathBuf {
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
    pub(crate) fn prepare_build_dir(&self) -> Result<()> {
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

    pub(crate) fn asset_dir(&self) -> PathBuf {
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
    fn exe_dir(&self) -> PathBuf {
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
    fn wasm_bindgen_out_dir(&self) -> PathBuf {
        self.root_dir().join("wasm")
    }

    /// Get the path to the wasm bindgen javascript output file
    pub(crate) fn wasm_bindgen_js_output_file(&self) -> PathBuf {
        self.wasm_bindgen_out_dir()
            .join(self.executable_name())
            .with_extension("js")
    }

    /// Get the path to the wasm bindgen wasm output file
    pub(crate) fn wasm_bindgen_wasm_output_file(&self) -> PathBuf {
        self.wasm_bindgen_out_dir()
            .join(format!("{}_bg", self.executable_name()))
            .with_extension("wasm")
    }

    /// Get the path to the asset optimizer version file
    pub(crate) fn asset_optimizer_version_file(&self) -> PathBuf {
        self.platform_dir().join(".cli-version")
    }

    fn flush_session_cache(&self) {
        let cache_dir = self.session_cache_dir();
        _ = std::fs::remove_dir_all(&cache_dir);
        _ = std::fs::create_dir_all(&cache_dir);
    }

    pub(crate) fn incremental_cache_dir(&self) -> PathBuf {
        self.platform_dir().join("incremental-cache")
    }

    /// Check for tooling that might be required for this build.
    ///
    /// This should generally be only called on the first build since it takes time to verify the tooling
    /// is in place, and we don't want to slow down subsequent builds.
    pub(crate) async fn verify_tooling(&self, ctx: &BuildContext) -> Result<()> {
        tracing::debug!("Verifying tooling...");
        ctx.status_installing_tooling();

        self
            .initialize_profiles()
            .context("Failed to initialize profiles - dioxus can't build without them. You might need to initialize them yourself.")?;

        match self.platform {
            Platform::Web => self.verify_web_tooling().await?,
            Platform::Ios => self.verify_ios_tooling().await?,
            Platform::Android => self.verify_android_tooling().await?,
            Platform::Linux => self.verify_linux_tooling().await?,
            Platform::MacOS | Platform::Windows | Platform::Server | Platform::Liveview => {}
        }

        Ok(())
    }

    async fn verify_web_tooling(&self) -> Result<()> {
        // Install target using rustup.
        #[cfg(not(feature = "no-downloads"))]
        if !self.workspace.has_wasm32_unknown_unknown() {
            tracing::info!(
                "Web platform requires wasm32-unknown-unknown to be installed. Installing..."
            );

            let _ = tokio::process::Command::new("rustup")
                .args(["target", "add", "wasm32-unknown-unknown"])
                .output()
                .await?;
        }

        // Ensure target is installed.
        if !self.workspace.has_wasm32_unknown_unknown() {
            return Err(Error::Other(anyhow::anyhow!(
                "Missing target wasm32-unknown-unknown."
            )));
        }

        // Wasm bindgen
        let krate_bindgen_version = self.wasm_bindgen_version().ok_or(anyhow::anyhow!(
            "failed to detect wasm-bindgen version, unable to proceed"
        ))?;

        WasmBindgen::verify_install(&krate_bindgen_version).await?;

        Ok(())
    }

    /// Currently does nothing, but eventually we need to check that the mobile tooling is installed.
    ///
    /// For ios, this would be just aarch64-apple-ios + aarch64-apple-ios-sim, as well as xcrun and xcode-select
    ///
    /// We don't auto-install these yet since we're not doing an architecture check. We assume most users
    /// are running on an Apple Silicon Mac, but it would be confusing if we installed these when we actually
    /// should be installing the x86 versions.
    async fn verify_ios_tooling(&self) -> Result<()> {
        // open the simulator
        // _ = tokio::process::Command::new("open")
        //     .arg("/Applications/Xcode.app/Contents/Developer/Applications/Simulator.app")
        //     .output()
        //     .await;

        // Now xcrun to open the device
        // todo: we should try and query the device list and/or parse it rather than hardcode this simulator
        // _ = tokio::process::Command::new("xcrun")
        //     .args(["simctl", "boot", "83AE3067-987F-4F85-AE3D-7079EF48C967"])
        //     .output()
        //     .await;

        // if !rustup
        //     .installed_toolchains
        //     .contains(&"aarch64-apple-ios".to_string())
        // {
        //     tracing::error!("You need to install aarch64-apple-ios to build for ios. Run `rustup target add aarch64-apple-ios` to install it.");
        // }

        // if !rustup
        //     .installed_toolchains
        //     .contains(&"aarch64-apple-ios-sim".to_string())
        // {
        //     tracing::error!("You need to install aarch64-apple-ios to build for ios. Run `rustup target add aarch64-apple-ios` to install it.");
        // }

        Ok(())
    }

    /// Check if the android tooling is installed
    ///
    /// looks for the android sdk + ndk
    ///
    /// will do its best to fill in the missing bits by exploring the sdk structure
    /// IE will attempt to use the Java installed from android studio if possible.
    async fn verify_android_tooling(&self) -> Result<()> {
        let android = android_tools().context("Android not installed properly. Please set the `ANDROID_NDK_HOME` environment variable to the root of your NDK installation.")?;

        let linker = android.android_cc(&self.triple);

        tracing::debug!("Verifying android linker: {linker:?}");

        if linker.exists() {
            return Ok(());
        }

        Err(anyhow::anyhow!(
            "Android linker not found. Please set the `ANDROID_NDK_HOME` environment variable to the root of your NDK installation."
        ).into())
    }

    /// Ensure the right dependencies are installed for linux apps.
    /// This varies by distro, so we just do nothing for now.
    ///
    /// Eventually, we want to check for the prereqs for wry/tao as outlined by tauri:
    ///     https://tauri.app/start/prerequisites/
    async fn verify_linux_tooling(&self) -> Result<()> {
        Ok(())
    }

    /// update the mtime of the "main" file to bust the fingerprint, forcing rustc to recompile it.
    ///
    /// This prevents rustc from using the cached version of the binary, which can cause issues
    /// with our hotpatching setup since it uses linker interception.
    ///
    /// This is sadly a hack. I think there might be other ways of busting the fingerprint (rustc wrapper?)
    /// but that would require relying on cargo internals.
    ///
    /// This might stop working if/when cargo stabilizies contents-based fingerprinting.
    fn bust_fingerprint(&self, ctx: &BuildContext) -> Result<()> {
        if !matches!(ctx.mode, BuildMode::Thin { .. }) {
            std::fs::File::open(&self.crate_target.src_path)?.set_modified(SystemTime::now())?;
        }
        Ok(())
    }
}
