//! # [`BuildRequest`] - the core of the build process
//!
//! The [`BuildRequest`] object is the core of the build process. It contains all the resolved arguments
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
//! - BuildMode::Base: A normal build generated using `cargo rustc`
//! - BuildMode::Fat: A "fat" build where all dependency rlibs are merged into a static library
//! - BuildMode::Thin: A "thin" build that dynamically links against the artifacts produced by the "fat" build
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
//! is a bit of a pain, but fortunately a solved process (<https://github.com/rust-mobile/xbuild>).
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
//! <https://docs.appimage.org/reference/appdir.html#ref-appdir>
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
//! <https://github.com/WanghongLin/miscellaneous/blob/master/tools/build-apk-manually.sh>
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
//!       x86/
//!           libmyapp.so
//!       x86_64/
//!           libmyapp.so
//! ```
//! Notice that we *could* feasibly build this ourselves :)
//!
//! ### Windows:
//! <https://superuser.com/questions/749447/creating-a-single-file-executable-from-a-directory-in-windows>
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
//! - apple: <https://developer.apple.com/documentation/bundleresources/placing_content_in_a_bundle>
//! - appimage: <https://docs.appimage.org/packaging-guide/manual.html#ref-manual>
//!
//! ## Extra links
//! - xbuild: <https://github.com/rust-mobile/xbuild/blob/master/xbuild/src/command/build.rs>

use crate::{
    build::cache::ObjectCache,
    AndroidTools, AppManifest, BuildContext, BuildId, BundleFormat, DioxusConfig, Error,
    LinkAction, LinkerFlavor, Platform, Renderer, Result, RustcArgs, TargetArgs, TraceSrc,
    WasmBindgen, WasmOptConfig, Workspace, DX_RUSTC_WRAPPER_ENV_VAR,
};
use anyhow::{bail, Context};
use cargo_metadata::diagnostic::Diagnostic;
use cargo_toml::{Profile, Profiles, StripSetting};
use depinfo::RustcDepInfo;
use dioxus_cli_config::{format_base_path_meta_element, PRODUCT_NAME_ENV};
use dioxus_cli_config::{APP_TITLE_ENV, ASSET_ROOT_ENV};
use dioxus_cli_opt::{process_file_to, AssetManifest};
use itertools::Itertools;
use krates::{cm::TargetKind, NodeId};
use manganis::{AssetOptions, BundledAsset};
use manganis_core::{AssetOptionsBuilder, AssetVariant};
use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};
use serde::{Deserialize, Serialize};
use std::{borrow::Cow, ffi::OsString};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    io::Write,
    path::{Path, PathBuf},
    process::Stdio,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::{SystemTime, UNIX_EPOCH},
};
use subsecond_types::JumpTable;
use target_lexicon::{Architecture, OperatingSystem, Triple};
use tempfile::TempDir;
use tokio::{io::AsyncBufReadExt, process::Command};
use uuid::Uuid;

use super::HotpatchModuleCache;

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
/// All updates from the build will be sent on a global "BuildProgress" channel.
#[derive(Clone)]
pub(crate) struct BuildRequest {
    pub(crate) workspace: Arc<Workspace>,
    pub(crate) config: DioxusConfig,
    pub(crate) crate_package: NodeId,
    pub(crate) crate_target: krates::cm::Target,
    pub(crate) profile: String,
    pub(crate) release: bool,
    pub(crate) bundle: BundleFormat,
    pub(crate) triple: Triple,
    pub(crate) device_name: Option<String>,
    pub(crate) should_codesign: bool,
    pub(crate) package: String,
    pub(crate) main_target: String,
    pub(crate) features: Vec<String>,
    pub(crate) rustflags: cargo_config2::Flags,
    pub(crate) extra_cargo_args: Vec<String>,
    pub(crate) extra_rustc_args: Vec<String>,
    pub(crate) no_default_features: bool,
    pub(crate) all_features: bool,
    pub(crate) target_dir: PathBuf,
    pub(crate) skip_assets: bool,
    pub(crate) wasm_split: bool,
    pub(crate) debug_symbols: bool,
    pub(crate) inject_loading_scripts: bool,
    pub(crate) custom_linker: Option<PathBuf>,
    pub(crate) base_path: Option<String>,
    pub(crate) using_dioxus_explicitly: bool,
    pub(crate) apple_entitlements: Option<PathBuf>,
    pub(crate) apple_team_id: Option<String>,
    pub(crate) session_cache_dir: PathBuf,
    pub(crate) raw_json_diagnostics: bool,
    pub(crate) windows_subsystem: Option<String>,
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
    ///
    /// "run" indicates whether this build is intended to be run immediately after building.
    /// This means we try to capture the build environment, saving vars like `CARGO_MANIFEST_DIR`
    /// for the running executable.
    Base { run: bool },

    /// A "Fat" build generated with cargo rustc and dx as a custom linker without -Wl,-dead-strip
    Fat,

    /// A "thin" build generated with `rustc` directly and dx as a custom linker
    Thin {
        workspace_rustc_args: HashMap<String, RustcArgs>,
        changed_files: Vec<PathBuf>,
        /// Which workspace crates had source file changes in this edit.
        changed_crates: Vec<String>,
        /// Cumulative set of all workspace crates modified since the fat build.
        modified_crates: HashSet<String>,
        aslr_reference: u64,
        cache: Arc<HotpatchModuleCache>,
        /// Cache of compiled objects from previous thin builds, for assembly diffing and relinking.
        object_cache: ObjectCache,
    },
}

/// The end result of a build.
///
/// Contains the final asset manifest, the executable, and metadata about the build.
/// Note that the `exe` might be stale and/or overwritten by the time you read it!
///
/// The patch cache is only populated on fat builds and then used for thin builds (see `BuildMode::Thin`).
#[derive(Clone, Debug)]
pub struct BuildArtifacts {
    pub(crate) root_dir: PathBuf,
    pub(crate) exe: PathBuf,
    pub(crate) workspace_rustc_args: HashMap<String, RustcArgs>,
    pub(crate) time_start: SystemTime,
    pub(crate) time_end: SystemTime,
    pub(crate) assets: AssetManifest,
    pub(crate) mode: BuildMode,
    pub(crate) patch_cache: Option<Arc<HotpatchModuleCache>>,
    pub(crate) depinfo: RustcDepInfo,
    pub(crate) build_id: BuildId,
    /// Updated object cache after thin build (returned to AppBuilder for persistence).
    pub(crate) object_cache: ObjectCache,
    /// Updated cumulative modified crates set after thin build.
    pub(crate) modified_crates: HashSet<String>,
}

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
    pub(crate) async fn new(args: &TargetArgs, workspace: Arc<Workspace>) -> Result<Self> {
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

        // Use the main_target for the client + server build if it is set, otherwise use the target name for this
        // specific build. This is important for @client @server syntax so we use the client's output directory for the bundle.
        let main_target = args.client_target.clone().unwrap_or(target_name.clone());

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
                    format!("Failed to find example {example}. \nAvailable examples are:\n{examples}")
                } else if let Some(bin) = &args.bin {
                    let binaries = target_of_kind(&TargetKind::Bin);
                    format!("Failed to find binary {bin}. \nAvailable binaries are:\n{binaries}")
                } else {
                    format!("Failed to find target {target_name}. \nIt looks like you are trying to build dioxus in a library crate. \
                    You either need to run dx from inside a binary crate or build a specific example with the `--example` flag. \
                    Available examples are:\n{}", target_of_kind(&TargetKind::Example))
                }
            })?
            .clone();

        // We usually use the simulator unless --device is passed *or* a device is detected by probing.
        // For now, though, since we don't have probing, it just defaults to false
        // Tools like xcrun/adb can detect devices
        let device = args.device.clone();

        let using_dioxus_explicitly = main_package
            .dependencies
            .iter()
            .any(|dep| dep.name == "dioxus");

        /*
        Determine which features, triple, profile, etc to pass to the build.

        Most of the time, users should use `dx serve --<platform>` where the platform name directly
        corresponds to the feature in their cargo.toml. So,
        - `dx serve --web` will enable the `web` feature
        - `dx serve --mobile` will enable the `mobile` feature
        - `dx serve --desktop` will enable the `desktop` feature

        In this case, we set default-features to false and then add back the default features that
        aren't renderers, and then add the feature for the given renderer (ie web/desktop/mobile).
        We call this "no-default-features-stripped."

        There are a few cases where the user doesn't need to pass a platform.
        - they selected one via `dioxus = { features = ["web"] }`
        - they have a single platform in their default features `default = ["web"]`
        - there is only a single non-server renderer as a feature `web = ["dioxus/web"], server = ["dioxus/server"]`
        - they compose the super triple via triple + bundleformat + features

        Note that we only use the names of the features to correspond with the platform.
        Platforms are "super triples", meaning they contain information about
        - bundle format
        - target triple
        - how to serve
        - enabled features

        By default, the --platform presets correspond to:
        - web:          bundle(web), triple(wasm32), serve(http-serve), features("web")
        - desktop:      alias to mac/win/linux
        - mac:          bundle(mac), triple(host), serve(appbundle-open), features("desktop")
        - windows:      bundle(exefolder), triple(host), serve(run-exe), features("desktop")
        - linux:        bundle(appimage), triple(host), serve(run-exe), features("desktop")
        - ios:          bundle(ios), triple(arm64-apple-ios), serve(ios-simulator/xcrun), features("mobile")
        - android:      bundle(android), triple(arm64-apple-ios), serve(android-emulator/adb), features("mobile")
        - server:       bundle(server), triple(host), serve(run-exe), features("server") (and disables the client)
        - liveview:     bundle(liveview), triple(host), serve(run-exe), features("liveview")
        - unknown:      <auto or default to desktop>

        Fullstack usage is inferred from the presence of the fullstack feature or --fullstack.
        */
        let mut features = args.features.clone();
        let no_default_features = args.no_default_features;
        let all_features = args.all_features;
        let mut triple = args.target.clone();
        let mut renderer = args.renderer;
        let mut bundle_format = args.bundle;
        let mut platform = args.platform;

        // the crate might be selecting renderers but the user also passes a renderer. this is weird
        // ie dioxus = { features = ["web"] } but also --platform desktop
        // anyways, we collect it here in the event we need it if platform is not specified.
        let dioxus_direct_renderer = Self::renderer_enabled_by_dioxus_dependency(main_package);
        let known_features_as_renderers = Self::features_that_enable_renderers(main_package);

        // The crate might enable multiple platforms or no platforms at
        // We collect all the platforms it enables first and then select based on the --platform arg
        let enabled_renderers = if no_default_features {
            vec![]
        } else {
            Self::enabled_cargo_toml_default_features_renderers(main_package)
        };

        // Try the easy autodetects.
        // - if the user has `dioxus = { features = ["web"] }`
        // - if the `default =["web"]` or `default = ["dioxus/web"]`
        // - if there's only one non-server platform ie `web = ["dioxus/web"], server = ["dioxus/server"]`
        // Only do this if we're explicitly using dioxus
        if matches!(platform, Platform::Unknown) && using_dioxus_explicitly {
            let auto = dioxus_direct_renderer
                .or_else(|| {
                    if enabled_renderers.len() == 1 {
                        Some(enabled_renderers[0].clone())
                    } else {
                        None
                    }
                })
                .or_else(|| {
                    // If multiple renderers are enabled, pick the first non-server one
                    if enabled_renderers.len() == 2
                        && enabled_renderers
                            .iter()
                            .any(|f| matches!(f.0, Renderer::Server))
                    {
                        return Some(
                            enabled_renderers
                                .iter()
                                .find(|f| !matches!(f.0, Renderer::Server))
                                .cloned()
                                .unwrap(),
                        );
                    }
                    None
                })
                .or_else(|| {
                    // Pick the first non-server feature in the cargo.toml
                    let non_server_features = known_features_as_renderers
                        .iter()
                        .filter(|f| f.1.as_str() != "server")
                        .collect::<Vec<_>>();
                    if non_server_features.len() == 1 {
                        Some(non_server_features[0].clone())
                    } else {
                        None
                    }
                });

            if let Some((direct, feature)) = auto {
                match direct {
                    _ if feature == "mobile" || feature == "dioxus/mobile" => {
                        bail!(
                            "Could not autodetect mobile platform. Use --ios or --android instead."
                        );
                    }
                    Renderer::Webview | Renderer::Native => {
                        if cfg!(target_os = "macos") {
                            platform = Platform::MacOS;
                        } else if cfg!(target_os = "linux") {
                            platform = Platform::Linux;
                        } else if cfg!(target_os = "windows") {
                            platform = Platform::Windows;
                        }
                    }
                    Renderer::Server => platform = Platform::Server,
                    Renderer::Liveview => platform = Platform::Liveview,
                    Renderer::Web => platform = Platform::Web,
                }
                renderer = renderer.or(Some(direct));
            }
        }

        // Set the super triple from the platform if it's provided.
        // Otherwise, we attempt to guess it from the rest of their inputs.
        match platform {
            Platform::Unknown => {}

            Platform::Web => {
                if main_package.features.contains_key("web") && renderer.is_none() {
                    features.push("web".into());
                }
                renderer = renderer.or(Some(Renderer::Web));
                bundle_format = bundle_format.or(Some(BundleFormat::Web));
                triple = triple.or(Some("wasm32-unknown-unknown".parse()?));
            }
            Platform::MacOS => {
                if main_package.features.contains_key("desktop") && renderer.is_none() {
                    features.push("desktop".into());
                }
                renderer = renderer.or(Some(Renderer::Webview));
                bundle_format = bundle_format.or(Some(BundleFormat::MacOS));
                triple = triple.or(Some(Triple::host()));
            }
            Platform::Windows => {
                if main_package.features.contains_key("desktop") && renderer.is_none() {
                    features.push("desktop".into());
                }
                renderer = renderer.or(Some(Renderer::Webview));
                bundle_format = bundle_format.or(Some(BundleFormat::Windows));
                triple = triple.or(Some(Triple::host()));
            }
            Platform::Linux => {
                if main_package.features.contains_key("desktop") && renderer.is_none() {
                    features.push("desktop".into());
                }
                renderer = renderer.or(Some(Renderer::Webview));
                bundle_format = bundle_format.or(Some(BundleFormat::Linux));
                triple = triple.or(Some(Triple::host()));
            }
            Platform::Ios => {
                if main_package.features.contains_key("mobile") && renderer.is_none() {
                    features.push("mobile".into());
                }
                renderer = renderer.or(Some(Renderer::Webview));
                bundle_format = bundle_format.or(Some(BundleFormat::Ios));
                match device.is_some() {
                    // If targeting device, we want to build for the device which is always aarch64
                    true => triple = triple.or(Some("aarch64-apple-ios".parse()?)),

                    // If the host is aarch64, we assume the user wants to build for iOS simulator
                    false if matches!(Triple::host().architecture, Architecture::Aarch64(_)) => {
                        triple = triple.or(Some("aarch64-apple-ios-sim".parse()?))
                    }

                    // Otherwise, it's the x86_64 simulator, which is just x86_64-apple-ios
                    _ => triple = triple.or(Some("x86_64-apple-ios".parse()?)),
                }
            }
            Platform::Android => {
                if main_package.features.contains_key("mobile") && renderer.is_none() {
                    features.push("mobile".into());
                }

                renderer = renderer.or(Some(Renderer::Webview));
                bundle_format = bundle_format.or(Some(BundleFormat::Android));

                // maybe probe adb?
                if let Some(_device_name) = device.as_ref() {
                    if triple.is_none() {
                        triple = Some(
                            crate::get_android_tools()
                                .context("Failed to get android tools")?
                                .autodetect_android_device_triple()
                                .await,
                        );
                    }
                } else {
                    triple = triple.or(Some({
                        match Triple::host().architecture {
                            Architecture::X86_32(_) => "i686-linux-android".parse()?,
                            Architecture::X86_64 => "x86_64-linux-android".parse()?,
                            Architecture::Aarch64(_) => "aarch64-linux-android".parse()?,
                            _ => "aarch64-linux-android".parse()?,
                        }
                    }));
                }
            }
            Platform::Server => {
                if main_package.features.contains_key("server") && renderer.is_none() {
                    features.push("server".into());
                }
                renderer = renderer.or(Some(Renderer::Server));
                bundle_format = bundle_format.or(Some(BundleFormat::Server));
                triple = triple.or(Some(Triple::host()));
            }
            Platform::Liveview => {
                if main_package.features.contains_key("liveview") && renderer.is_none() {
                    features.push("liveview".into());
                }
                renderer = renderer.or(Some(Renderer::Liveview));
                bundle_format = bundle_format.or(Some(BundleFormat::Server));
                triple = triple.or(Some(Triple::host()));
            }
        }

        // If default features are enabled, we need to add the default features
        // which don't enable a renderer
        if !no_default_features {
            features.extend(Self::rendererless_features(main_package));
            features.dedup();
            features.sort();
        }

        // The triple will be the triple passed or the host if using dioxus.
        let triple = if using_dioxus_explicitly {
            triple.context("Could not automatically detect target triple")?
        } else {
            triple.unwrap_or(Triple::host())
        };

        // The bundle format will be the bundle format passed or the host.
        let bundle = if using_dioxus_explicitly {
            bundle_format.context("Could not automatically detect bundle format")?
        } else {
            bundle_format.unwrap_or(BundleFormat::host())
        };

        // Add any features required to turn on the client
        if let Some(renderer) = renderer {
            if let Some(feature) =
                Self::feature_for_platform_and_renderer(main_package, &triple, renderer)
            {
                features.push(feature);
                features.dedup();
            }
        }

        // Set the profile of the build if it's not already set
        // This is mostly used for isolation of builds (preventing thrashing) but also useful to have multiple performance profiles
        // We might want to move some of these profiles into dioxus.toml and make them "virtual".
        let profile = match args.profile.clone() {
            Some(profile) => profile,
            None => bundle.profile_name(args.release),
        };

        // Determine if we should codesign
        let should_codesign =
            args.codesign || device.is_some() || args.apple_entitlements.is_some();

        // Determining release mode is based on the profile, actually, so we need to check that
        let release = workspace.is_release_profile(&profile);

        // Determine the --package we'll pass to cargo.
        // todo: I think this might be wrong - we don't want to use main_package necessarily...
        let package = args
            .package
            .clone()
            .unwrap_or_else(|| main_package.name.clone());

        // Somethings we override are also present in the user's config.
        // If we can't get them by introspecting cargo, then we need to get them from the config
        //
        // This involves specifically two fields:
        // - The linker since we override it for Android and hotpatching
        // - RUSTFLAGS since we also override it for Android and hotpatching
        let cargo_config = cargo_config2::Config::load().unwrap();
        let mut custom_linker = cargo_config.linker(triple.to_string()).ok().flatten();
        let mut rustflags = cargo_config2::Flags::default();

        // Make sure to take into account the RUSTFLAGS env var and the CARGO_TARGET_<triple>_RUSTFLAGS
        for env in [
            "RUSTFLAGS".to_string(),
            format!("CARGO_TARGET_{triple}_RUSTFLAGS"),
        ] {
            if let Ok(flags) = std::env::var(env) {
                rustflags
                    .flags
                    .extend(cargo_config2::Flags::from_space_separated(&flags).flags);
            }
        }

        // Use the user's linker if the specify it at the target level
        if let Ok(target) = cargo_config.target(triple.to_string()) {
            if let Some(flags) = target.rustflags {
                rustflags.flags.extend(flags.flags);
            }
        }

        // When we do android builds we need to make sure we link against the android libraries
        // We also `--export-dynamic` to make sure we can do shenanigans like `dlsym` the `main` symbol
        if matches!(bundle, BundleFormat::Android) {
            rustflags.flags.extend([
                "-Clink-arg=-landroid".to_string(),
                "-Clink-arg=-llog".to_string(),
                "-Clink-arg=-lOpenSLES".to_string(),
                "-Clink-arg=-lc++abi".to_string(),
                "-Clink-arg=-Wl,--export-dynamic".to_string(),
                format!(
                    "-Clink-arg=-Wl,--sysroot={}",
                    workspace.android_tools()?.sysroot().display()
                ),
            ]);
        }

        // Make sure we set the sysroot for ios builds in the event the user doesn't have it set
        if matches!(bundle, BundleFormat::Ios) {
            let xcode_path = Workspace::get_xcode_path()
                .await
                .unwrap_or_else(|| "/Applications/Xcode.app".to_string().into());

            let sysroot_location = match triple.environment {
                target_lexicon::Environment::Sim => xcode_path
                    .join("Platforms/iPhoneSimulator.platform/Developer/SDKs/iPhoneSimulator.sdk"),
                _ => xcode_path.join("Platforms/iPhoneOS.platform/Developer/SDKs/iPhoneOS.sdk"),
            };

            if sysroot_location.exists() && !rustflags.flags.iter().any(|f| f == "-isysroot") {
                rustflags.flags.extend([
                    "-Clink-arg=-isysroot".to_string(),
                    format!("-Clink-arg={}", sysroot_location.display()),
                ]);
            }
        }

        // automatically set the getrandom backend for web builds if the user requested it
        if matches!(bundle, BundleFormat::Web) && args.wasm_js_cfg {
            rustflags.flags.extend(
                cargo_config2::Flags::from_space_separated(r#"--cfg getrandom_backend="wasm_js""#)
                    .flags,
            );
        }

        // If no custom linker is set, then android falls back to us as the linker
        if custom_linker.is_none() && bundle == BundleFormat::Android {
            let min_sdk_version = config.application.android_min_sdk_version.unwrap_or(28);
            custom_linker = Some(
                workspace
                    .android_tools()?
                    .android_cc(&triple, min_sdk_version),
            );
        }

        let target_dir = std::env::var("CARGO_TARGET_DIR")
            .ok()
            .map(PathBuf::from)
            .or_else(|| cargo_config.build.target_dir.clone())
            .unwrap_or_else(|| workspace.workspace_root().join("target"));

        // If the user provided a profile and wasm_split is enabled, we should check that LTO=true and debug=true
        if args.wasm_split {
            if let Some(profile_data) = workspace.cargo_toml.profile.custom.get(&profile) {
                use cargo_toml::{DebugSetting, LtoSetting};
                if matches!(profile_data.lto, Some(LtoSetting::None) | None) {
                    tracing::warn!("wasm-split requires LTO to be enabled in the profile. \
                        Please set `lto = true` in the `[profile.{profile}]` section of your Cargo.toml");
                }
                if matches!(profile_data.debug, Some(DebugSetting::None) | None) {
                    tracing::warn!("wasm-split requires debug symbols to be enabled in the profile. \
                        Please set `debug = true` in the `[profile.{profile}]` section of your Cargo.toml");
                }
            }
        }

        #[allow(deprecated)]
        let session_cache_dir = args
            .session_cache_dir
            .clone()
            .unwrap_or_else(|| TempDir::new().unwrap().into_path());

        let extra_rustc_args = shell_words::split(&args.rustc_args.clone().unwrap_or_default())
            .context("Failed to parse rustc args")?;

        let extra_cargo_args = shell_words::split(&args.cargo_args.clone().unwrap_or_default())
            .context("Failed to parse cargo args")?;

        tracing::debug!(
            r#"Target Info:
                • features: {features:?}
                • triple: {triple}
                • bundle format: {bundle:?}
                • session cache dir: {session_cache_dir:?}
                • linker: {custom_linker:?}
                • target_dir: {target_dir:?}"#,
        );

        Ok(Self {
            features,
            bundle,
            // We hardcode passing `--no-default-features` to Cargo because dx manually enables
            // the default features we want.
            no_default_features: true,
            all_features,
            crate_package,
            crate_target,
            profile,
            triple,
            device_name: device,
            workspace,
            config,
            target_dir,
            custom_linker,
            extra_rustc_args,
            extra_cargo_args,
            release,
            package,
            main_target,
            rustflags,
            using_dioxus_explicitly,
            should_codesign,
            session_cache_dir,
            skip_assets: args.skip_assets,
            base_path: args.base_path.clone(),
            wasm_split: args.wasm_split,
            debug_symbols: args.debug_symbols,
            inject_loading_scripts: args.inject_loading_scripts,
            apple_entitlements: args.apple_entitlements.clone(),
            apple_team_id: args.apple_team_id.clone(),
            raw_json_diagnostics: args.raw_json_diagnostics,
            windows_subsystem: args.windows_subsystem.clone(),
        })
    }

    pub(crate) async fn prebuild(&self, ctx: &BuildContext) -> Result<()> {
        // Create the session cache directory
        let cache_dir = self.session_cache_dir();
        _ = std::fs::create_dir_all(&cache_dir);
        _ = std::fs::create_dir_all(self.rustc_wrapper_args_dir());
        _ = std::fs::File::create_new(self.link_err_file());
        _ = std::fs::File::create_new(self.link_args_file());
        _ = std::fs::File::create_new(self.windows_command_file());

        if !matches!(ctx.mode, BuildMode::Thin { .. }) {
            self.prepare_build_dir(ctx)?;
        }

        if !ctx.is_primary_build() {
            return Ok(());
        }

        // Run the tailwind build before bundling anything else
        _ = crate::TailwindCli::run_once(
            self.package_manifest_dir(),
            self.config.application.tailwind_input.clone(),
            self.config.application.tailwind_output.clone(),
        )
        .await;

        // We want to copy over the prebuilt OpenSSL binaries to ~/.dx/prebuilt/openssl-<version>
        if self.bundle == BundleFormat::Android {
            AndroidTools::unpack_prebuilt_openssl()?;
        }

        Ok(())
    }

    pub(crate) async fn build(&self, ctx: &BuildContext) -> Result<BuildArtifacts> {
        let time_start = SystemTime::now();

        // If we forget to do this, then we won't get the linker args since rust skips the full build
        // We need to make sure to not react to this though, so the filemap must cache it
        _ = self.bust_fingerprint(ctx);

        // For thin builds, compile workspace dep crates BEFORE the tip crate.
        // This updates their rlibs on disk so cargo links the tip against fresh code.
        let mut thin_pre_state: Option<(ObjectCache, HashSet<String>)> = None;
        if let BuildMode::Thin {
            workspace_rustc_args,
            changed_crates,
            modified_crates,
            object_cache,
            ..
        } = &ctx.mode
        {
            let tip_name = self.tip_crate_name();
            let mut object_cache = object_cache.clone();
            let mut modified_crates = modified_crates.clone();

            // Compile workspace dep crates with cascade detection.
            //
            // Start with the explicitly changed dep crates (already in leaf-first order
            // from handle_file_change). As we compile each, assembly-diff to detect
            // cross-crate changes. If cascade is needed, add the crate's workspace
            // dependents to the compile list so they get recompiled too.
            let mut crates_to_compile: Vec<String> = changed_crates
                .iter()
                .filter(|c| *c != &tip_name)
                .cloned()
                .collect();
            let mut compiled = HashSet::new();
            let mut idx = 0;

            while idx < crates_to_compile.len() {
                let crate_name = crates_to_compile[idx].clone();
                idx += 1;

                // Skip already-compiled crates and the tip (compiled separately below)
                if !compiled.insert(crate_name.clone()) || crate_name == tip_name {
                    continue;
                }

                let Some(rustc_args) = workspace_rustc_args.get(&crate_name) else {
                    tracing::warn!(
                        "No captured rustc args for workspace crate {crate_name}, skipping"
                    );
                    continue;
                };

                // Get old objects for assembly diffing (if we have them cached)
                let old_objects = object_cache.get(&crate_name).cloned();

                // Compile the dep crate directly with rustc
                tracing::debug!("Compiling workspace dep crate: {crate_name}");
                if let Err(e) = self.compile_dep_crate(&crate_name, rustc_args).await {
                    tracing::warn!(
                        "Failed to compile workspace dep crate {crate_name}: {e}"
                    );
                    continue;
                }

                // Find and cache the new objects from the rlib
                if let Some(rlib_path) =
                    self.find_rlib_for_crate(&crate_name, rustc_args)
                {
                    if let Err(e) =
                        object_cache.cache_from_rlib(&crate_name, &rlib_path)
                    {
                        tracing::warn!(
                            "Failed to cache objects from rlib for {crate_name}: {e}"
                        );
                    }
                }

                modified_crates.insert(crate_name.clone());

                // Assembly diff: compare old vs new objects for informational logging.
                // Note: we ALWAYS cascade to dependents regardless of the diff result,
                // because rustc's SVH (stable version hash) changes on ANY recompilation,
                // even for implementation-only changes. Downstream crates compiled against
                // the old SVH become invalid and must be recompiled.
                //
                // In the future, the diff result could be used to skip the TIP crate
                // compilation (Phase 2), but intermediate dep crates must always cascade.
                if let (Some(old), Some(new)) =
                    (old_objects.as_deref(), object_cache.get(&crate_name))
                {
                    let diff = crate::build::diff::diff_objects(old, new);
                    tracing::debug!(
                        "Assembly diff for {crate_name}: {} changed symbols, cascade={}",
                        diff.changed_symbols.len(),
                        diff.needs_downstream_recompile
                    );
                }

                // Always cascade: recompile workspace dependents so their rlibs have
                // consistent SVH references to the just-recompiled crate.
                for dependent in self.workspace_dependents_of(&crate_name) {
                    if dependent != tip_name && !compiled.contains(&dependent) {
                        tracing::debug!(
                            "Cascade: recompiling {dependent} (depends on recompiled {crate_name})"
                        );
                        crates_to_compile.push(dependent);
                    }
                }
            }

            // If the tip crate has a lib target (src/lib.rs + src/main.rs), compile it
            // before the bin target so the bin links against the fresh lib rlib.
            let lib_key = format!("{tip_name}.lib");
            if let Some(lib_args) = workspace_rustc_args.get(&lib_key) {
                // Check rlib mod time before compilation to verify it actually changes
                let rlib_pre = self.find_rlib_for_crate(&tip_name, lib_args);
                let pre_modified = rlib_pre
                    .as_ref()
                    .and_then(|p| std::fs::metadata(p).ok())
                    .and_then(|m| m.modified().ok());

                tracing::info!("Compiling tip lib target: {lib_key}");
                if let Err(e) = self.compile_dep_crate(&tip_name, lib_args).await {
                    tracing::warn!("Failed to compile tip lib target: {e}");
                } else if let Some(rlib_path) = self.find_rlib_for_crate(&tip_name, lib_args) {
                    let post_modified = std::fs::metadata(&rlib_path)
                        .ok()
                        .and_then(|m| m.modified().ok());
                    let rlib_changed = match (pre_modified, post_modified) {
                        (Some(pre), Some(post)) => post > pre,
                        _ => true, // assume changed if we can't check
                    };
                    tracing::info!(
                        "Found lib rlib at: {} (modified={})",
                        rlib_path.display(),
                        rlib_changed,
                    );

                    match object_cache.cache_from_rlib(&lib_key, &rlib_path) {
                        Ok(()) => {
                            let count = object_cache.get(&lib_key).map(|v| v.len()).unwrap_or(0);
                            tracing::info!("Cached {count} objects from tip lib rlib");
                            modified_crates.insert(lib_key);
                        }
                        Err(e) => tracing::warn!("Failed to cache tip lib objects: {e}"),
                    }
                } else {
                    tracing::warn!("Could not find rlib for tip lib target {tip_name}");
                }
            } else {
                tracing::debug!(
                    "No lib target for tip crate (key '{lib_key}' not in workspace_rustc_args, keys={:?})",
                    workspace_rustc_args.keys().collect::<Vec<_>>()
                );
            }

            thin_pre_state = Some((object_cache, modified_crates));
        }

        // Run the cargo build to produce our artifacts.
        // For Thin mode this compiles the tip's bin target (deps already updated above).
        let mut artifacts = self.cargo_build(ctx).await?;

        // Write the build artifacts to the bundle on the disk
        match &ctx.mode {
            BuildMode::Thin {
                aslr_reference,
                cache,
                ..
            } => {
                let (mut object_cache, mut modified_crates) =
                    thin_pre_state.expect("thin_pre_state must be set for Thin mode");
                let tip_name = self.tip_crate_name();

                // Cache tip crate objects from the FRESH linker args (from the just-completed
                // thin build, not the stale ones from ctx.mode's fat build).
                let tip_args = artifacts
                    .workspace_rustc_args
                    .get(&tip_name)
                    .cloned()
                    .unwrap_or_default();
                let tip_object_paths: Vec<PathBuf> = tip_args
                    .link_args
                    .iter()
                    .filter(|arg| arg.ends_with(".rcgu.o"))
                    .map(PathBuf::from)
                    .collect();
                if !tip_object_paths.is_empty() {
                    if let Err(e) = object_cache.cache_from_paths(&tip_name, &tip_object_paths) {
                        tracing::warn!("Failed to cache tip crate objects: {e}");
                    }
                }
                modified_crates.insert(tip_name.clone());

                // Write dep crate objects from the cache to temp files for the linker.
                let dep_object_dir = self.session_cache_dir().join("dep_objects");
                std::fs::create_dir_all(&dep_object_dir)
                    .context("Failed to create dep objects directory")?;
                let mut extra_object_paths = Vec::new();

                // Collect objects from all modified dep crates (not tip — tip objects come
                // from the linker args and are already handled by write_patch).
                for dep_name in modified_crates.iter().filter(|c| *c != &tip_name) {
                    if let Some(objects) = object_cache.get(dep_name) {
                        for obj in objects {
                            let obj_path =
                                dep_object_dir.join(format!("{}-{}", dep_name, obj.name));
                            std::fs::write(&obj_path, &obj.data).with_context(|| {
                                format!("Failed to write dep object file: {}", obj_path.display())
                            })?;
                            extra_object_paths.push(obj_path);
                        }
                    }
                }

                tracing::info!(
                    "Linking patch with {} tip objects + {} dep objects from {} modified crates ({:?})",
                    tip_object_paths.len(),
                    extra_object_paths.len(),
                    modified_crates.len(),
                    modified_crates,
                );

                // Link the patch dylib with all accumulated objects.
                self.write_patch(
                    ctx,
                    *aslr_reference,
                    &mut artifacts,
                    cache,
                    &tip_args,
                    &extra_object_paths,
                )
                .await?;

                // Return updated state in artifacts so AppBuilder can persist it.
                artifacts.object_cache = object_cache;
                artifacts.modified_crates = modified_crates;
            }

            BuildMode::Base { .. } | BuildMode::Fat => {
                ctx.status_start_bundle();

                self.strip_binary(&artifacts).await?;
                self.write_executable(ctx, &artifacts.exe, &mut artifacts.assets)
                    .await
                    .context("Failed to write executable")?;
                let tip_args = artifacts.workspace_rustc_args
                    .get(&self.tip_crate_name())
                    .cloned()
                    .unwrap_or_default();
                self.write_frameworks(ctx, &tip_args)
                    .await
                    .context("Failed to write frameworks")?;
                self.write_assets(ctx, &artifacts.assets)
                    .await
                    .context("Failed to write assets")?;
                self.write_metadata()
                    .await
                    .context("Failed to write metadata")?;
                self.optimize(ctx)
                    .await
                    .context("Failed to optimize build")?;
                self.assemble(ctx)
                    .await
                    .context("Failed to assemble build")?;

                tracing::debug!("Bundle created at {}", self.root_dir().display());
            }
        }

        // Populate the patch cache if we're in fat mode
        if matches!(ctx.mode, BuildMode::Fat) {
            artifacts.patch_cache = Some(Arc::new(self.create_patch_cache(&artifacts.exe)?));
        }

        // Calculate some final metadata for logging
        let time_taken = SystemTime::now()
            .duration_since(time_start)
            .map(|d| d.as_millis())
            .unwrap_or_default();
        tracing::debug!(
            telemetry = %serde_json::json!({
                "event": "build_and_bundle_complete",
                "time_taken": time_taken,
                "mode": match ctx.mode {
                    BuildMode::Base { .. } => "base",
                    BuildMode::Fat => "fat",
                    BuildMode::Thin { .. } => "thin",
                },
                "blah": 123,
                "triple": self.triple.to_string(),
                "format": self.bundle.to_string(),
                "num_dependencies": self.workspace.krates.len(),
            }),
            "Build completed in {time_taken}ms",
        );

        Ok(artifacts)
    }

    /// Run the cargo build by assembling the build command and executing it.
    ///
    /// This method needs to be very careful with processing output since errors being swallowed will
    /// be very confusing to the user.
    async fn cargo_build(&self, ctx: &BuildContext) -> Result<BuildArtifacts> {
        let time_start = SystemTime::now();

        // Extract the unit count of the crate graph so build_cargo has more accurate data
        // "Thin" builds only build the final exe, so we only need to build one crate
        let crate_count = match ctx.mode {
            BuildMode::Thin { .. } => 1,
            _ => self.get_unit_count_estimate(&ctx.mode).await,
        };

        // Update the status to show that we're starting the build and how many crates we expect to build
        ctx.status_starting_build(crate_count);

        let mut cmd = self.build_command(&ctx.mode)?;
        tracing::debug!(dx_src = ?TraceSrc::Build, "Executing cargo for {} using {}", self.bundle, self.triple);

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

            // If raw JSON diagnostics are requested, relay the line directly
            if self.raw_json_diagnostics {
                println!("{}", line);
            }

            let Some(Ok(message)) = Message::parse_stream(std::io::Cursor::new(line)).next() else {
                continue;
            };

            match message {
                Message::BuildScriptExecuted(_) => units_compiled += 1,
                Message::CompilerMessage(msg) => ctx.status_build_diagnostic(msg.message),
                Message::TextLine(line) => {
                    // Handle the case where we're getting lines directly from rustc.
                    // These are in a different format than the normal cargo output, though I imagine
                    // this parsing code is quite fragile/sensitive to changes in cargo, cargo_metadata, rustc, etc.
                    #[derive(Deserialize)]
                    struct RustcArtifact {
                        artifact: PathBuf,
                        emit: String,
                    }

                    // These outputs look something like:
                    //
                    // { "artifact":"target/debug/deps/libdioxus_core-4f2a0b3c1e5f8b7c.rlib", "emit":"link" }
                    //
                    // There are other outputs like depinfo that we might be interested in the future.
                    if let Ok(artifact) = serde_json::from_str::<RustcArtifact>(&line) {
                        if artifact.emit == "link" {
                            output_location = Some(artifact.artifact);
                        }
                    }

                    // Handle direct rustc diagnostics
                    if let Ok(diag) = serde_json::from_str::<Diagnostic>(&line) {
                        ctx.status_build_diagnostic(diag);
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
                    ctx.status_build_progress(units_compiled, crate_count, artifact.target.name);
                    output_location = artifact.executable.map(Into::into);
                }
                // todo: this can occasionally swallow errors, so we should figure out what exactly is going wrong
                //       since that is a really bad user experience.
                Message::BuildFinished(finished) => {
                    if !finished.success {
                        bail!(
                            "cargo build finished with errors for target: {} [{}]",
                            self.main_target,
                            self.triple
                        );
                    }
                }
                _ => {}
            }
        }

        // Load per-crate rustc args from the wrapper directory.
        // Each workspace crate compiled through the wrapper has its own JSON file.
        // For crates with both lib.rs and main.rs, the lib target is stored as
        // "{crate_name}.lib.json" (key: "{crate_name}.lib") and the bin target as
        // "{crate_name}.json" (key: "{crate_name}").
        let mut workspace_rustc_args = HashMap::new();
        let args_dir = self.rustc_wrapper_args_dir();
        if let Ok(entries) = std::fs::read_dir(&args_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().is_some_and(|e| e == "json") {
                    if let Ok(contents) = std::fs::read_to_string(&path) {
                        if let Ok(args) = serde_json::from_str::<RustcArgs>(&contents) {
                            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                                workspace_rustc_args.insert(stem.to_string(), args);
                            }
                        }
                    }
                }
            }
        }
        tracing::debug!(
            "Loaded workspace rustc args from {}: keys={:?}",
            args_dir.display(),
            workspace_rustc_args.keys().collect::<Vec<_>>(),
        );

        // If there's any warnings from the linker, we should print them out
        if let Ok(linker_warnings) = std::fs::read_to_string(self.link_err_file()) {
            if !linker_warnings.is_empty() {
                if output_location.is_none() {
                    tracing::error!("Linker warnings: {}", linker_warnings);
                } else {
                    tracing::debug!("Linker warnings: {}", linker_warnings);
                }
            }
        }

        // Collect the linker args and attach them to the tip crate's entry
        let link_args: Vec<String> = std::fs::read_to_string(self.link_args_file())
            .context("Failed to read link args from file")?
            .lines()
            .map(|s| s.to_string())
            .collect();

        let tip_crate_name = self.tip_crate_name();
        if let Some(tip_args) = workspace_rustc_args.get_mut(&tip_crate_name) {
            tip_args.link_args = link_args;
        }

        let exe = output_location.context("Cargo build failed - no output location. Toggle tracing mode (press `t`) for more information.")?;

        // Fat builds need to be linked with the fat linker. Would also like to link here for thin builds
        if matches!(ctx.mode, BuildMode::Fat) {
            ctx.status_starting_link();
            let link_start = SystemTime::now();
            let tip_args = workspace_rustc_args
                .get(&tip_crate_name)
                .cloned()
                .unwrap_or_default();
            self.run_fat_link(&exe, &tip_args).await?;
            tracing::debug!(
                "Fat linking completed in {}us",
                SystemTime::now()
                    .duration_since(link_start)
                    .unwrap()
                    .as_micros()
            );
        }

        let assets = self.collect_assets(&exe, ctx).await?;
        let time_end = SystemTime::now();
        let mode = ctx.mode.clone();
        let depinfo = RustcDepInfo::from_file(&exe.with_extension("d")).unwrap_or_default();

        tracing::debug!(
            "Build completed successfully in {}us: {:?}",
            time_end.duration_since(time_start).unwrap().as_micros(),
            exe
        );

        Ok(BuildArtifacts {
            time_end,
            exe,
            workspace_rustc_args,
            time_start,
            assets,
            mode,
            depinfo,
            root_dir: self.root_dir(),
            patch_cache: None,
            build_id: ctx.build_id,
            object_cache: ObjectCache::new(),
            modified_crates: HashSet::new(),
        })
    }

    /// Collect the assets from the final executable and modify the binary in place to point to the right
    /// hashed asset location.
    async fn collect_assets(&self, exe: &Path, ctx: &BuildContext) -> Result<AssetManifest> {
        // And then add from the exe directly, just in case it's LTO compiled and has no incremental cache
        if self.skip_assets {
            return Ok(AssetManifest::default());
        }

        ctx.status_extracting_assets();

        let mut manifest = super::assets::extract_assets_from_file(exe).await?;

        // If the user has a public dir, we submit all the entries there as assets too
        //
        // These don't receive a hash in their filename, since they're user-provided static assets
        // We only do this for web builds
        if matches!(self.bundle, BundleFormat::Web)
            && matches!(ctx.mode, BuildMode::Base { .. } | BuildMode::Fat)
        {
            if let Some(dir) = self.user_public_dir() {
                for entry in walkdir::WalkDir::new(&dir)
                    .into_iter()
                    .filter_map(|e| e.ok())
                    .filter(|e| e.file_type().is_file())
                {
                    let from = entry.path().to_path_buf();
                    let relative_path = from.strip_prefix(&dir).unwrap();
                    let to = format!("../{}", relative_path.display());
                    manifest.insert_asset(BundledAsset::new(
                        from.to_string_lossy().as_ref(),
                        to.as_str(),
                        AssetOptionsBuilder::new()
                            .with_hash_suffix(false)
                            .into_asset_options(),
                    ));
                }
            }
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
        match self.bundle {
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
            BundleFormat::Web => {
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
            //
            // These are all super simple, just copy the exe into the folder
            // eventually, perhaps, maybe strip + encrypt the exe?
            BundleFormat::Android
            | BundleFormat::MacOS
            | BundleFormat::Windows
            | BundleFormat::Linux
            | BundleFormat::Ios
            | BundleFormat::Server => {
                std::fs::create_dir_all(self.exe_dir())?;
                std::fs::copy(exe, self.main_exe())?;
            }
        }

        Ok(())
    }

    async fn write_frameworks(&self, _ctx: &BuildContext, direct_rustc: &RustcArgs) -> Result<()> {
        let framework_dir = self.frameworks_folder();

        // We have some prebuilt stuff that needs to be copied into the framework dir
        let openssl_dir = AndroidTools::openssl_lib_dir(&self.triple);
        let openssl_dir_disp = openssl_dir.display().to_string();

        for arg in &direct_rustc.link_args {
            // todo - how do we handle windows dlls? we don't want to bundle the system dlls
            // for now, we don't do anything with dlls, and only use .dylibs and .so files

            // Write dylibs and dlls to the frameworks folder
            if arg.ends_with(".dylib") | arg.ends_with(".so") {
                let from = PathBuf::from(arg);
                let to = framework_dir.join(from.file_name().unwrap());
                _ = std::fs::remove_file(&to);

                tracing::debug!("Copying framework from {from:?} to {to:?}");

                _ = std::fs::create_dir_all(&framework_dir);

                // in dev and on normal oses, we want to symlink the file
                // otherwise, just copy it (since in release you want to distribute the framework)
                if cfg!(any(windows, unix)) && !self.release {
                    #[cfg(windows)]
                    std::os::windows::fs::symlink_file(from, to).with_context(|| {
                        "Failed to symlink framework into bundle: {from:?} -> {to:?}"
                    })?;

                    #[cfg(unix)]
                    std::os::unix::fs::symlink(from, to).with_context(|| {
                        "Failed to symlink framework into bundle: {from:?} -> {to:?}"
                    })?;
                } else {
                    std::fs::copy(from, to)?;
                }
            }

            // Always create the framework dir for android
            if self.bundle == BundleFormat::Android {
                _ = std::fs::create_dir_all(&framework_dir);
            }

            // On android, the c++_shared flag means we need to copy the libc++_shared.so precompiled
            // library to the jniLibs folder
            if self.bundle == BundleFormat::Android && arg.contains("-lc++_shared") {
                std::fs::copy(
                    self.workspace.android_tools()?.libcpp_shared(&self.triple),
                    framework_dir.join("libc++_shared.so"),
                )
                .with_context(|| "Failed to copy libc++_shared.so into bundle")?;
            }

            // Copy over libssl and libcrypto if they are present in the link args
            if self.bundle == BundleFormat::Android && arg.contains(openssl_dir_disp.as_str()) {
                let libssl_source = openssl_dir.join("libssl.so");
                let libcrypto_source = openssl_dir.join("libcrypto.so");
                let libssl_target = framework_dir.join("libssl.so");
                let libcrypto_target = framework_dir.join("libcrypto.so");
                std::fs::copy(&libssl_source, &libssl_target).with_context(|| {
                    format!("Failed to copy libssl.so into bundle\nfrom {libssl_source:?}\nto {libssl_target:?}")
                })?;
                std::fs::copy(&libcrypto_source, &libcrypto_target).with_context(
                    || format!("Failed to copy libcrypto.so into bundle\nfrom {libcrypto_source:?}\nto {libcrypto_target:?}"),
                )?;
            }
        }

        Ok(())
    }

    fn frameworks_folder(&self) -> PathBuf {
        match self.triple.operating_system {
            OperatingSystem::Darwin(_) | OperatingSystem::MacOSX(_) => {
                self.root_dir().join("Contents").join("Frameworks")
            }
            OperatingSystem::IOS(_) => self.root_dir().join("Frameworks"),
            OperatingSystem::Linux if self.bundle == BundleFormat::Android => {
                let arch = match self.triple.architecture {
                    Architecture::Aarch64(_) => "arm64-v8a",
                    Architecture::Arm(_) => "armeabi-v7a",
                    Architecture::X86_32(_) => "x86",
                    Architecture::X86_64 => "x86_64",
                    _ => panic!(
                        "Unsupported architecture for Android: {:?}",
                        self.triple.architecture
                    ),
                };

                self.root_dir()
                    .join("app")
                    .join("src")
                    .join("main")
                    .join("jniLibs")
                    .join(arch)
            }
            OperatingSystem::Linux | OperatingSystem::Windows => self.root_dir(),
            _ => self.root_dir(),
        }
    }

    /// Copy the assets out of the manifest and into the target location
    ///
    /// Should be the same on all platforms - just copy over the assets from the manifest into the output directory
    async fn write_assets(&self, ctx: &BuildContext, assets: &AssetManifest) -> Result<()> {
        // Server doesn't need assets - web will provide them
        if !ctx.is_primary_build() {
            return Ok(());
        }

        let asset_dir = self.asset_dir();

        // First, clear the asset dir of any files that don't exist in the new manifest
        _ = std::fs::create_dir_all(&asset_dir);

        // Create a set of all the paths that new files will be bundled to
        let mut keep_bundled_output_paths: HashSet<_> = assets
            .unique_assets()
            .map(|a| asset_dir.join(a.bundled_path()))
            .collect();

        // The CLI creates a .manifest.json file in the asset dir to keep track of the assets and
        // other build metadata. If we can't parse this file (or the CLI version changed), then we
        // want to re-copy all the assets rather than trying to do an incremental update.
        let clear_cache = self
            .load_manifest()
            .map(|manifest| manifest.cli_version != crate::VERSION.as_str())
            .unwrap_or(true);
        if clear_cache {
            keep_bundled_output_paths.clear();
        }

        tracing::trace!(
            "Keeping bundled output paths: {:#?}",
            keep_bundled_output_paths
        );

        // todo(jon): we also want to eventually include options for each asset's optimization and compression, which we currently aren't
        let mut assets_to_transfer = vec![];

        // Queue the bundled assets
        for bundled in assets.unique_assets() {
            let from = PathBuf::from(bundled.absolute_source_path());
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

        // Remove the wasm dir if we packaged it to an "asset"-type app
        if self.should_bundle_to_asset() {
            _ = std::fs::remove_dir_all(self.wasm_bindgen_out_dir());
        }

        // Write the version file so we know what version of the optimizer we used
        self.write_app_manifest(assets).await?;

        Ok(())
    }

    /// Run our custom linker setup to generate a patch file in the right location
    ///
    /// This should be the only case where the cargo output is a "dummy" file and requires us to
    /// manually do any linking.
    ///
    /// We also run some post processing steps here, like extracting out any new assets.
    /// Run our custom linker setup to generate a patch file in the right location.
    ///
    /// `extra_objects` contains additional object file paths from compiled workspace dep crates
    /// that should be included in the patch dylib. These are combined with the tip crate's
    /// `.rcgu.o` files extracted from linker args, creating a self-contained patch.
    async fn write_patch(
        &self,
        ctx: &BuildContext,
        aslr_reference: u64,
        artifacts: &mut BuildArtifacts,
        cache: &Arc<HotpatchModuleCache>,
        rustc_args: &RustcArgs,
        extra_objects: &[PathBuf],
    ) -> Result<()> {
        ctx.status_hotpatching();

        let tip_crate_name = self.tip_crate_name();
        let args = artifacts
            .workspace_rustc_args
            .get(&tip_crate_name)
            .map(|a| a.link_args.clone())
            .unwrap_or_default();

        // Extract out the incremental object files.
        //
        // This is sadly somewhat of a hack, but it might be a moderately reliable hack.
        //
        // When rustc links your project, it passes the args as how a linker would expect, but with
        // a somewhat reliable ordering. These are all internal details to cargo/rustc, so we can't
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
        // dyld will complain.
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
        let mut dylibs = vec![];
        let mut object_files = args
            .iter()
            .filter(|arg| arg.ends_with(".rcgu.o"))
            .sorted()
            .map(PathBuf::from)
            .collect::<Vec<_>>();

        // Include additional object files from workspace dep crates
        object_files.extend_from_slice(extra_objects);

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
        if !self.is_wasm_or_wasi() {
            let stub_bytes = crate::build::create_undefined_symbol_stub(
                cache,
                &object_files,
                &self.triple,
                aslr_reference,
            )
            .expect("failed to resolve patch symbols");

            // Currently we're dropping stub.o in the exe dir, but should probably just move to a tempfile?
            let patch_file = self.main_exe().with_file_name("stub.o");
            std::fs::write(&patch_file, stub_bytes)?;
            object_files.push(patch_file);

            // Add the dylibs/sos to the linker args
            // Make sure to use the one in the bundle, not the ones in the target dir or system.
            for arg in &rustc_args.link_args {
                if arg.ends_with(".dylib") || arg.ends_with(".so") {
                    let path = PathBuf::from(arg);
                    dylibs.push(self.frameworks_folder().join(path.file_name().unwrap()));
                }
            }
        }

        // And now we can run the linker with our new args
        let linker = self.select_linker()?;
        let out_exe = self.patch_exe(artifacts.time_start);
        let out_arg = match self.triple.operating_system {
            OperatingSystem::Windows => vec![format!("/OUT:{}", out_exe.display())],
            _ => vec!["-o".to_string(), out_exe.display().to_string()],
        };

        tracing::trace!("Linking with {:?} using args: {:#?}", linker, object_files);

        let mut out_args: Vec<OsString> = vec![];
        out_args.extend(object_files.iter().map(Into::into));
        out_args.extend(dylibs.iter().map(Into::into));
        out_args.extend(self.thin_link_args(&args)?.iter().map(Into::into));
        out_args.extend(out_arg.iter().map(Into::into));

        if cfg!(windows) {
            let cmd_contents: String = out_args
                .iter()
                .map(|s| format!("\"{}\"", s.to_string_lossy()))
                .join(" ");
            std::fs::write(self.windows_command_file(), cmd_contents)
                .context("Failed to write linker command file")?;
            out_args = vec![format!("@{}", self.windows_command_file().display()).into()];
        }

        // Add more search paths for the linker
        let mut command_envs = rustc_args.envs.clone();

        // On linux, we need to set a more complete PATH for the linker to find its libraries
        if cfg!(target_os = "linux") {
            command_envs.push(("PATH".to_string(), std::env::var("PATH").unwrap()));
        }

        // Run the linker directly!
        //
        // We dump its output directly into the patch exe location which is different than how rustc
        // does it since it uses llvm-objcopy into the `target/debug/` folder.
        let res = Command::new(linker)
            .args(out_args)
            .env_clear()
            .envs(command_envs)
            .output()
            .await?;

        if !res.stderr.is_empty() {
            let errs = String::from_utf8_lossy(&res.stderr);
            if !self.patch_exe(artifacts.time_start).exists() || !res.status.success() {
                tracing::error!(
                    telemetry = %serde_json::json!({ "event": "hotpatch_linker_failed" }),
                    "Failed to generate patch: {}",
                    errs.trim()
                );
            } else {
                tracing::trace!("Linker output during thin linking: {}", errs.trim());
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
            _ = std::fs::remove_file(PathBuf::from(args[idx + 1].as_str()));
        }

        // Now extract the assets from the fat binary
        artifacts.assets = self
            .collect_assets(&self.patch_exe(artifacts.time_start), ctx)
            .await?;

        // If this is a web build, reset the index.html file in case it was modified by SSG
        self.write_index_html(&artifacts.assets)
            .context("Failed to write index.html")?;

        // Clean up the temps manually
        // todo: we might want to keep them around for debugging purposes
        for file in object_files {
            _ = std::fs::remove_file(file);
        }

        Ok(())
    }

    /// Take the original args passed to the "fat" build and then create the "thin" variant.
    ///
    /// This is basically just stripping away the rlibs and other libraries that will be satisfied
    /// by our stub step.
    fn thin_link_args(&self, original_args: &[String]) -> Result<Vec<String>> {
        let mut out_args = vec![];

        match self.linker_flavor() {
            // wasm32-unknown-unknown -> use wasm-ld (gnu-lld)
            //
            // We need to import a few things - namely the memory and ifunc table.
            //
            // We can safely export everything, I believe, though that led to issues with the "fat"
            // binaries that also might lead to issues here too. wasm-bindgen chokes on some symbols
            // and the resulting JS has issues.
            //
            // We turn on both --pie and --experimental-pic but I think we only need --pie.
            //
            // We don't use *any* of the original linker args since they do lots of custom exports
            // and other things that we don't need.
            //
            // The trickiest one here is -Crelocation-model=pic, which forces data symbols
            // into a GOT, making it possible to import them from the main module.
            //
            // I think we can make relocation-model=pic work for non-wasm platforms, enabling
            // fully relocatable modules with no host coordination in lieu of sending out
            // the aslr slide at runtime.
            LinkerFlavor::WasmLld => {
                out_args.extend([
                    "--fatal-warnings".to_string(),
                    "--verbose".to_string(),
                    "--import-memory".to_string(),
                    "--import-table".to_string(),
                    "--growable-table".to_string(),
                    "--export".to_string(),
                    "main".to_string(),
                    "--allow-undefined".to_string(),
                    "--no-demangle".to_string(),
                    "--no-entry".to_string(),
                    "--pie".to_string(),
                    "--experimental-pic".to_string(),
                ]);

                // retain exports so post-processing has hooks to work with
                for (idx, arg) in original_args.iter().enumerate() {
                    if *arg == "--export" {
                        out_args.push(arg.to_string());
                        out_args.push(original_args[idx + 1].to_string());
                    }
                }
            }

            // This uses "cc" and these args need to be ld compatible
            //
            // Most importantly, we want to pass `-dylib` to both CC and the linker to indicate that
            // we want to generate the shared library instead of an executable.
            LinkerFlavor::Darwin => {
                out_args.extend(["-Wl,-dylib".to_string()]);

                // Preserve the original args. We only preserve:
                // -framework
                // -arch
                // -lxyz
                // There might be more, but some flags might break our setup.
                for (idx, arg) in original_args.iter().enumerate() {
                    if *arg == "-framework"
                        || *arg == "-arch"
                        || *arg == "-L"
                        || *arg == "-target"
                        || *arg == "-isysroot"
                    {
                        out_args.push(arg.to_string());
                        out_args.push(original_args[idx + 1].to_string());
                    }

                    if arg.starts_with("-l")
                        || arg.starts_with("-m")
                        || arg.starts_with("-nodefaultlibs")
                    {
                        out_args.push(arg.to_string());
                    }
                }
            }

            // android/linux need to be compatible with lld
            //
            // android currently drags along its own libraries and other zany flags
            LinkerFlavor::Gnu => {
                out_args.extend([
                    "-shared".to_string(),
                    "-Wl,--eh-frame-hdr".to_string(),
                    "-Wl,-z,noexecstack".to_string(),
                    "-Wl,-z,relro,-z,now".to_string(),
                    "-nodefaultlibs".to_string(),
                    "-Wl,-Bdynamic".to_string(),
                ]);

                // Preserve the original args. We only preserve:
                // -L <path>
                // -arch
                // -lxyz
                // There might be more, but some flags might break our setup.
                for (idx, arg) in original_args.iter().enumerate() {
                    if *arg == "-L" {
                        out_args.push(arg.to_string());
                        out_args.push(original_args[idx + 1].to_string());
                    }

                    if arg.starts_with("-l")
                        || arg.starts_with("-m")
                        || arg.starts_with("-Wl,--target=")
                        || arg.starts_with("-Wl,-fuse-ld")
                        || arg.starts_with("-fuse-ld")
                        || arg.contains("-ld-path")
                    {
                        out_args.push(arg.to_string());
                    }
                }
            }

            LinkerFlavor::Msvc => {
                out_args.extend([
                    "shlwapi.lib".to_string(),
                    "kernel32.lib".to_string(),
                    "advapi32.lib".to_string(),
                    "ntdll.lib".to_string(),
                    "userenv.lib".to_string(),
                    "ws2_32.lib".to_string(),
                    "dbghelp.lib".to_string(),
                    "/defaultlib:msvcrt".to_string(),
                    "/DLL".to_string(),
                    "/DEBUG".to_string(),
                    "/PDBALTPATH:%_PDB%".to_string(),
                    "/EXPORT:main".to_string(),
                    "/HIGHENTROPYVA:NO".to_string(),
                ]);
            }

            LinkerFlavor::Unsupported => {
                bail!("Unsupported platform for thin linking")
            }
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
            time_start
                .duration_since(UNIX_EPOCH)
                .map(|f| f.as_millis())
                .unwrap_or(0),
        ));

        let extension = match self.linker_flavor() {
            LinkerFlavor::Darwin => "dylib",
            LinkerFlavor::Gnu => "so",
            LinkerFlavor::WasmLld => "wasm",
            LinkerFlavor::Msvc => "dll",
            LinkerFlavor::Unsupported => "",
        };

        path.with_extension(extension)
    }

    /// When we link together the fat binary, we need to make sure every `.o` file in *every* rlib
    /// is taken into account. This is the same work that the rust compiler does when assembling
    /// staticlibs.
    ///
    /// <https://github.com/rust-lang/rust/blob/191df20fcad9331d3a948aa8e8556775ec3fe69d/compiler/rustc_codegen_ssa/src/back/link.rs#L448>
    ///
    /// Since we're going to be passing these to the linker, we need to make sure and not provide any
    /// weird files (like the rmeta) file that rustc generates.
    ///
    /// We discovered the need for this after running into issues with wasm-ld not being able to
    /// handle the rmeta file.
    ///
    /// <https://github.com/llvm/llvm-project/issues/55786>
    ///
    /// Also, crates might not drag in all their dependent code. The monorphizer won't lift trait-based generics:
    ///
    /// <https://github.com/rust-lang/rust/blob/191df20fcad9331d3a948aa8e8556775ec3fe69d/compiler/rustc_monomorphize/src/collector.rs>
    ///
    /// When Rust normally handles this, it uses the +whole-archive directive which adjusts how the rlib
    /// is written to disk.
    ///
    /// Since creating this object file can be a lot of work, we cache it in the target dir by hashing
    /// the names of the rlibs in the command and storing it in the target dir. That way, when we run
    /// this command again, we can just used the cached object file.
    ///
    /// In theory, we only need to do this for every crate accessible by the current crate, but that's
    /// hard acquire without knowing the exported symbols from each crate.
    ///
    /// todo: I think we can traverse our immediate dependencies and inspect their symbols, unless they `pub use` a crate
    /// todo: we should try and make this faster with memmapping
    pub(crate) async fn run_fat_link(&self, exe: &Path, rustc_args: &RustcArgs) -> Result<()> {
        // Filter out the rlib files from the arguments
        let rlibs = rustc_args
            .link_args
            .iter()
            .filter(|arg| arg.ends_with(".rlib"))
            .map(PathBuf::from)
            .collect::<Vec<_>>();

        // Acquire a hash from the rlib names, sizes, modified times, and dx's git commit hash
        // This ensures that any changes in dx or the rlibs will cause a new hash to be generated
        // The hash relies on both dx and rustc hashes, so it should be thoroughly unique. Keep it
        // short to avoid long file names.
        let hash_id = Uuid::new_v5(
            &Uuid::NAMESPACE_OID,
            rlibs
                .iter()
                .map(|p| {
                    format!(
                        "{}-{}-{}-{}",
                        p.file_name().unwrap().to_string_lossy(),
                        p.metadata().map(|m| m.len()).unwrap_or_default(),
                        p.metadata()
                            .ok()
                            .and_then(|m| m.modified().ok())
                            .and_then(|f| f.duration_since(UNIX_EPOCH).map(|f| f.as_secs()).ok())
                            .unwrap_or_default(),
                        crate::dx_build_info::GIT_COMMIT_HASH.unwrap_or_default()
                    )
                })
                .collect::<String>()
                .as_bytes(),
        )
        .to_string()
        .chars()
        .take(8)
        .collect::<String>();

        // Check if we already have a cached object file
        let out_ar_path = exe.with_file_name(format!("libdeps-{hash_id}.a",));
        let out_rlibs_list = exe.with_file_name(format!("rlibs-{hash_id}.txt"));
        let mut archive_has_contents = out_ar_path.exists();

        // Use the rlibs list if it exists
        let mut compiler_rlibs = std::fs::read_to_string(&out_rlibs_list)
            .ok()
            .map(|s| s.lines().map(PathBuf::from).collect::<Vec<_>>())
            .unwrap_or_default();

        // Create it by dumping all the rlibs into it
        // This will include the std rlibs too, which can severely bloat the size of the archive
        //
        // The nature of this process involves making extremely fat archives, so we should try and
        // speed up the future linking process by caching the archive.
        //
        // Since we're using the git hash for the CLI entropy, debug builds should always regenerate
        // the archive since their hash might not change, but the logic might.
        if !archive_has_contents || cfg!(debug_assertions) {
            compiler_rlibs.clear();

            let mut bytes = vec![];
            let mut out_ar = ar::Builder::new(&mut bytes);
            for rlib in &rlibs {
                // Skip compiler rlibs since they're missing bitcode
                //
                // https://github.com/rust-lang/rust/issues/94232#issuecomment-1048342201
                //
                // if the rlib is not in the target directory, we skip it.
                if !rlib.starts_with(self.workspace_dir()) {
                    compiler_rlibs.push(rlib.clone());
                    tracing::trace!("Skipping rlib: {:?}", rlib);
                    continue;
                }

                tracing::trace!("Adding rlib to staticlib: {:?}", rlib);

                let rlib_contents = std::fs::read(rlib)?;
                let mut reader = ar::Archive::new(std::io::Cursor::new(rlib_contents));
                let mut keep_linker_rlib = false;
                while let Some(Ok(object_file)) = reader.next_entry() {
                    let name = std::str::from_utf8(object_file.header().identifier()).unwrap();
                    if name.ends_with(".rmeta") {
                        continue;
                    }

                    if object_file.header().size() == 0 {
                        continue;
                    }

                    // rlibs might contain dlls/sos/lib files which we don't want to include
                    //
                    // This catches .dylib, .so, .dll, .lib, .o, etc files that are not compatible with
                    // our "fat archive" linking process.
                    //
                    // We only trust `.rcgu.o` files to make it into the --all_load archive.
                    // This is a temporary stopgap to prevent issues with libraries that generate
                    // object files that are not compatible with --all_load.
                    // see https://github.com/DioxusLabs/dioxus/issues/4237
                    if !(name.ends_with(".rcgu.o") || name.ends_with(".obj")) {
                        keep_linker_rlib = true;
                        continue;
                    }

                    archive_has_contents = true;
                    out_ar
                        .append(&object_file.header().clone(), object_file)
                        .context("Failed to add object file to archive")?;
                }

                // Some rlibs contain weird artifacts that we don't want to include in the fat archive.
                // However, we still want them around in the linker in case the regular linker can handle them.
                if keep_linker_rlib {
                    compiler_rlibs.push(rlib.clone());
                }
            }

            let bytes = out_ar.into_inner().context("Failed to finalize archive")?;
            std::fs::write(&out_ar_path, bytes).context("Failed to write archive")?;
            tracing::debug!("Wrote fat archive to {:?}", out_ar_path);

            // Run the ranlib command to index the archive. This slows down this process a bit,
            // but is necessary for some linkers to work properly.
            // We ignore its error in case it doesn't recognize the architecture
            if self.linker_flavor() == LinkerFlavor::Darwin {
                if let Some(ranlib) = Workspace::select_ranlib() {
                    _ = Command::new(ranlib).arg(&out_ar_path).output().await;
                }
            }
        }

        compiler_rlibs.dedup();

        // We're going to replace the first rlib in the args with our fat archive
        // And then remove the rest of the rlibs
        //
        // We also need to insert the -force_load flag to force the linker to load the archive
        let mut args: Vec<_> = rustc_args.link_args.clone();
        if let Some(last_object) = args.iter().rposition(|arg| arg.ends_with(".o")) {
            if archive_has_contents {
                match self.linker_flavor() {
                    LinkerFlavor::WasmLld => {
                        args.insert(last_object, "--whole-archive".to_string());
                        args.insert(last_object + 1, out_ar_path.display().to_string());
                        args.insert(last_object + 2, "--no-whole-archive".to_string());
                        args.retain(|arg| !arg.ends_with(".rlib"));
                        for rlib in compiler_rlibs.iter().rev() {
                            args.insert(last_object + 3, rlib.display().to_string());
                        }
                    }
                    LinkerFlavor::Gnu => {
                        args.insert(last_object, "-Wl,--whole-archive".to_string());
                        args.insert(last_object + 1, out_ar_path.display().to_string());
                        args.insert(last_object + 2, "-Wl,--no-whole-archive".to_string());
                        args.retain(|arg| !arg.ends_with(".rlib"));
                        for rlib in compiler_rlibs.iter().rev() {
                            args.insert(last_object + 3, rlib.display().to_string());
                        }
                    }
                    LinkerFlavor::Darwin => {
                        args.insert(last_object, "-Wl,-force_load".to_string());
                        args.insert(last_object + 1, out_ar_path.display().to_string());
                        args.retain(|arg| !arg.ends_with(".rlib"));
                        for rlib in compiler_rlibs.iter().rev() {
                            args.insert(last_object + 2, rlib.display().to_string());
                        }
                    }
                    LinkerFlavor::Msvc => {
                        args.insert(
                            last_object,
                            format!("/WHOLEARCHIVE:{}", out_ar_path.display()),
                        );
                        args.retain(|arg| !arg.ends_with(".rlib"));
                        for rlib in compiler_rlibs.iter().rev() {
                            args.insert(last_object + 1, rlib.display().to_string());
                        }
                    }
                    LinkerFlavor::Unsupported => {
                        tracing::error!("Unsupported platform for fat linking: {}", self.triple);
                    }
                };
            }
        }

        // Add custom args to the linkers
        match self.linker_flavor() {
            LinkerFlavor::Gnu => {
                // Export `main` so subsecond can use it for a reference point
                args.push("-Wl,--export-dynamic-symbol,main".to_string());
            }
            LinkerFlavor::Darwin => {
                args.push("-Wl,-exported_symbol,_main".to_string());
            }
            LinkerFlavor::Msvc => {
                // Prevent alsr from overflowing 32 bits
                args.push("/HIGHENTROPYVA:NO".to_string());

                // Export `main` so subsecond can use it for a reference point
                args.push("/EXPORT:main".to_string());
            }
            LinkerFlavor::WasmLld | LinkerFlavor::Unsupported => {}
        }

        // We also need to remove the `-o` flag since we want the linker output to end up in the
        // rust exe location, not in the deps dir as it normally would.
        if let Some(idx) = args
            .iter()
            .position(|arg| *arg == "-o" || *arg == "--output")
        {
            args.remove(idx + 1);
            args.remove(idx);
        }

        // same but windows support
        if let Some(idx) = args.iter().position(|arg| arg.starts_with("/OUT")) {
            args.remove(idx);
        }

        // We want to go through wasm-ld directly, so we need to remove the -flavor flag
        if let Some(flavor_idx) = args.iter().position(|arg| *arg == "-flavor") {
            args.remove(flavor_idx + 1);
            args.remove(flavor_idx);
        }

        // Set the output file
        match self.triple.operating_system {
            OperatingSystem::Windows => args.push(format!("/OUT:{}", exe.display())),
            _ => args.extend(["-o".to_string(), exe.display().to_string()]),
        }

        // And now we can run the linker with our new args
        let linker = self.select_linker()?;

        tracing::trace!("Fat linking with args: {:?} {:#?}", linker, args);
        tracing::trace!("Fat linking with env:");
        for e in rustc_args.envs.iter() {
            tracing::trace!("  {}={}", e.0, e.1);
        }

        // Handle windows command files
        let mut out_args = args.clone();
        if cfg!(windows) {
            let cmd_contents: String = out_args.iter().map(|f| format!("\"{f}\"")).join(" ");
            std::fs::write(self.windows_command_file(), cmd_contents)
                .context("Failed to write linker command file")?;
            out_args = vec![format!("@{}", self.windows_command_file().display())];
        }

        // Add more search paths for the linker
        let mut command_envs = rustc_args.envs.clone();

        // On linux, we need to set a more complete PATH for the linker to find its libraries
        if cfg!(target_os = "linux") {
            command_envs.push(("PATH".to_string(), std::env::var("PATH").unwrap()));
        }

        // Run the linker directly!
        let res = Command::new(linker)
            .args(out_args)
            .env_clear()
            .envs(command_envs)
            .output()
            .await?;

        if !res.stderr.is_empty() {
            let errs = String::from_utf8_lossy(&res.stderr);
            if !res.status.success() {
                tracing::error!(
                    telemetry = %serde_json::json!({ "event": "hotpatch_fat_binary_generation_failed" }),
                    "Failed to generate fat binary: {}",
                    errs.trim()
                );
            } else {
                tracing::trace!("Warnings during fat linking: {}", errs.trim());
            }
        }

        if !res.stdout.is_empty() {
            let out = String::from_utf8_lossy(&res.stdout);
            tracing::trace!("Output from fat linking: {}", out.trim());
        }

        // Clean up the temps manually
        for f in args.iter().filter(|arg| arg.ends_with(".rcgu.o")) {
            _ = std::fs::remove_file(f);
        }

        // Cache the rlibs list
        _ = std::fs::write(
            &out_rlibs_list,
            compiler_rlibs
                .into_iter()
                .map(|s| s.display().to_string())
                .join("\n"),
        );

        Ok(())
    }

    pub(crate) fn create_jump_table(
        &self,
        patch: &Path,
        cache: &HotpatchModuleCache,
    ) -> Result<JumpTable> {
        use crate::build::patch::{
            create_native_jump_table, create_wasm_jump_table, create_windows_jump_table,
        };

        let root_dir = self.root_dir();
        let base_path = self.base_path();
        let triple = &self.triple;

        // Symbols are stored differently based on the platform, so we need to handle them differently.
        // - Wasm requires the walrus crate and actually modifies the patch file
        // - windows requires the pdb crate and pdb files
        // - nix requires the object crate
        let mut jump_table = match triple.operating_system {
            OperatingSystem::Windows => create_windows_jump_table(patch, cache)?,
            _ if triple.architecture == Architecture::Wasm32 => {
                create_wasm_jump_table(patch, cache)?
            }
            _ => create_native_jump_table(patch, triple, cache)?,
        };

        // root_dir: &Path,
        //     base_path: Option<&str>,
        // Rebase the wasm binary to be relocatable once the jump table is generated
        if triple.architecture == target_lexicon::Architecture::Wasm32 {
            // Make sure we use the dir relative to the public dir, so the web can load it as a proper URL
            //
            // ie we would've shipped `/Users/foo/Projects/dioxus/target/dx/project/debug/web/public/wasm/lib.wasm`
            //    but we want to ship `/wasm/lib.wasm`
            jump_table.lib = PathBuf::from(
                "/".to_string() + base_path.unwrap_or_default().trim_start_matches('/'),
            )
            .join(jump_table.lib.strip_prefix(root_dir).unwrap())
        }

        Ok(jump_table)
    }

    /// Automatically detect the linker flavor based on the target triple and any custom linkers.
    ///
    /// This tries to replicate what rustc does when selecting the linker flavor based on the linker
    /// and triple.
    fn linker_flavor(&self) -> LinkerFlavor {
        if let Some(custom) = self.custom_linker.as_ref() {
            let name = custom.file_name().unwrap().to_ascii_lowercase();
            match name.to_str() {
                Some("lld-link") => return LinkerFlavor::Msvc,
                Some("lld-link.exe") => return LinkerFlavor::Msvc,
                Some("wasm-ld") => return LinkerFlavor::WasmLld,
                Some("ld64.lld") => return LinkerFlavor::Darwin,
                Some("ld.lld") => return LinkerFlavor::Gnu,
                Some("ld.gold") => return LinkerFlavor::Gnu,
                Some("mold") => return LinkerFlavor::Gnu,
                Some("sold") => return LinkerFlavor::Gnu,
                Some("wild") => return LinkerFlavor::Gnu,
                _ => {}
            }
        }

        match self.triple.environment {
            target_lexicon::Environment::Gnu
            | target_lexicon::Environment::Gnuabi64
            | target_lexicon::Environment::Gnueabi
            | target_lexicon::Environment::Gnueabihf
            | target_lexicon::Environment::GnuLlvm => LinkerFlavor::Gnu,
            target_lexicon::Environment::Musl => LinkerFlavor::Gnu,
            target_lexicon::Environment::Android => LinkerFlavor::Gnu,
            target_lexicon::Environment::Msvc => LinkerFlavor::Msvc,
            target_lexicon::Environment::Macabi => LinkerFlavor::Darwin,
            _ => match self.triple.operating_system {
                OperatingSystem::Darwin(_) => LinkerFlavor::Darwin,
                OperatingSystem::IOS(_) => LinkerFlavor::Darwin,
                OperatingSystem::MacOSX(_) => LinkerFlavor::Darwin,
                OperatingSystem::Linux => LinkerFlavor::Gnu,
                OperatingSystem::Windows => LinkerFlavor::Msvc,
                _ => match self.triple.architecture {
                    target_lexicon::Architecture::Wasm32 => LinkerFlavor::WasmLld,
                    target_lexicon::Architecture::Wasm64 => LinkerFlavor::WasmLld,
                    _ => LinkerFlavor::Unsupported,
                },
            },
        }
    }

    /// Select the linker to use for this platform.
    ///
    /// We prefer to use the rust-lld linker when we can since it's usually there.
    /// On macos, we use the system linker since macho files can be a bit finicky.
    ///
    /// This means we basically ignore the linker flavor that the user configured, which could
    /// cause issues with a custom linker setup. In theory, rust translates most flags to the right
    /// linker format.
    fn select_linker(&self) -> Result<PathBuf, Error> {
        if let Some(linker) = self.custom_linker.clone() {
            return Ok(linker);
        }

        let cc = match self.linker_flavor() {
            LinkerFlavor::WasmLld => self.workspace.wasm_ld(),

            // On macOS, we use the system linker since it's usually there.
            // We could also use `lld` here, but it might not be installed by default.
            //
            // Note that this is *clang*, not `lld`.
            LinkerFlavor::Darwin => self.workspace.cc(),

            // On Linux, we use the system linker since it's usually there.
            LinkerFlavor::Gnu => self.workspace.cc(),

            // On windows, instead of trying to find the system linker, we just go with the lld.link
            // that rustup provides. It's faster and more stable then reyling on link.exe in path.
            LinkerFlavor::Msvc => self.workspace.lld_link(),

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
            LinkerFlavor::Unsupported => self.workspace.cc(),
        };

        Ok(cc)
    }

    /// Assemble the `cargo rustc` / `rustc` command
    ///
    /// When building fat/base binaries, we use `cargo rustc`.
    /// When building thin binaries, we use `rustc` directly.
    ///
    /// When processing the output of this command, you need to make sure to handle both cases which
    /// both have different formats (but with json output for both).
    fn build_command(&self, build_mode: &BuildMode) -> Result<Command> {
        match build_mode {
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
            BuildMode::Thin {
                workspace_rustc_args,
                ..
            } => {
                let tip_crate_name = self.tip_crate_name();
                let rustc_args = workspace_rustc_args
                    .get(&tip_crate_name)
                    .context("Missing rustc args for tip crate")?;

                let mut cmd = Command::new("rustc");
                cmd.current_dir(self.workspace_dir());
                cmd.env_clear();
                cmd.args(rustc_args.args[1..].iter());
                cmd.env_remove("RUSTC_WORKSPACE_WRAPPER");
                cmd.env_remove("RUSTC_WRAPPER");
                cmd.env_remove(DX_RUSTC_WRAPPER_ENV_VAR);
                cmd.envs(
                    self.cargo_build_env_vars(build_mode)?
                        .iter()
                        .map(|(k, v)| (k.as_ref(), v)),
                );
                cmd.arg(format!("-Clinker={}", Workspace::path_to_dx()?.display()));

                if self.is_wasm_or_wasi() {
                    cmd.arg("-Crelocation-model=pic");
                }

                cmd.envs(rustc_args.envs.iter().cloned());

                Ok(cmd)
            }

            // For Base and Fat builds, we use a regular cargo setup, but we intercept rustc for
            // workspace member crates to capture their args/envs for hot-patching.
            //
            // We use RUSTC_WORKSPACE_WRAPPER which wraps only workspace member crates, letting us
            // capture per-crate args without interfering with external dependency compilation.
            //
            // We've also had a number of issues with incorrect canonicalization when passing paths
            // through envs on windows, hence the frequent use of dunce::canonicalize.
            _ => {
                let mut cmd = Command::new("cargo");

                let env = self.cargo_build_env_vars(build_mode)?;
                let args = self.cargo_build_arguments(build_mode);

                tracing::trace!("Building with cargo rustc");
                for e in env.iter() {
                    tracing::trace!(": {}={}", e.0, e.1.to_string_lossy());
                }

                for a in args.iter() {
                    tracing::trace!(": {}", a);
                }

                cmd.arg("rustc")
                    .current_dir(self.crate_dir())
                    .arg("--message-format")
                    .arg("json-diagnostic-rendered-ansi")
                    .args(args)
                    .envs(env.iter().map(|(k, v)| (k.as_ref(), v)));

                if matches!(build_mode, BuildMode::Fat | BuildMode::Base { run: true }) {
                    let args_dir = self.rustc_wrapper_args_dir();
                    std::fs::create_dir_all(&args_dir)
                        .context("Failed to create rustc wrapper args directory")?;
                    cmd.env(
                        DX_RUSTC_WRAPPER_ENV_VAR,
                        dunce::canonicalize(&args_dir)
                            .context("Failed to canonicalize rustc wrapper args dir")?
                            .display()
                            .to_string(),
                    );
                    cmd.env(
                        "RUSTC_WORKSPACE_WRAPPER",
                        Workspace::path_to_dx()?.display().to_string(),
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
    #[allow(clippy::vec_init_then_push)]
    pub(crate) fn cargo_build_arguments(&self, build_mode: &BuildMode) -> Vec<String> {
        let mut cargo_args = Vec::with_capacity(4);

        // Set the `--config profile.{profile}.{key}={value}` flags for the profile, filling in adhoc profile
        cargo_args.extend(self.profile_args());

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

        if self.all_features {
            cargo_args.push("--all-features".to_string());
        }

        if !self.features.is_empty() {
            cargo_args.push("--features".to_string());
            cargo_args.push(self.features.join(" "));
        }

        // We *always* set the package since that's discovered from cargo metadata
        cargo_args.push(String::from("-p"));
        cargo_args.push(self.package.clone());

        // Set the executable
        match self.executable_type() {
            TargetKind::Bin => cargo_args.push("--bin".to_string()),
            TargetKind::Lib => cargo_args.push("--lib".to_string()),
            TargetKind::Example => cargo_args.push("--example".to_string()),
            _ => {}
        };
        cargo_args.push(self.executable_name().to_string());

        // Set offline/locked/frozen
        let lock_opts = crate::verbosity_or_default();
        if lock_opts.frozen {
            cargo_args.push("--frozen".to_string());
        }
        if lock_opts.locked {
            cargo_args.push("--locked".to_string());
        }
        if lock_opts.offline {
            cargo_args.push("--offline".to_string());
        }

        // Merge in extra args. Order shouldn't really matter.
        cargo_args.extend(self.extra_cargo_args.clone());
        cargo_args.push("--".to_string());
        cargo_args.extend(self.extra_rustc_args.clone());

        // On windows, we pass /SUBSYSTEM:WINDOWS to prevent a console from appearing
        if matches!(self.bundle, BundleFormat::Windows)
            && !self
                .rustflags
                .flags
                .iter()
                .any(|f| f.starts_with("-Clink-arg=/SUBSYSTEM:"))
        {
            let subsystem = self
                .windows_subsystem
                .clone()
                .unwrap_or_else(|| "WINDOWS".to_string());

            cargo_args.push(format!("-Clink-arg=/SUBSYSTEM:{}", subsystem));
            // We also need to set the entry point to mainCRTStartup to avoid windows looking
            // for a WinMain function
            cargo_args.push("-Clink-arg=/ENTRY:mainCRTStartup".to_string());
        }

        // The bundle splitter needs relocation data to create a call-graph.
        // This will automatically be erased by wasm-opt during the optimization step.
        if self.bundle == BundleFormat::Web && self.wasm_split {
            cargo_args.push("-Clink-args=--emit-relocs".to_string());
        }

        // dx *always* links android and thin builds
        if self.custom_linker.is_some()
            || matches!(build_mode, BuildMode::Thin { .. } | BuildMode::Fat)
        {
            cargo_args.push(format!(
                "-Clinker={}",
                Workspace::path_to_dx().expect("can't find dx").display()
            ));
        }

        // for debuggability, we need to make sure android studio can properly understand our build
        // https://stackoverflow.com/questions/68481401/debugging-a-prebuilt-shared-library-in-android-studio
        if self.bundle == BundleFormat::Android {
            cargo_args.push("-Clink-arg=-Wl,--build-id=sha1".to_string());
        }

        // Handle frameworks/dylibs by setting the rpath
        // This is dependent on the bundle structure - in this case, appimage and appbundle for mac/linux
        // todo: we need to figure out what to do for windows
        match self.triple.operating_system {
            OperatingSystem::Darwin(_) | OperatingSystem::IOS(_) => {
                cargo_args.push("-Clink-arg=-Wl,-rpath,@executable_path/../Frameworks".to_string());
                cargo_args.push("-Clink-arg=-Wl,-rpath,@executable_path".to_string());
            }
            OperatingSystem::Linux => {
                cargo_args.push("-Clink-arg=-Wl,-rpath,$ORIGIN/../lib".to_string());
                cargo_args.push("-Clink-arg=-Wl,-rpath,$ORIGIN".to_string());
            }
            _ => {}
        }

        // Our fancy hot-patching engine needs a lot of customization to work properly.
        //
        // These args are mostly intended to be passed when *fat* linking but are generally fine to
        // pass for both fat and thin linking.
        //
        // We need save-temps and no-dead-strip in both cases though. When we run `cargo rustc` with
        // these args, they will be captured and re-ran for the fast compiles in the future, so whatever
        // we set here will be set for all future hot patches too.
        if matches!(build_mode, BuildMode::Thin { .. } | BuildMode::Fat) {
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
            //
            // macOS/iOS use ld64 but through the `cc` interface.
            // cargo_args.push("-Clink-args=-Wl,-all_load".to_string());
            //
            // Linux and Android fit under this umbrella, both with the same clang-like entrypoint
            // and the gnu-ld interface.
            //
            // cargo_args.push("-Clink-args=-Wl,--whole-archive".to_string());
            //
            // If windows -Wl,--whole-archive is required since it follows gnu-ld convention.
            // There might be other flags on windows - we haven't tested windows thoroughly.
            //
            // cargo_args.push("-Clink-args=-Wl,--whole-archive".to_string());
            // https://learn.microsoft.com/en-us/cpp/build/reference/wholearchive-include-all-library-object-files?view=msvc-170
            //
            // ------------------------------------------------------------
            //
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
            // The tricky one is -Ctarget-cpu=mvp, which prevents rustc from generating externref
            // entries.
            //
            // https://blog.rust-lang.org/2024/09/24/webassembly-targets-change-in-default-target-features/#disabling-on-by-default-webassembly-proposals
            //
            // It's fine that these exist in the base module but not in the patch.
            if matches!(
                self.triple.architecture,
                target_lexicon::Architecture::Wasm32 | target_lexicon::Architecture::Wasm64
            ) || self.triple.operating_system == OperatingSystem::Wasi
            {
                // cargo_args.push("-Ctarget-cpu=mvp".into()); // disabled due to changes in wasm-bindgne
                cargo_args.push("-Clink-arg=--no-gc-sections".into());
                cargo_args.push("-Clink-arg=--growable-table".into());
                cargo_args.push("-Clink-arg=--export-table".into());
                cargo_args.push("-Clink-arg=--export-memory".into());
                cargo_args.push("-Clink-arg=--emit-relocs".into());
                cargo_args.push("-Clink-arg=--export=__stack_pointer".into());
                cargo_args.push("-Clink-arg=--export=__heap_base".into());
                cargo_args.push("-Clink-arg=--export=__data_end".into());
            }
        }

        cargo_args
    }

    pub(crate) fn cargo_build_env_vars(
        &self,
        build_mode: &BuildMode,
    ) -> Result<Vec<(Cow<'static, str>, OsString)>> {
        let mut env_vars = vec![];

        // Make sure to set all the crazy android flags. Cross-compiling is hard, man.
        if self.bundle == BundleFormat::Android {
            env_vars.extend(self.android_env_vars()?);
        };

        // If this is a release build, bake the base path and title into the binary with env vars.
        // todo: should we even be doing this? might be better being a build.rs or something else.
        if self.release {
            if let Some(base_path) = self.trimmed_base_path() {
                env_vars.push((ASSET_ROOT_ENV.into(), base_path.to_string().into()));
            }
            env_vars.push((
                APP_TITLE_ENV.into(),
                self.config.web.app.title.clone().into(),
            ));
            env_vars.push((PRODUCT_NAME_ENV.into(), self.bundled_app_name().into()));
        }

        // Assemble the rustflags by peering into the `.cargo/config.toml` file
        let rust_flags = self.rustflags.clone();

        // seems like this is fixed?
        // // Disable reference types on wasm when using hotpatching
        // // https://blog.rust-lang.org/2024/09/24/webassembly-targets-change-in-default-target-features/#disabling-on-by-default-webassembly-proposals
        // if self.is_wasm_or_wasi() && matches!(build_mode, BuildMode::Thin { .. } | BuildMode::Fat) {
        //     rust_flags.flags.push("-Ctarget-cpu=mvp".to_string());
        // }

        // Set the rust flags for the build if they're not empty.
        if !rust_flags.flags.is_empty() {
            env_vars.push((
                "RUSTFLAGS".into(),
                rust_flags
                    .encode_space_separated()
                    .context("Failed to encode RUSTFLAGS")?
                    .into(),
            ));
        }

        // If we're either zero-linking or using a custom linker, make `dx` itself do the linking.
        if self.custom_linker.is_some()
            || matches!(build_mode, BuildMode::Thin { .. } | BuildMode::Fat)
        {
            LinkAction {
                triple: self.triple.clone(),
                linker: self.custom_linker.clone(),
                link_err_file: dunce::canonicalize(self.link_err_file())?,
                link_args_file: dunce::canonicalize(self.link_args_file())?,
            }
            .write_env_vars(&mut env_vars)?;
        }

        Ok(env_vars)
    }

    /// Set the environment variables required for building on Android.
    ///
    /// This involves setting sysroots, CC, CXX, AR, and other environment variables along with
    /// vars that cc-rs uses for its C/C++ compilation.
    ///
    /// We pulled the environment setup from `cargo ndk` and attempt to mimic its behavior to retain
    /// compatibility with existing crates that work with `cargo ndk`.
    ///
    /// <https://github.com/bbqsrc/cargo-ndk/blob/1d1a6dc70a99b7f95bc71ed07bf893ef37966efc/src/cargo.rs#L97-L102>
    ///
    /// cargo-ndk is MIT licensed.
    ///
    /// <https://github.com/bbqsrc/cargo-ndk>
    fn android_env_vars(&self) -> Result<Vec<(Cow<'static, str>, OsString)>> {
        // Derived from getenv_with_target_prefixes in `cc` crate.
        fn cc_env(var_base: &str, triple: &str) -> (String, Option<String>) {
            #[inline]
            fn env_var_with_key(key: String) -> Option<(String, String)> {
                std::env::var(&key).map(|value| (key, value)).ok()
            }

            let triple_u = triple.replace('-', "_");
            let most_specific_key = format!("{}_{}", var_base, triple);

            env_var_with_key(most_specific_key.to_string())
                .or_else(|| env_var_with_key(format!("{}_{}", var_base, triple_u)))
                .or_else(|| env_var_with_key(format!("TARGET_{}", var_base)))
                .or_else(|| env_var_with_key(var_base.to_string()))
                .map(|(key, value)| (key, Some(value)))
                .unwrap_or_else(|| (most_specific_key, None))
        }

        fn cargo_env_target_cfg(triple: &str, key: &str) -> String {
            format!("CARGO_TARGET_{}_{}", &triple.replace('-', "_"), key).to_uppercase()
        }

        fn clang_target(rust_target: &str, api_level: u8) -> String {
            let target = match rust_target {
                "arm-linux-androideabi" => "armv7a-linux-androideabi",
                "armv7-linux-androideabi" => "armv7a-linux-androideabi",
                _ => rust_target,
            };
            format!("--target={target}{api_level}")
        }

        fn sysroot_target(rust_target: &str) -> &str {
            (match rust_target {
                "armv7-linux-androideabi" => "arm-linux-androideabi",
                _ => rust_target,
            }) as _
        }
        fn rt_builtins(rust_target: &str) -> &str {
            (match rust_target {
                "armv7-linux-androideabi" => "arm",
                "aarch64-linux-android" => "aarch64",
                "i686-linux-android" => "i686",
                "x86_64-linux-android" => "x86_64",
                _ => rust_target,
            }) as _
        }

        let mut env_vars: Vec<(Cow<'static, str>, OsString)> = vec![];

        let min_sdk_version = self.min_sdk_version_or_default();

        let tools = self.workspace.android_tools()?;
        let linker = tools.android_cc(&self.triple, min_sdk_version);
        let ar_path = tools.ar_path();
        let target_cc = tools.target_cc();
        let target_cxx = tools.target_cxx();
        let java_home = tools.java_home();
        let ndk_home = tools.ndk.clone();
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

        if let Some(java_home) = java_home {
            tracing::debug!("Setting JAVA_HOME to {java_home:?}");
            env_vars.push(("JAVA_HOME".into(), java_home.into_os_string()));
        }

        let triple = self.triple.to_string();

        // Environment variables for the `cc` crate
        let (cc_key, _cc_value) = cc_env("CC", &triple);
        let (cflags_key, cflags_value) = cc_env("CFLAGS", &triple);
        let (cxx_key, _cxx_value) = cc_env("CXX", &triple);
        let (cxxflags_key, cxxflags_value) = cc_env("CXXFLAGS", &triple);
        let (ar_key, _ar_value) = cc_env("AR", &triple);
        let (ranlib_key, _ranlib_value) = cc_env("RANLIB", &triple);

        // Environment variables for cargo
        let cargo_ar_key = cargo_env_target_cfg(&triple, "ar");
        let cargo_rust_flags_key = cargo_env_target_cfg(&triple, "rustflags");
        let bindgen_clang_args_key =
            format!("BINDGEN_EXTRA_CLANG_ARGS_{}", &triple.replace('-', "_"));

        let clang_target = clang_target(&self.triple.to_string(), min_sdk_version as _);
        let target_cc = tools.target_cc();
        let target_cflags = match cflags_value {
            Some(v) => format!("{clang_target} {v}"),
            None => clang_target.to_string(),
        };
        let target_cxx = tools.target_cxx();
        let target_cxxflags = match cxxflags_value {
            Some(v) => format!("{clang_target} {v}"),
            None => clang_target.to_string(),
        };
        let cargo_ndk_sysroot_path_key = "CARGO_NDK_SYSROOT_PATH";
        let cargo_ndk_sysroot_path = tools.sysroot();
        let cargo_ndk_sysroot_target_key = "CARGO_NDK_SYSROOT_TARGET";
        let cargo_ndk_sysroot_target = sysroot_target(&triple);
        let cargo_ndk_sysroot_libs_path_key = "CARGO_NDK_SYSROOT_LIBS_PATH";
        let cargo_ndk_sysroot_libs_path = cargo_ndk_sysroot_path
            .join("usr")
            .join("lib")
            .join(cargo_ndk_sysroot_target);
        let target_ar = tools.ar_path();
        let target_ranlib = tools.ranlib();
        let clang_folder = tools.clang_folder();

        // choose the clang target with the highest version
        // Should we filter for only numbers?
        let clang_rt = std::fs::read_dir(&clang_folder)
            .map(|dir| {
                let clang_builtins_target = dir
                    .filter_map(|a| a.ok())
                    .max_by(|a, b| a.file_name().cmp(&b.file_name()))
                    .map(|s| s.path())
                    .unwrap_or_else(|| clang_folder.join("clang"));

                format!(
                    "-L{} -lstatic=clang_rt.builtins-{}-android",
                    clang_builtins_target.join("lib").join("linux").display(),
                    rt_builtins(&triple)
                )
            })
            .unwrap_or_default();

        let extra_include: String = format!(
            "{}/usr/include/{}",
            &cargo_ndk_sysroot_path.display(),
            &cargo_ndk_sysroot_target
        );

        let bindgen_args = format!(
            "--sysroot={} -I{}",
            &cargo_ndk_sysroot_path.display(),
            extra_include
        );

        // Load up the OpenSSL environment variables, using our defaults if not set.
        // if the user specifies `/vendor`, then they get vendored, unless OPENSSL_NO_VENDOR is passed (implicitly...)
        let openssl_lib_dir = std::env::var("OPENSSL_LIB_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| AndroidTools::openssl_lib_dir(&self.triple));
        let openssl_include_dir = std::env::var("OPENSSL_INCLUDE_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| AndroidTools::openssl_include_dir());
        let openssl_libs =
            std::env::var("OPENSSL_LIBS").unwrap_or_else(|_| "ssl:crypto".to_string());

        for env in [
            (cc_key, target_cc.clone().into_os_string()),
            (cflags_key, target_cflags.into()),
            (cxx_key, target_cxx.into_os_string()),
            (cxxflags_key, target_cxxflags.into()),
            (ar_key, target_ar.clone().into()),
            (ranlib_key, target_ranlib.into_os_string()),
            (cargo_ar_key, target_ar.into_os_string()),
            (
                cargo_ndk_sysroot_path_key.to_string(),
                cargo_ndk_sysroot_path.clone().into_os_string(),
            ),
            (
                cargo_ndk_sysroot_libs_path_key.to_string(),
                cargo_ndk_sysroot_libs_path.into_os_string(),
            ),
            (
                cargo_ndk_sysroot_target_key.to_string(),
                cargo_ndk_sysroot_target.into(),
            ),
            (cargo_rust_flags_key, clang_rt.into()),
            (bindgen_clang_args_key, bindgen_args.into()),
            (
                "ANDROID_NATIVE_API_LEVEL".to_string(),
                min_sdk_version.to_string().into(),
            ),
            (
                format!(
                    "CARGO_TARGET_{}_LINKER",
                    self.triple
                        .to_string()
                        .to_ascii_uppercase()
                        .replace("-", "_")
                ),
                linker.into_os_string(),
            ),
            ("ANDROID_NDK_ROOT".to_string(), ndk_home.into_os_string()),
            (
                "OPENSSL_LIB_DIR".to_string(),
                openssl_lib_dir.into_os_string(),
            ),
            (
                "OPENSSL_INCLUDE_DIR".to_string(),
                openssl_include_dir.into_os_string(),
            ),
            ("OPENSSL_LIBS".to_string(), openssl_libs.into()),
            // Set the wry env vars - this is where wry will dump its kotlin files.
            // Their setup is really annoying and requires us to hardcode `dx` to specific versions of tao/wry.
            (
                "WRY_ANDROID_PACKAGE".to_string(),
                "dev.dioxus.main".to_string().into(),
            ),
            (
                "WRY_ANDROID_LIBRARY".to_string(),
                "dioxusmain".to_string().into(),
            ),
            (
                "WRY_ANDROID_KOTLIN_FILES_OUT_DIR".to_string(),
                self.wry_android_kotlin_files_out_dir().into_os_string(),
            ),
            // Found this through a comment related to bindgen using the wrong clang for cross compiles
            //
            // https://github.com/rust-lang/rust-bindgen/issues/2962#issuecomment-2438297124
            //
            // https://github.com/KyleMayes/clang-sys?tab=readme-ov-file#environment-variables
            ("CLANG_PATH".into(), target_cc.with_extension("exe").into()),
        ] {
            env_vars.push((env.0.into(), env.1));
        }

        if std::env::var("MSYSTEM").is_ok() || std::env::var("CYGWIN").is_ok() {
            for var in env_vars.iter_mut() {
                // Convert windows paths to unix-style paths
                // This is a workaround for the fact that the `cc` crate expects unix-style paths
                // and will fail if it encounters windows-style paths.
                var.1 = var.1.to_string_lossy().replace('\\', "/").into();
            }
        }

        Ok(env_vars)
    }

    /// Get an estimate of the number of units in the crate. If nightly rustc is not available, this
    /// will return an estimate of the number of units in the crate based on cargo metadata.
    ///
    /// TODO: always use <https://doc.rust-lang.org/nightly/cargo/reference/unstable.html#unit-graph> once it is stable
    async fn get_unit_count_estimate(&self, build_mode: &BuildMode) -> usize {
        // Try to get it from nightly
        if let Ok(count) = self.get_unit_count(build_mode).await {
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
    async fn get_unit_count(&self, build_mode: &BuildMode) -> crate::Result<usize> {
        #[derive(Debug, Deserialize)]
        struct UnitGraph {
            units: Vec<serde_json::Value>,
        }

        let output = tokio::process::Command::new("cargo")
            .arg("+nightly")
            .arg("rustc")
            .arg("--unit-graph")
            .arg("-Z")
            .arg("unstable-options")
            .args(self.cargo_build_arguments(build_mode))
            .envs(
                self.cargo_build_env_vars(build_mode)?
                    .iter()
                    .map(|(k, v)| (k.as_ref(), v)),
            )
            .output()
            .await?;

        if !output.status.success() {
            tracing::trace!(
                "Failed to get unit count: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            bail!("Failed to get unit count");
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

        match self.bundle {
            BundleFormat::Web => platform_dir.join("public"),
            BundleFormat::Server => platform_dir.clone(), // ends up *next* to the public folder

            // These might not actually need to be called `.app` but it does let us run these with `open`
            BundleFormat::MacOS => platform_dir.join(format!("{}.app", self.bundled_app_name())),
            BundleFormat::Ios => platform_dir.join(format!("{}.app", self.bundled_app_name())),

            // in theory, these all could end up directly in the root dir
            BundleFormat::Android => platform_dir.join("app"), // .apk (after bundling)
            BundleFormat::Linux => platform_dir.join("app"),   // .appimage (after bundling)
            BundleFormat::Windows => platform_dir.join("app"), // .exe (after bundling)
        }
    }

    /// Create a workdir for the given platform
    /// This can be used as a temporary directory for the build, but in an observable way such that
    /// you can see the files in the directory via `target`
    ///
    /// target/dx/build/app/web/
    /// target/dx/build/app/web/public/
    /// target/dx/build/app/web/server.exe
    fn platform_dir(&self) -> PathBuf {
        self.internal_out_dir()
            .join(&self.main_target)
            .join(if self.release { "release" } else { "debug" })
            .join(self.bundle.build_folder_name())
    }

    fn platform_exe_name(&self) -> String {
        match self.bundle {
            // mac/ios are unixy and dont have an exe extension
            BundleFormat::MacOS | BundleFormat::Ios => self.executable_name().to_string(),

            // "server" and windows can be the same
            BundleFormat::Server | BundleFormat::Windows => match self.triple.operating_system {
                OperatingSystem::Windows => format!("{}.exe", self.executable_name()),
                _ => self.executable_name().to_string(),
            },

            // from the apk spec, the root exe is a shared library
            // we include the user's rust code as a shared library with a fixed namespace
            BundleFormat::Android => "libdioxusmain.so".to_string(),

            // this will be wrong, I think, but not important?
            BundleFormat::Web => format!("{}_bg.wasm", self.executable_name()),

            // todo: maybe this should be called AppRun?
            BundleFormat::Linux => self.executable_name().to_string(),
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
            android_bundle: Option<crate::AndroidSettings>,
        }
        let hbs_data = AndroidHandlebarsObjects {
            application_id: self.bundle_identifier(),
            app_name: self.bundled_app_name(),
            android_bundle: self.config.bundle.android.clone(),
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

        let manifest_xml = match self.config.application.android_manifest.as_deref() {
            Some(manifest) => std::fs::read_to_string(self.package_manifest_dir().join(manifest))
                .context("Failed to locate custom AndroidManifest.xml")?,
            _ => hbs.render_template(
                include_str!("../../assets/android/gen/app/src/main/AndroidManifest.xml.hbs"),
                &hbs_data,
            )?,
        };

        write(
            app.join("src").join("main").join("AndroidManifest.xml"),
            manifest_xml,
        )?;

        // Write the main activity manually since tao dropped support for it
        let main_activity = match self.config.application.android_main_activity.as_deref() {
            Some(activity) => std::fs::read_to_string(self.package_manifest_dir().join(activity))
                .context("Failed to locate custom MainActivity.kt")?,
            _ => hbs.render_template(
                include_str!("../../assets/android/MainActivity.kt.hbs"),
                &hbs_data,
            )?,
        };
        write(
            self.wry_android_kotlin_files_out_dir()
                .join("MainActivity.kt"),
            main_activity,
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

        create_dir_all(res.join("xml"))?;
        write(
            res.join("xml").join("network_security_config.xml"),
            include_bytes!(
                "../../assets/android/gen/app/src/main/res/xml/network_security_config.xml"
            ),
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

    /// Get the directory where this app can write to for this session that's guaranteed to be stable
    /// for the same app. This is useful for emitting state like window position and size.
    ///
    /// The directory is specific for this app and might be
    pub(crate) fn session_cache_dir(&self) -> PathBuf {
        self.session_cache_dir.join(self.bundle.to_string())
    }

    pub(crate) fn rustc_wrapper_args_dir(&self) -> PathBuf {
        self.session_cache_dir().join("rustc_wrapper_args")
    }

    /// The crate name that rustc uses for the tip crate (hyphens replaced with underscores).
    fn tip_crate_name(&self) -> String {
        self.main_target.replace('-', "_")
    }

    fn link_err_file(&self) -> PathBuf {
        self.session_cache_dir().join("link_err.txt")
    }

    fn link_args_file(&self) -> PathBuf {
        self.session_cache_dir().join("link_args.json")
    }

    fn windows_command_file(&self) -> PathBuf {
        self.session_cache_dir().join("windows_command.txt")
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
        let dir = self.target_dir.join("dx");
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// target/dx/bundle/app/
    /// target/dx/bundle/app/blah.app
    /// target/dx/bundle/app/blah.exe
    /// target/dx/bundle/app/public/
    pub(crate) fn bundle_dir(&self, bundle: BundleFormat) -> PathBuf {
        self.internal_out_dir()
            .join(&self.main_target)
            .join("bundle")
            .join(bundle.build_folder_name())
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

    /// Get the features required to build for the given platform
    fn feature_for_platform_and_renderer(
        package: &krates::cm::Package,
        triple: &Triple,
        renderer: Renderer,
    ) -> Option<String> {
        // Try to find the feature that activates the dioxus feature for the given platform
        let dioxus_feature = renderer.feature_name(triple);

        let res = package.features.iter().find_map(|(key, features)| {
            // if the feature is just the name of the platform, we use that
            if key == dioxus_feature {
                tracing::debug!("Found feature {key} for renderer {renderer}");
                return Some(key.clone());
            }

            // Otherwise look for the feature that starts with dioxus/ or dioxus?/ and matches just the single platform
            // we are looking for.
            let mut dioxus_renderers_enabled = Vec::new();
            for feature in features {
                if let Some((_, after_dioxus)) = feature.split_once("dioxus") {
                    if let Some(dioxus_feature_enabled) =
                        after_dioxus.trim_start_matches('?').strip_prefix('/')
                    {
                        if Renderer::autodetect_from_cargo_feature(dioxus_feature_enabled).is_some()
                        {
                            dioxus_renderers_enabled.push(dioxus_feature_enabled.to_string());
                        }
                    }
                }
            }

            // If there is exactly one renderer enabled by this feature, we can use it
            if let [feature_name] = dioxus_renderers_enabled.as_slice() {
                if feature_name == dioxus_feature {
                    tracing::debug!(
                        "Found feature {key} for renderer {renderer} which enables dioxus/{renderer}"
                    );
                    return Some(key.clone());
                }
            }

            None
        });

        res.or_else(|| {
            let depends_on_dioxus = package.dependencies.iter().any(|dep| dep.name == "dioxus");
            if depends_on_dioxus {
                let fallback = format!("dioxus/{dioxus_feature}");
                tracing::debug!(
                    "Could not find explicit feature for renderer {renderer}, passing `fallback` instead"
                );
                Some(fallback)
            } else {
                None
            }
        })
    }

    /// Checks the strip setting for the package, resolving profiles recursively
    pub(crate) fn get_strip_setting(&self) -> StripSetting {
        let cargo_toml = &self.workspace.cargo_toml;
        let profile = &self.profile;
        let release = self.release;
        let profile = match (cargo_toml.profile.custom.get(profile), release) {
            (Some(custom_profile), _) => Some(custom_profile),
            (_, true) => cargo_toml.profile.release.as_ref(),
            (_, false) => cargo_toml.profile.dev.as_ref(),
        };

        let Some(profile) = profile else {
            return StripSetting::None;
        };

        // Get the strip setting from the profile or the profile it inherits from
        fn get_strip(profile: &Profile, profiles: &Profiles) -> Option<StripSetting> {
            profile.strip.or_else(|| {
                // If we can't find the strip setting, check if we inherit from another profile
                profile.inherits.as_ref().and_then(|inherits| {
                    let profile = match inherits.as_str() {
                        "dev" => profiles.dev.as_ref(),
                        "release" => profiles.release.as_ref(),
                        "test" => profiles.test.as_ref(),
                        "bench" => profiles.bench.as_ref(),
                        other => profiles.custom.get(other),
                    };
                    profile.and_then(|p| get_strip(p, profiles))
                })
            })
        }

        let Some(strip) = get_strip(profile, &cargo_toml.profile) else {
            // If the profile doesn't have a strip option, return None
            return StripSetting::None;
        };

        strip
    }

    pub(crate) fn renderer_enabled_by_dioxus_dependency(
        package: &krates::cm::Package,
    ) -> Option<(Renderer, String)> {
        let mut renderers = vec![];

        // Attempt to discover the platform directly from the dioxus dependency
        //
        // [dependencies]
        // dioxus = { features = ["web"] }
        //
        if let Some(dxs) = package.dependencies.iter().find(|dep| dep.name == "dioxus") {
            for feature in dxs.features.iter() {
                if let Some(renderer) = Renderer::autodetect_from_cargo_feature(feature) {
                    renderers.push((renderer, format!("dioxus/{}", feature)));
                }
            }
        }

        if renderers.len() != 1 {
            return None;
        }

        Some(renderers[0].clone())
    }

    pub(crate) fn features_that_enable_renderers(
        package: &krates::cm::Package,
    ) -> Vec<(Renderer, String)> {
        package
            .features
            .keys()
            .filter_map(|key| {
                Renderer::autodetect_from_cargo_feature(key).map(|v| (v, key.to_string()))
            })
            .collect()
    }

    /// Return the platforms that are enabled for the package only from the default features
    ///
    /// Ideally only one platform is enabled but we need to be able to
    pub(crate) fn enabled_cargo_toml_default_features_renderers(
        package: &krates::cm::Package,
    ) -> Vec<(Renderer, String)> {
        let mut renderers = vec![];

        // Start searching through the default features
        //
        // [features]
        // default = ["dioxus/web"]
        //
        // or
        //
        // [features]
        // default = ["web"]
        // web = ["dioxus/web"]
        let Some(default) = package.features.get("default") else {
            return renderers;
        };

        // we only trace features 1 level deep..
        // TODO: trace all enabled features, not just default features
        for feature in default.iter() {
            // If the user directly specified a platform we can just use that.
            if feature.starts_with("dioxus/") {
                let dx_feature = feature.trim_start_matches("dioxus/");
                let auto = Renderer::autodetect_from_cargo_feature(dx_feature);
                if let Some(auto) = auto {
                    renderers.push((auto, dx_feature.to_string()));
                }
            }

            // If the user is specifying an internal feature that points to a platform, we can use that
            let internal_feature = package.features.get(feature);
            if let Some(internal_feature) = internal_feature {
                for feature in internal_feature {
                    if feature.starts_with("dioxus/") {
                        let dx_feature = feature.trim_start_matches("dioxus/");
                        let auto = Renderer::autodetect_from_cargo_feature(dx_feature);
                        if let Some(auto) = auto {
                            renderers.push((auto, dx_feature.to_string()));
                        }
                    }
                }
            }
        }

        renderers.sort();
        renderers.dedup();

        renderers
    }

    /// Gather the features that are enabled for the package
    fn rendererless_features(package: &krates::cm::Package) -> Vec<String> {
        let Some(default) = package.features.get("default") else {
            return Vec::new();
        };

        let mut kept_features = vec![];

        // Only keep the top-level features in the default list that don't point to a platform directly
        // IE we want to drop `web` if default = ["web"]
        'top: for feature in default {
            // Don't keep features that point to a platform via dioxus/blah
            if feature.starts_with("dioxus/") {
                let dx_feature = feature.trim_start_matches("dioxus/");
                if Renderer::autodetect_from_cargo_feature(dx_feature).is_some() {
                    tracing::debug!(
                        "Dropping feature {feature} since it points to a platform renderer"
                    );
                    continue 'top;
                }
            }

            // Don't keep features that point to a platform via an internal feature
            if let Some(internal_feature) = package.features.get(feature) {
                for feature in internal_feature {
                    if feature.starts_with("dioxus/") {
                        let dx_feature = feature.trim_start_matches("dioxus/");
                        if Renderer::autodetect_from_cargo_feature(dx_feature).is_some() {
                            tracing::debug!(
                                "Dropping feature {feature} since it points to a platform renderer transitively"
                            );
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

    pub(crate) fn bundled_app_name(&self) -> String {
        use convert_case::{Case, Casing};
        self.executable_name().to_case(Case::Pascal)
    }

    pub(crate) fn bundle_identifier(&self) -> String {
        if let Some(identifier) = &self.config.bundle.identifier {
            if identifier.contains('.')
                && !identifier.starts_with('.')
                && !identifier.ends_with('.')
                && !identifier.contains("..")
            {
                return identifier.clone();
            } else {
                // The original `mobile_org` function used `expect` directly.
                // Maybe it's acceptable for the CLI to panic directly when this error occurs.
                // And if we change it to a Result type, the `client_connected` function in serve/runner.rs does not return a Result and cannot call `?`,
                // We also need to handle the error in place, otherwise it will expand the scope of modifications further.
                panic!("Invalid bundle identifier: {identifier:?}. E.g. `com.example`, `com.example.app`");
            }
        }

        format!("com.example.{}", self.bundled_app_name())
    }

    /// The item that we'll try to run directly if we need to.
    ///
    /// todo(jon): we should name the app properly instead of making up the exe name. It's kinda okay for dev mode, but def not okay for prod
    pub(crate) fn main_exe(&self) -> PathBuf {
        self.exe_dir().join(self.platform_exe_name())
    }

    fn is_wasm_or_wasi(&self) -> bool {
        matches!(
            self.triple.architecture,
            target_lexicon::Architecture::Wasm32 | target_lexicon::Architecture::Wasm64
        ) || self.triple.operating_system == target_lexicon::OperatingSystem::Wasi
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

        // Check if any of the features in our feature list enables a feature that enables "fullstack"
        let transitive = self
            .package()
            .features
            .iter()
            .filter(|(_name, list)| list.iter().any(|f| f == "dioxus/fullstack"));

        for (name, _list) in transitive {
            if self.features.contains(name) {
                return true;
            }
        }

        false
    }

    /// todo(jon): use handlebars templates instead of these prebaked templates
    async fn write_metadata(&self) -> Result<()> {
        // write the Info.plist file
        match self.bundle {
            BundleFormat::MacOS => {
                let dest = self.root_dir().join("Contents").join("Info.plist");
                let plist = self.info_plist_contents(self.bundle)?;
                std::fs::write(dest, plist)?;
            }

            BundleFormat::Ios => {
                let dest = self.root_dir().join("Info.plist");
                let plist = self.info_plist_contents(self.bundle)?;
                std::fs::write(dest, plist)?;
            }

            // AndroidManifest.xml
            // er.... maybe even all the kotlin/java/gradle stuff?
            BundleFormat::Android => {}

            // Probably some custom format or a plist file (haha)
            // When we do the proper bundle, we'll need to do something with wix templates, I think?
            BundleFormat::Windows => {}

            // eventually we'll create the .appimage file, I guess?
            BundleFormat::Linux => {}

            // These are served as folders, not appimages, so we don't need to do anything special (I think?)
            // Eventually maybe write some secrets/.env files for the server?
            // We could also distribute them as a deb/rpm for linux and msi for windows
            BundleFormat::Web => {}
            BundleFormat::Server => {}
        }

        Ok(())
    }

    /// Run the optimizers, obfuscators, minimizers, signers, etc
    async fn optimize(&self, ctx: &BuildContext) -> Result<()> {
        match self.bundle {
            BundleFormat::Web => {
                // Compress the asset dir
                // If pre-compressing is enabled, we can pre_compress the wasm-bindgen output
                let pre_compress = self.should_pre_compress_web_assets(self.release);

                if pre_compress {
                    ctx.status_compressing_assets();
                    let asset_dir = self.asset_dir();
                    tokio::task::spawn_blocking(move || {
                        crate::fastfs::pre_compress_folder(&asset_dir, pre_compress)
                    })
                    .await
                    .unwrap()?;
                }
            }
            BundleFormat::MacOS
            | BundleFormat::Windows
            | BundleFormat::Linux
            | BundleFormat::Ios
            | BundleFormat::Android
            | BundleFormat::Server => {}
        }

        Ok(())
    }

    /// Strip the final binary after extracting all assets with rustc-objcopy
    async fn strip_binary(&self, artifacts: &BuildArtifacts) -> Result<()> {
        // Never strip the binary if we are going to bundle split it
        if self.wasm_split {
            return Ok(());
        }
        let exe = &artifacts.exe;
        // https://github.com/rust-lang/rust/blob/cb80ff132a0e9aa71529b701427e4e6c243b58df/compiler/rustc_codegen_ssa/src/back/linker.rs#L1433-L1443
        let strip_arg = match self.get_strip_setting() {
            StripSetting::Debuginfo => Some("--strip-debug"),
            StripSetting::Symbols => Some("--strip-all"),
            StripSetting::None => None,
        };
        if let Some(strip_arg) = strip_arg {
            let rustc_objcopy = self.workspace.rustc_objcopy();
            let dylib_path = self.workspace.rustc_objcopy_dylib_path();
            let mut command = Command::new(rustc_objcopy);
            command.env("LD_LIBRARY_PATH", &dylib_path);
            command.arg(strip_arg).arg(exe).arg(exe);
            let output = command.output().await?;
            if !output.status.success() {
                if let Ok(stdout) = std::str::from_utf8(&output.stdout) {
                    tracing::error!("{}", stdout);
                }
                if let Ok(stderr) = std::str::from_utf8(&output.stderr) {
                    tracing::error!("{}", stderr);
                }
                return Err(anyhow::anyhow!("Failed to strip binary"));
            }
        }
        Ok(())
    }

    /// Check if assets should be pre_compressed. This will only be true in release mode if the user
    /// has enabled pre_compress in the web config.
    fn should_pre_compress_web_assets(&self, release: bool) -> bool {
        self.config.web.pre_compress & release
    }

    /// Check if the wasm output should be bundled to an asset type app.
    fn should_bundle_to_asset(&self) -> bool {
        self.release && !self.wasm_split && self.bundle == BundleFormat::Web
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
        let post_bindgen_wasm = self.wasm_bindgen_wasm_output_file();
        let should_bundle_split: bool = self.wasm_split;
        let bindgen_version = self
            .workspace
            .wasm_bindgen_version()
            .expect("this should have been checked by tool verification");

        // Prepare any work dirs
        _ = std::fs::remove_dir_all(&bindgen_outdir);
        std::fs::create_dir_all(&bindgen_outdir)?;

        // Lift the internal functions to exports
        if ctx.mode == BuildMode::Fat {
            let unprocessed = std::fs::read(exe)?;
            let all_exported_bytes = crate::build::prepare_wasm_base_module(&unprocessed)?;
            std::fs::write(exe, all_exported_bytes)?;
        }

        // Prepare our configuration
        //
        // we turn on debug symbols in dev mode
        //
        // We leave demangling to false since it's faster and these tools seem to prefer the raw symbols.
        // todo(jon): investigate if the chrome extension needs them demangled or demangles them automatically.
        let keep_debug = self.config.web.wasm_opt.debug
            || self.debug_symbols
            || self.wasm_split
            || !self.release
            || ctx.mode == BuildMode::Fat;
        let keep_names = self.wasm_split || ctx.mode == BuildMode::Fat;
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
            .input_path(exe)
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

            // Load the contents of these binaries since we need both of them
            // We're going to use the default makeLoad glue from wasm-split
            let original = std::fs::read(exe)?;
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
                    glue, "export const __wasm_split_load_chunk_{idx} = makeLoad(\"/{base_path}/assets/{url}\", [], fusedImports);",
                    base_path = self.base_path_or_default(),
                    url = assets
                        .register_asset(&path, AssetOptions::builder().into_asset_options())?.bundled_path(),
                )?;
            }

            // Write the modules that contain the entrypoints
            tracing::debug!("Writing split modules to disk");
            for (idx, module) in modules.modules.iter().enumerate() {
                let comp_name = module
                    .component_name
                    .as_ref()
                    .context("generated bindgen module has no name?")?;

                let path = bindgen_outdir.join(format!("module_{idx}_{comp_name}.wasm"));
                wasm_opt::write_wasm(&module.bytes, &path, &wasm_opt_options).await?;

                let hash_id = module
                    .hash_id
                    .as_ref()
                    .context("generated wasm-split bindgen module has no hash id?")?;

                writeln!(
                    glue,
                    "export const __wasm_split_load_{module}_{hash_id}_{comp_name} = makeLoad(\"/{base_path}/assets/{url}\", [{deps}], fusedImports);",
                    module = module.module_name,

                    base_path = self.base_path_or_default(),

                    // Again, register this wasm with the asset system
                    url = assets
                        .register_asset(&path, AssetOptions::builder().into_asset_options())?
                        .bundled_path(),

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
            let uuid = Uuid::new_v5(&Uuid::NAMESPACE_URL, glue.as_bytes());
            std::fs::OpenOptions::new()
                .append(true)
                .open(self.wasm_bindgen_js_output_file())
                .context("Failed to open main.js file")?
                .write_all(format!("/*{uuid}*/").as_bytes())?;

            // Write the main wasm_bindgen file and register it with the asset system
            // This will overwrite the file in place
            // We will wasm-opt it in just a second...
            std::fs::write(&post_bindgen_wasm, modules.main.bytes).unwrap();
        }

        if matches!(ctx.mode, BuildMode::Fat) {
            // add `export { __wbg_get_imports };` to the end of the wasmbindgen js file
            let mut js = std::fs::read(self.wasm_bindgen_js_output_file())?;
            writeln!(js, "\nexport {{ __wbg_get_imports }};")?;
            std::fs::write(self.wasm_bindgen_js_output_file(), js)?;
        }

        // Make sure to optimize the main wasm file if requested or if bundle splitting
        if should_bundle_split || self.release {
            ctx.status_optimizing_wasm();
            wasm_opt::optimize(&post_bindgen_wasm, &post_bindgen_wasm, &wasm_opt_options).await?;
        }

        if self.should_bundle_to_asset() {
            // Make sure to register the main wasm file with the asset system
            assets.register_asset(
                &post_bindgen_wasm,
                AssetOptions::builder().into_asset_options(),
            )?;
        }

        // Now that the wasm is registered as an asset, we can write the js glue shim
        self.write_js_glue_shim(assets)?;

        if self.should_bundle_to_asset() {
            // Register the main.js with the asset system so it bundles in the snippets and optimizes
            assets.register_asset(
                &self.wasm_bindgen_js_output_file(),
                AssetOptions::js()
                    .with_minify(true)
                    .with_preload(true)
                    .into_asset_options(),
            )?;
        }

        // Write the index.html file with the pre-configured contents we got from pre-rendering
        self.write_index_html(assets)?;

        Ok(())
    }

    fn write_js_glue_shim(&self, assets: &AssetManifest) -> Result<()> {
        let wasm_path = self.bundled_wasm_path(assets);

        // Load and initialize wasm without requiring a separate javascript file.
        // This also allows using a strict Content-Security-Policy.
        let mut js = std::fs::OpenOptions::new()
            .append(true)
            .open(self.wasm_bindgen_js_output_file())?;
        let mut buf_writer = std::io::BufWriter::new(&mut js);
        writeln!(
            buf_writer,
            r#"
globalThis.__wasm_split_main_initSync = initSync;

// Actually perform the load
__wbg_init({{module_or_path: "/{}/{wasm_path}"}}).then((wasm) => {{
    // assign this module to be accessible globally
    globalThis.__dx_mainWasm = wasm;
    globalThis.__dx_mainInit = __wbg_init;
    globalThis.__dx_mainInitSync = initSync;
    globalThis.__dx___wbg_get_imports = __wbg_get_imports;

    if (wasm.__wbindgen_start == undefined) {{
        wasm.main();
    }}
}});
"#,
            self.base_path_or_default(),
        )?;

        Ok(())
    }

    /// Write the index.html file to the output directory. This must be called after the wasm and js
    /// assets are registered with the asset system if this is a release build.
    pub(crate) fn write_index_html(&self, assets: &AssetManifest) -> Result<()> {
        let wasm_path = self.bundled_wasm_path(assets);
        let js_path = self.bundled_js_path(assets);

        // Write the index.html file with the pre-configured contents we got from pre-rendering
        std::fs::write(
            self.root_dir().join("index.html"),
            self.prepare_html(assets, &wasm_path, &js_path).unwrap(),
        )?;

        Ok(())
    }

    fn bundled_js_path(&self, assets: &AssetManifest) -> String {
        let wasm_bindgen_js_out = self.wasm_bindgen_js_output_file();
        if self.should_bundle_to_asset() {
            let name = assets
                .get_first_asset_for_source(&wasm_bindgen_js_out)
                .expect("The js source must exist before creating index.html");
            format!("assets/{}", name.bundled_path())
        } else {
            format!(
                "wasm/{}",
                wasm_bindgen_js_out.file_name().unwrap().to_str().unwrap()
            )
        }
    }

    /// Get the path to the wasm-bindgen output files. Either the direct file or the optimized one depending on the build mode
    fn bundled_wasm_path(&self, assets: &AssetManifest) -> String {
        let wasm_bindgen_wasm_out = self.wasm_bindgen_wasm_output_file();
        if self.should_bundle_to_asset() {
            let name = assets
                .get_first_asset_for_source(&wasm_bindgen_wasm_out)
                .expect("The wasm source must exist before creating index.html");
            format!("assets/{}", name.bundled_path())
        } else {
            format!(
                "wasm/{}",
                wasm_bindgen_wasm_out.file_name().unwrap().to_str().unwrap()
            )
        }
    }

    fn info_plist_contents(&self, bundle: BundleFormat) -> Result<String> {
        #[derive(Serialize)]
        pub struct InfoPlistData {
            pub display_name: String,
            pub bundle_name: String,
            pub bundle_identifier: String,
            pub executable_name: String,
        }

        // Attempt to use the user's manually specified
        let _app = &self.config.application;
        match bundle {
            BundleFormat::MacOS => {
                if let Some(macos_info_plist) = _app.macos_info_plist.as_deref() {
                    return Ok(std::fs::read_to_string(macos_info_plist)?);
                }
            }
            BundleFormat::Ios => {
                if let Some(macos_info_plist) = _app.ios_info_plist.as_deref() {
                    return Ok(std::fs::read_to_string(macos_info_plist)?);
                }
            }
            _ => {}
        }

        match bundle {
            BundleFormat::MacOS => handlebars::Handlebars::new()
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
            BundleFormat::Ios => handlebars::Handlebars::new()
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
            _ => Err(anyhow::anyhow!("Unsupported platform for Info.plist")),
        }
    }

    /// Run any final tools to produce apks or other artifacts we might need.
    ///
    /// This might include codesigning, zipping, creating an appimage, etc
    async fn assemble(&self, ctx: &BuildContext) -> Result<()> {
        if let BundleFormat::Android = self.bundle {
            ctx.status_running_gradle();

            // When the build mode is set to release and there is an Android signature configuration, use assembleRelease
            let build_type = if self.release && self.config.bundle.android.is_some() {
                "assembleRelease"
            } else {
                "assembleDebug"
            };

            let output = Command::new(self.gradle_exe()?)
                .arg(build_type)
                .current_dir(self.root_dir())
                .output()
                .await
                .context("Failed to run gradle")?;

            if !output.status.success() {
                bail!(
                    "Failed to assemble apk: {}",
                    String::from_utf8_lossy(&output.stderr)
                );
            }
        }

        // if the triple is a ios or macos target, we need to codesign the binary
        if matches!(
            self.triple.operating_system,
            OperatingSystem::Darwin(_) | OperatingSystem::IOS(_)
        ) && self.should_codesign
        {
            self.codesign_apple(ctx).await?;
        }

        Ok(())
    }

    /// Run bundleRelease and return the path to the `.aab` file
    ///
    /// <https://stackoverflow.com/questions/57072558/whats-the-difference-between-gradlewassemblerelease-gradlewinstallrelease-and>
    pub(crate) async fn android_gradle_bundle(&self) -> Result<PathBuf> {
        let output = Command::new(self.gradle_exe()?)
            .arg("bundleRelease")
            .current_dir(self.root_dir())
            .output()
            .await
            .context("Failed to run gradle bundleRelease")?;

        if !output.status.success() {
            bail!(
                "Failed to bundleRelease: {}",
                String::from_utf8_lossy(&output.stderr)
            );
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
            )
            .context("Failed to make gradlew executable")?;
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
    pub(crate) fn prepare_build_dir(&self, ctx: &BuildContext) -> Result<()> {
        use std::fs::{create_dir_all, remove_dir_all};
        use std::sync::OnceLock;

        static PRIMARY_INITIALIZED: OnceLock<Result<()>> = OnceLock::new();
        static SECONDARY_INITIALIZED: OnceLock<Result<()>> = OnceLock::new();

        let initializer = if ctx.is_primary_build() {
            &PRIMARY_INITIALIZED
        } else {
            &SECONDARY_INITIALIZED
        };

        let success = initializer.get_or_init(|| {
            if ctx.is_primary_build() {
                _ = remove_dir_all(self.exe_dir());
            }

            create_dir_all(self.root_dir())?;
            create_dir_all(self.exe_dir())?;
            create_dir_all(self.asset_dir())?;

            tracing::debug!(
                r#"Initialized build dirs:
               • root dir: {:?}
               • exe dir: {:?}
               • asset dir: {:?}"#,
                self.root_dir(),
                self.exe_dir(),
                self.asset_dir(),
            );

            // we could download the templates from somewhere (github?) but after having banged my head against
            // cargo-mobile2 for ages, I give up with that. We're literally just going to hardcode the templates
            // by writing them here.
            if self.bundle == BundleFormat::Android {
                self.build_android_app_dir()?;
            }

            Ok(())
        });

        if let Err(e) = success.as_ref() {
            bail!("Failed to initialize build directory: {e}");
        }

        Ok(())
    }

    pub(crate) fn asset_dir(&self) -> PathBuf {
        match self.bundle {
            BundleFormat::MacOS => self
                .root_dir()
                .join("Contents")
                .join("Resources")
                .join("assets"),

            BundleFormat::Android => self
                .root_dir()
                .join("app")
                .join("src")
                .join("main")
                .join("assets"),

            // We put assets in public/assets for server apps
            BundleFormat::Server => self.root_dir().join("public").join("assets"),

            // everyone else is soooo normal, just app/assets :)
            BundleFormat::Web | BundleFormat::Ios | BundleFormat::Windows | BundleFormat::Linux => {
                self.root_dir().join("assets")
            }
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
        match self.bundle {
            BundleFormat::MacOS => self.root_dir().join("Contents").join("MacOS"),
            BundleFormat::Web => self.root_dir().join("wasm"),

            // Android has a whole build structure to it
            BundleFormat::Android => self
                .root_dir()
                .join("app")
                .join("src")
                .join("main")
                .join("jniLibs")
                .join(AndroidTools::android_jnilib(&self.triple)),

            // these are all the same, I think?
            BundleFormat::Windows
            | BundleFormat::Linux
            | BundleFormat::Ios
            | BundleFormat::Server => self.root_dir(),
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

    /// Get the path to the app manifest file
    ///
    /// This includes metadata about the build such as the bundle format, target triple, features, etc.
    /// Manifests are only written by the `PRIMARY` build.
    pub(crate) fn app_manifest(&self) -> PathBuf {
        self.platform_dir().join(".manifest.json")
    }

    pub(crate) fn load_manifest(&self) -> Result<AppManifest> {
        let manifest_path = self.app_manifest();
        let manifest_data = std::fs::read_to_string(&manifest_path)
            .with_context(|| format!("Failed to read manifest at {:?}", &manifest_path))?;
        let manifest: AppManifest = serde_json::from_str(&manifest_data)
            .with_context(|| format!("Failed to parse manifest at {:?}", &manifest_path))?;
        Ok(manifest)
    }

    /// Check for tooling that might be required for this build.
    ///
    /// This should generally be only called on the first build since it takes time to verify the tooling
    /// is in place, and we don't want to slow down subsequent builds.
    pub(crate) async fn verify_tooling(&self, ctx: &BuildContext) -> Result<()> {
        ctx.status_installing_tooling();

        self.verify_toolchain_installed().await?;

        match self.bundle {
            BundleFormat::Web => self.verify_web_tooling().await?,
            BundleFormat::Ios => self.verify_ios_tooling().await?,
            BundleFormat::Android => self.verify_android_tooling().await?,
            BundleFormat::Linux => self.verify_linux_tooling().await?,
            BundleFormat::MacOS | BundleFormat::Windows | BundleFormat::Server => {}
        }

        Ok(())
    }

    async fn verify_toolchain_installed(&self) -> Result<()> {
        let toolchain_dir = self.workspace.sysroot.join("lib/rustlib");
        let triple = self.triple.to_string();

        // Install target using rustup.
        if !toolchain_dir.join(&triple).exists() {
            tracing::info!(
                "{} platform requires {} to be installed. Installing...",
                self.bundle,
                triple
            );

            let mut child = tokio::process::Command::new("rustup")
                .args(["target", "add"])
                .arg(&triple)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .kill_on_drop(true)
                .spawn()?;

            let stdout = tokio::io::BufReader::new(child.stdout.take().unwrap());
            let stderr = tokio::io::BufReader::new(child.stderr.take().unwrap());
            let mut stdout_lines = stdout.lines();
            let mut stderr_lines = stderr.lines();
            loop {
                tokio::select! {
                    line = stdout_lines.next_line() => {
                        match line {
                            Ok(Some(line)) => tracing::info!("{}", line),
                            Err(err) => tracing::error!("{}", err),
                            Ok(_) => break,
                        }
                    }
                    line = stderr_lines.next_line() => {
                        match line {
                            Ok(Some(line)) => tracing::info!("{}", line),
                            Err(err) => tracing::error!("{}", err),
                            Ok(_) => break,
                        }
                    }
                }
            }
        }

        // Ensure target is installed.
        if !toolchain_dir.join(&triple).exists() {
            bail!("Missing rust target {}", triple);
        }

        Ok(())
    }

    async fn verify_web_tooling(&self) -> Result<()> {
        // Wasm bindgen
        let krate_bindgen_version =
            self.workspace
                .wasm_bindgen_version()
                .ok_or(anyhow::anyhow!(
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
        let linker = self
            .workspace
            .android_tools()?
            .android_cc(&self.triple, self.min_sdk_version_or_default());

        tracing::debug!("Verifying android linker: {linker:?}");

        if linker.exists() {
            return Ok(());
        }

        bail!(
            "Android linker not found at {linker:?}. Please set the `ANDROID_NDK_HOME` environment variable to the root of your NDK installation."
        );
    }

    /// Ensure the right dependencies are installed for linux apps.
    /// This varies by distro, so we just do nothing for now.
    ///
    /// Eventually, we want to check for the prereqs for wry/tao as outlined by tauri:
    ///     <https://tauri.app/start/prerequisites/>
    async fn verify_linux_tooling(&self) -> Result<()> {
        Ok(())
    }

    /// Blow away the fingerprint for this package, forcing rustc to recompile it.
    ///
    /// This prevents rustc from using the cached version of the binary, which can cause issues
    /// Find workspace crates that directly depend on the given crate.
    ///
    /// Returns underscore-normalized crate names of workspace members that have `crate_name`
    /// as a dependency. Used for cascade detection — when a dep's public symbols change,
    /// its dependents need recompilation too.
    fn workspace_dependents_of(&self, crate_name: &str) -> Vec<String> {
        let krates = &self.workspace.krates;

        // Find the NodeId for the target crate
        let target_nid = krates.workspace_members().find_map(|member| {
            if let krates::Node::Krate { id, krate, .. } = member {
                if krate.name.replace('-', "_") == crate_name {
                    return krates.nid_for_kid(id);
                }
            }
            None
        });

        let Some(target_nid) = target_nid else {
            return Vec::new();
        };

        // Use krates' direct_dependents to find reverse deps, filter to workspace members
        let workspace_names: HashSet<String> = krates
            .workspace_members()
            .filter_map(|m| {
                if let krates::Node::Krate { krate, .. } = m {
                    Some(krate.name.replace('-', "_"))
                } else {
                    None
                }
            })
            .collect();

        krates
            .direct_dependents(target_nid)
            .into_iter()
            .filter_map(|dep| {
                let name = dep.krate.name.replace('-', "_");
                if workspace_names.contains(&name) {
                    Some(name)
                } else {
                    None
                }
            })
            .collect()
    }

    /// Compile a workspace dependency crate directly with `rustc` using its captured args.
    ///
    /// This produces an updated rlib at the same path cargo originally wrote to.
    /// Used during thin builds to recompile changed workspace deps before the tip crate.
    async fn compile_dep_crate(&self, crate_name: &str, rustc_args: &RustcArgs) -> Result<()> {
        let mut cmd = Command::new("rustc");
        cmd.current_dir(self.workspace_dir());
        cmd.env_clear();

        // Skip args[0] which is the rustc binary path captured by the wrapper
        cmd.args(rustc_args.args[1..].iter());

        // Restore the captured environment, filtering out wrapper env vars and
        // stale cargo jobserver vars to prevent recursive invocation and warnings.
        let filtered_env_keys = [
            "RUSTC_WORKSPACE_WRAPPER",
            "RUSTC_WRAPPER",
            DX_RUSTC_WRAPPER_ENV_VAR,
            "CARGO_MAKEFLAGS",
            "MAKEFLAGS",
        ];
        cmd.envs(
            rustc_args
                .envs
                .iter()
                .filter(|(k, _)| !filtered_env_keys.contains(&k.as_str()))
                .cloned(),
        );

        let output = cmd.output().await?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!(
                "Failed to compile workspace dep crate '{crate_name}':\n{stderr}"
            );
        }

        Ok(())
    }

    /// Find the rlib path for a workspace crate from its captured rustc args.
    ///
    /// Extracts `--out-dir` from the args and globs for `lib<crate_name>-*.rlib`.
    fn find_rlib_for_crate(&self, crate_name: &str, rustc_args: &RustcArgs) -> Option<PathBuf> {
        // Extract --out-dir from the captured args
        let out_dir = rustc_args
            .args
            .iter()
            .zip(rustc_args.args.iter().skip(1))
            .find(|(flag, _)| *flag == "--out-dir")
            .map(|(_, dir)| PathBuf::from(dir))?;

        // Glob for lib<crate_name>-<hash>.rlib in the output directory
        let prefix = format!("lib{crate_name}-");
        let entries = std::fs::read_dir(&out_dir).ok()?;
        for entry in entries.flatten() {
            if let Some(name) = entry.file_name().to_str() {
                if name.starts_with(&prefix) && name.ends_with(".rlib") {
                    return Some(entry.path());
                }
            }
        }

        None
    }

    /// with our hotpatching setup since it uses linker interception.
    ///
    /// This is sadly a hack. I think there might be other ways of busting the fingerprint (rustc wrapper?)
    /// but that would require relying on cargo internals.
    ///
    /// This might stop working if/when cargo stabilizes contents-based fingerprinting.
    fn bust_fingerprint(&self, ctx: &BuildContext) -> Result<()> {
        if matches!(ctx.mode, BuildMode::Fat) {
            // Bust fingerprints for ALL workspace member crates during Fat builds.
            // This ensures cargo recompiles them through RUSTC_WORKSPACE_WRAPPER
            // so we capture their rustc args for later thin builds.
            let mut busted = Vec::new();
            for member in self.workspace.krates.workspace_members() {
                if let krates::Node::Krate { krate, .. } = member {
                    self.bust_fingerprint_for_package(&krate.name)?;
                    busted.push(krate.name.clone());
                }
            }
            tracing::debug!("Busted fingerprints for workspace members: {busted:?}");
        }
        Ok(())
    }

    /// Bust cargo fingerprints for a specific package name.
    ///
    /// `dx` compiles everything with `--target` which ends up with a structure like:
    /// `target/<triple>/<profile>/.fingerprint/<package_name>-<hash>`
    ///
    /// Normally you can't rely on this structure (ie with `cargo build`) but the explicit
    /// target arg guarantees this will work.
    pub(crate) fn bust_fingerprint_for_package(&self, package_name: &str) -> Result<()> {
        let fingerprint_dir = self
            .target_dir
            .join(self.triple.to_string())
            .join(&self.profile)
            .join(".fingerprint");

        // split at the last `-` used to separate the hash from the name
        // This causes to more aggressively bust hashes for all combinations of features
        // and fingerprints for this package since we're just ignoring the hash
        if let Ok(entries) = std::fs::read_dir(&fingerprint_dir) {
            for entry in entries.flatten() {
                if let Some(fname) = entry.file_name().to_str() {
                    if let Some((name, _)) = fname.rsplit_once('-') {
                        if name == package_name {
                            _ = std::fs::remove_dir_all(entry.path());
                        }
                    }
                }
            }
        }
        Ok(())
    }


    pub(crate) fn patch_cache_exe(&self, exe: &Path) -> PathBuf {
        match self.bundle {
            BundleFormat::Web => self.wasm_bindgen_wasm_output_file(),
            _ => exe.to_path_buf(),
        }
    }

    pub(crate) fn create_patch_cache(&self, exe: &Path) -> Result<HotpatchModuleCache> {
        Ok(HotpatchModuleCache::new(
            &self.patch_cache_exe(exe),
            &self.triple,
        )?)
    }

    /// Users create an index.html for their SPA if they want it
    ///
    /// We always write our wasm as main.js and main_bg.wasm
    ///
    /// In prod we run the optimizer which bundles everything together properly
    ///
    /// So their index.html needs to include main.js in the scripts otherwise nothing happens?
    ///
    /// Seems like every platform has a weird file that declares a bunch of stuff
    /// - web: index.html
    /// - ios: info.plist
    /// - macos: info.plist
    /// - linux: appimage root thing?
    /// - android: androidmanifest.xml
    ///
    /// You also might different variants of these files (staging / prod) and different flavors (eu/us)
    ///
    /// web's index.html is weird since it's not just a bundle format but also a *content* format
    pub(crate) fn prepare_html(
        &self,
        assets: &AssetManifest,
        wasm_path: &str,
        js_path: &str,
    ) -> Result<String> {
        let mut html = {
            const DEV_DEFAULT_HTML: &str = include_str!("../../assets/web/dev.index.html");
            const PROD_DEFAULT_HTML: &str = include_str!("../../assets/web/prod.index.html");

            let crate_root: &Path = &self.crate_dir();
            let custom_html_file = crate_root.join("index.html");
            let default_html = match self.release {
                true => PROD_DEFAULT_HTML,
                false => DEV_DEFAULT_HTML,
            };
            std::fs::read_to_string(custom_html_file).unwrap_or_else(|_| String::from(default_html))
        };

        // Inject any resources from the config into the html
        self.inject_resources(assets, &mut html)?;

        // Inject loading scripts if they are not already present
        self.inject_loading_scripts(assets, &mut html);

        // Replace any special placeholders in the HTML with resolved values
        self.replace_template_placeholders(&mut html, wasm_path, js_path);

        let title = self.config.web.app.title.clone();
        Self::replace_or_insert_before("{app_title}", "</title", &title, &mut html);

        Ok(html)
    }

    fn is_dev_build(&self) -> bool {
        !self.release
    }

    // Inject any resources from the config into the html
    fn inject_resources(&self, assets: &AssetManifest, html: &mut String) -> Result<()> {
        use std::fmt::Write;

        // Collect all resources into a list of styles and scripts
        let resources = &self.config.web.resource;
        let mut style_list = resources.style.clone().unwrap_or_default();
        let mut script_list = resources.script.clone().unwrap_or_default();

        if self.is_dev_build() {
            style_list.extend(resources.dev.style.iter().cloned());
            script_list.extend(resources.dev.script.iter().cloned());
        }

        let mut head_resources = String::new();

        // Add all styles to the head
        for style in &style_list {
            writeln!(
                &mut head_resources,
                "<link rel=\"stylesheet\" href=\"{}\">",
                &style.to_str().unwrap(),
            )?;
        }

        // Add all scripts to the head
        for script in &script_list {
            writeln!(
                &mut head_resources,
                "<script src=\"{}\"></script>",
                &script.to_str().unwrap(),
            )?;
        }

        // Add the base path to the head if this is a debug build
        if self.is_dev_build() {
            if let Some(base_path) = &self.trimmed_base_path() {
                head_resources.push_str(&format_base_path_meta_element(base_path));
            }
        }

        // Inject any resources from manganis into the head
        for asset in assets.unique_assets() {
            let asset_path = asset.bundled_path();
            match asset.options().variant() {
                AssetVariant::Css(css_options) => {
                    if css_options.preloaded() {
                        _ = write!(
                            head_resources,
                            r#"<link rel="preload" as="style" href="/{{base_path}}/assets/{asset_path}" crossorigin>"#
                        );
                    }
                    if css_options.static_head() {
                        _ = write!(
                            head_resources,
                            r#"<link rel="stylesheet" href="/{{base_path}}/assets/{asset_path}" type="text/css">"#
                        );
                    }
                }
                AssetVariant::Image(image_options) => {
                    if image_options.preloaded() {
                        _ = write!(
                            head_resources,
                            r#"<link rel="preload" as="image" href="/{{base_path}}/assets/{asset_path}" crossorigin>"#
                        );
                    }
                }
                AssetVariant::Js(js_options) => {
                    if js_options.preloaded() {
                        _ = write!(
                            head_resources,
                            r#"<link rel="preload" as="script" href="/{{base_path}}/assets/{asset_path}" crossorigin>"#
                        );
                    }
                    if js_options.static_head() {
                        _ = write!(
                            head_resources,
                            r#"<script src="/{{base_path}}/assets/{asset_path}"></script>"#
                        );
                    }
                }
                _ => {}
            }
        }

        // Do not preload the wasm file, because in Safari, preload as=fetch requires additional fetch() options to exactly match the network request
        // And if they do not match then Safari downloads the wasm file twice.
        // See https://github.com/wasm-bindgen/wasm-bindgen/blob/ac51055a4c39fa0affe02f7b63fb1d4c9b3ddfaf/crates/cli-support/src/js/mod.rs#L967
        Self::replace_or_insert_before("{style_include}", "</head", &head_resources, html);

        Ok(())
    }

    /// Inject loading scripts if they are not already present
    fn inject_loading_scripts(&self, assets: &AssetManifest, html: &mut String) {
        // If the current build opted out of injecting loading scripts, don't inject anything
        if !self.inject_loading_scripts {
            return;
        }

        // If not, insert the script
        *html = html.replace(
            "</body",
            &format!(
                r#"<script type="module" async src="/{}/{}"></script>
            </body"#,
                self.base_path_or_default(),
                self.bundled_js_path(assets)
            ),
        );
    }

    /// Replace any special placeholders in the HTML with resolved values
    fn replace_template_placeholders(&self, html: &mut String, wasm_path: &str, js_path: &str) {
        let base_path = self.base_path_or_default();
        *html = html.replace("{base_path}", base_path);

        let app_name = &self.executable_name();

        // If the html contains the old `{app_name}` placeholder, replace {app_name}_bg.wasm and {app_name}.js
        // with the new paths
        *html = html.replace("wasm/{app_name}_bg.wasm", wasm_path);
        *html = html.replace("wasm/{app_name}.js", js_path);

        // Otherwise replace the new placeholders
        *html = html.replace("{wasm_path}", wasm_path);
        *html = html.replace("{js_path}", js_path);

        // Replace the app_name if we find it anywhere standalone
        *html = html.replace("{app_name}", app_name);
    }

    /// Replace a string or insert the new contents before a marker
    fn replace_or_insert_before(
        replace: &str,
        or_insert_before: &str,
        with: &str,
        content: &mut String,
    ) {
        if content.contains(replace) {
            *content = content.replace(replace, with);
        } else if let Some(pos) = content.find(or_insert_before) {
            content.insert_str(pos, with);
        }
    }

    /// Resolve the configured public directory relative to the crate, if any.
    pub(crate) fn user_public_dir(&self) -> Option<PathBuf> {
        let path = self.config.application.public_dir.as_ref()?;

        if path.as_os_str().is_empty() {
            return None;
        }

        Some(if path.is_absolute() {
            path.clone()
        } else {
            self.crate_dir().join(path)
        })
    }

    pub(crate) fn path_is_in_public_dir(&self, path: &Path) -> bool {
        let Some(static_dir) = self.user_public_dir() else {
            return false;
        };

        // Canonicalize when possible so we work with editors that use tmp files
        let canonical_static =
            dunce::canonicalize(&static_dir).unwrap_or_else(|_| static_dir.clone());
        let canonical_path = dunce::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());

        canonical_path.starts_with(&canonical_static)
    }

    /// Get the base path from the config or None if this is not a web or server build
    pub(crate) fn base_path(&self) -> Option<&str> {
        self.base_path
            .as_deref()
            .or(self.config.web.app.base_path.as_deref())
            .filter(|_| matches!(self.bundle, BundleFormat::Web | BundleFormat::Server))
    }

    /// Get the normalized base path for the application with `/` trimmed from both ends.
    pub(crate) fn trimmed_base_path(&self) -> Option<&str> {
        self.base_path()
            .map(|p| p.trim_matches('/'))
            .filter(|p| !p.is_empty())
    }

    /// Get the trimmed base path or `.` if no base path is set
    pub(crate) fn base_path_or_default(&self) -> &str {
        self.trimmed_base_path().unwrap_or(".")
    }

    /// Get the path to the package manifest directory
    pub(crate) fn package_manifest_dir(&self) -> PathBuf {
        self.workspace.krates[self.crate_package]
            .manifest_path
            .parent()
            .unwrap()
            .to_path_buf()
            .into()
    }

    /// Returns the min sdk version set in config. If not set 24 is returned as a default.
    pub(crate) fn min_sdk_version_or_default(&self) -> u32 {
        self.config
            .application
            .android_min_sdk_version
            .unwrap_or(28)
    }

    pub(crate) async fn start_simulators(&self) -> Result<()> {
        if self.device_name.is_some() {
            return Ok(());
        }

        match self.bundle {
            // Boot an iOS simulator if one is not already running.
            //
            // We always choose the most recently opened simulator based on the xcrun list.
            // Note that simulators can be running but the simulator app itself is not open.
            // Calling `open::that` is always fine, even on running apps, since apps are singletons.
            BundleFormat::Ios => {
                #[derive(Deserialize, Debug)]
                struct XcrunListJson {
                    // "com.apple.CoreSimulator.SimRuntime.iOS-18-4": [{}, {}, {}]
                    devices: BTreeMap<String, Vec<XcrunDevice>>,
                }

                #[derive(Deserialize, Debug)]
                struct XcrunDevice {
                    #[serde(rename = "lastBootedAt")]
                    last_booted_at: Option<String>,
                    udid: String,
                    name: String,
                    state: String,
                }
                let xcrun_list = Command::new("xcrun")
                    .arg("simctl")
                    .arg("list")
                    .arg("-j")
                    .output()
                    .await?;

                let as_str = String::from_utf8_lossy(&xcrun_list.stdout);
                let xcrun_list_json = serde_json::from_str::<XcrunListJson>(as_str.trim());
                if let Ok(xcrun_list_json) = xcrun_list_json {
                    if xcrun_list_json.devices.is_empty() {
                        tracing::warn!(
                            "No iOS sdks installed found. Please install the iOS SDK in Xcode."
                        );
                    }

                    if let Some((_rt, devices)) = xcrun_list_json.devices.iter().next() {
                        if devices.iter().all(|device| device.state != "Booted") {
                            let last_booted =
                                devices
                                    .iter()
                                    .max_by_key(|device| match device.last_booted_at {
                                        Some(ref last_booted) => last_booted,
                                        None => "2000-01-01T01:01:01Z",
                                    });

                            if let Some(device) = last_booted {
                                tracing::info!("Booting iOS simulator: \"{}\"", device.name);
                                Command::new("xcrun")
                                    .arg("simctl")
                                    .arg("boot")
                                    .arg(&device.udid)
                                    .output()
                                    .await?;
                            }
                        }
                    }
                }
                let path_to_xcode = Command::new("xcode-select")
                    .arg("--print-path")
                    .output()
                    .await?;
                let path_to_xcode: PathBuf = String::from_utf8_lossy(&path_to_xcode.stdout)
                    .as_ref()
                    .trim()
                    .into();
                let path_to_sim = path_to_xcode.join("Applications").join("Simulator.app");
                open::that_detached(path_to_sim)?;
            }

            BundleFormat::Android => {
                let tools = self.workspace.android_tools()?;
                tokio::spawn(async move {
                    let emulator = tools.emulator();
                    let avds = Command::new(&emulator)
                        .arg("-list-avds")
                        .output()
                        .await
                        .unwrap();
                    let avds = String::from_utf8_lossy(&avds.stdout);
                    let avd = avds.trim().lines().next().map(|s| s.trim().to_string());
                    if let Some(avd) = avd {
                        tracing::info!("Booting Android emulator: \"{avd}\"");
                        Command::new(&emulator)
                            .arg("-avd")
                            .arg(avd)
                            .args(["-netdelay", "none", "-netspeed", "full"])
                            .stdout(std::process::Stdio::null()) // prevent accumulating huge amounts of mem usage
                            .stderr(std::process::Stdio::null()) // prevent accumulating huge amounts of mem usage
                            .output()
                            .await
                            .unwrap();
                    } else {
                        tracing::warn!("No Android emulators found. Please create one using `emulator -avd <name>`");
                    }
                });
            }

            _ => {
                // nothing - maybe on the web we should open the browser?
            }
        };

        Ok(())
    }

    /// Assemble a series of `--config key=value` arguments for the build command.
    ///
    /// This adds adhoc profiles that dx uses to isolate builds from each other. Normally if you ran
    /// `cargo build --feature desktop` and `cargo build --feature server`, then both binaries get
    /// the same name and overwrite each other, causing thrashing and locking issues.
    ///
    /// By creating adhoc profiles, we can ensure that each build is isolated and doesn't interfere with each other.
    ///
    /// The user can also define custom profiles in their `Cargo.toml` file, which will be used instead
    /// of the adhoc profiles.
    ///
    /// The names of the profiles are:
    /// - web-dev
    /// - web-release
    /// - desktop-dev
    /// - desktop-release
    /// - server-dev
    /// - server-release
    /// - ios-dev
    /// - ios-release
    /// - android-dev
    /// - android-release
    /// - liveview-dev
    /// - liveview-release
    ///
    /// Note how every platform gets its own profile, and each platform has a dev and release profile.
    fn profile_args(&self) -> Vec<String> {
        // Always disable stripping so symbols still exist for the asset system. We will apply strip manually
        // after assets are built
        let profile = self.profile.as_str();
        let mut args = Vec::new();
        args.push(format!(r#"profile.{profile}.strip=false"#));

        // If the user defined the profile in the Cargo.toml, we don't need to add it to our adhoc list
        if !self
            .workspace
            .cargo_toml
            .profile
            .custom
            .contains_key(&self.profile)
        {
            // Otherwise, we need to add the profile arguments to make it adhoc
            let inherits = if self.release { "release" } else { "dev" };

            // Add the profile definition first.
            args.push(format!(r#"profile.{profile}.inherits="{inherits}""#));

            // The default dioxus experience is to lightly optimize the web build, both in debug and release
            // Note that typically in release builds, you would strip debuginfo, but we actually choose to do
            // that with wasm-opt tooling instead.
            if matches!(self.bundle, BundleFormat::Web) {
                if self.release {
                    args.push(format!(r#"profile.{profile}.opt-level="s""#));
                }

                if self.wasm_split {
                    args.push(format!(r#"profile.{profile}.lto=true"#));
                    args.push(format!(r#"profile.{profile}.debug=true"#));
                }
            }
        }

        // Prepend --config to each argument
        args.into_iter()
            .flat_map(|arg| ["--config".to_string(), arg])
            .collect()
    }

    pub async fn codesign_apple(&self, ctx: &BuildContext) -> Result<()> {
        ctx.status_codesigning();

        // We don't want to drop the entitlements file, until the end of the block, so we hoist it to this temporary.
        let mut _saved_entitlements = None;

        let mut app_dev_name = self.apple_team_id.clone();
        if app_dev_name.is_none() {
            app_dev_name = Some(Self::auto_provision_signing_name().await.context(
                "Failed to automatically provision signing name for Apple codesigning.",
            )?);
        }

        let mut entitlements_file = self.apple_entitlements.clone();
        if entitlements_file.is_none() {
            let entitlements_xml = Self::auto_provision_entitlements()
                .await
                .context("Failed to auto-provision entitlements for Apple codesigning.")?;
            let entitlements_temp_file = tempfile::NamedTempFile::new()?;
            std::fs::write(entitlements_temp_file.path(), entitlements_xml)?;
            entitlements_file = Some(entitlements_temp_file.path().to_path_buf());
            _saved_entitlements = Some(entitlements_temp_file);
        }

        let entitlements_file = entitlements_file.as_ref().context(
            "No entitlements file provided and could not provision entitlements to sign app.",
        )?;
        let app_dev_name = app_dev_name.as_ref().context(
            "No Apple Development signing name provided and could not auto-provision one.",
        )?;

        tracing::debug!(
            "Codesigning Apple app with entitlements: {} and dev name: {}",
            entitlements_file.display(),
            app_dev_name
        );

        // determine the target exe - the server and macos bundles are different
        let target_exe = match self.bundle {
            BundleFormat::MacOS => self.root_dir(),
            BundleFormat::Ios => self.root_dir(),
            BundleFormat::Server => self.main_exe(),
            _ => bail!("Codesigning is only supported for MacOS and iOS bundles"),
        };

        // codesign the app
        let output = Command::new("codesign")
            .args([
                "--force",
                "--entitlements",
                entitlements_file.to_str().unwrap(),
                "--sign",
                app_dev_name,
            ])
            .arg(target_exe)
            .output()
            .await
            .context("Failed to codesign the app - is `codesign` in your path?")?;

        if !output.status.success() {
            bail!(
                "Failed to codesign the app: {}",
                String::from_utf8(output.stderr).unwrap_or_default()
            );
        }

        Ok(())
    }

    async fn auto_provision_signing_name() -> Result<String> {
        let identities = Command::new("security")
            .args(["find-identity", "-v", "-p", "codesigning"])
            .output()
            .await
            .context("Failed to run `security find-identity -v -p codesigning` - is `security` in your path?")
            .map(|e| {
                String::from_utf8(e.stdout)
                    .context("Failed to parse `security find-identity -v -p codesigning`")
            })??;

        // Parsing this:
        // 1231231231231asdasdads123123 "Apple Development: foo@gmail.com (XYZYZY)"
        let app_dev_name = regex::Regex::new(r#""Apple Development: (.+)""#)
            .unwrap()
            .captures(&identities)
            .and_then(|caps| caps.get(1))
            .map(|m| m.as_str())
            .context(
                "Failed to find Apple Development in `security find-identity -v -p codesigning`",
            )?;

        Ok(app_dev_name.to_string())
    }

    async fn auto_provision_entitlements() -> Result<String> {
        const CODESIGN_ERROR: &str = r#"This is likely because you haven't
- Created a provisioning profile before
- Accepted the Apple Developer Program License Agreement

The agreement changes frequently and might need to be accepted again.
To accept the agreement, go to https://developer.apple.com/account

To create a provisioning profile, follow the instructions here:
https://developer.apple.com/documentation/xcode/sharing-your-teams-signing-certificates"#;

        // Check the xcode 16 location first
        let mut profiles_folder = dirs::home_dir()
            .context("Your machine has no home-dir")?
            .join("Library/Developer/Xcode/UserData/Provisioning Profiles");

        // If it doesn't exist, check the old location
        if !profiles_folder.exists() {
            profiles_folder = dirs::home_dir()
                .context("Your machine has no home-dir")?
                .join("Library/MobileDevice/Provisioning Profiles");
        }

        if !profiles_folder.exists() || profiles_folder.read_dir()?.next().is_none() {
            tracing::error!(
                r#"No provisioning profiles found when trying to codesign the app.
We checked the folders:
- XCode16: ~/Library/Developer/Xcode/UserData/Provisioning Profiles
- XCode15: ~/Library/MobileDevice/Provisioning Profiles

{CODESIGN_ERROR}
"#
            )
        }

        // Acquire the provision file
        let provision_file = profiles_folder
            .read_dir()?
            .flatten()
            .find(|entry| {
                entry
                    .file_name()
                    .to_str()
                    .map(|s| s.contains("mobileprovision"))
                    .unwrap_or_default()
            })
            .context("Failed to find a provisioning profile. \n\n{CODESIGN_ERROR}")?;

        // The .mobileprovision file has some random binary thrown into into, but it's still basically a plist
        // Let's use the plist markers to find the start and end of the plist
        fn cut_plist(bytes: &[u8], byte_match: &[u8]) -> Option<usize> {
            bytes
                .windows(byte_match.len())
                .enumerate()
                .rev()
                .find(|(_, slice)| *slice == byte_match)
                .map(|(i, _)| i + byte_match.len())
        }
        let bytes = std::fs::read(provision_file.path())?;
        let cut1 = cut_plist(&bytes, b"<plist").context("Failed to parse .mobileprovision file")?;
        let cut2 = cut_plist(&bytes, r#"</dict>"#.as_bytes())
            .context("Failed to parse .mobileprovision file")?;
        let sub_bytes = &bytes[(cut1 - 6)..cut2];
        let mbfile: ProvisioningProfile =
            plist::from_bytes(sub_bytes).context("Failed to parse .mobileprovision file")?;

        #[derive(serde::Deserialize, Debug)]
        struct ProvisioningProfile {
            #[serde(rename = "TeamIdentifier")]
            team_identifier: Vec<String>,
            #[serde(rename = "Entitlements")]
            entitlements: Entitlements,
            #[allow(dead_code)]
            #[serde(rename = "ApplicationIdentifierPrefix")]
            application_identifier_prefix: Vec<String>,
        }

        #[derive(serde::Deserialize, Debug)]
        struct Entitlements {
            #[serde(rename = "application-identifier")]
            application_identifier: String,
            #[serde(rename = "keychain-access-groups")]
            keychain_access_groups: Vec<String>,
        }

        Ok(format!(
            r#"
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
    <key>application-identifier</key>
    <string>{APPLICATION_IDENTIFIER}</string>
    <key>keychain-access-groups</key>
    <array>
        <string>{APP_ID_ACCESS_GROUP}.*</string>
    </array>
    <key>get-task-allow</key>
    <true/>
    <key>com.apple.developer.team-identifier</key>
    <string>{TEAM_IDENTIFIER}</string>
</dict></plist>
        "#,
            APPLICATION_IDENTIFIER = mbfile.entitlements.application_identifier,
            APP_ID_ACCESS_GROUP = mbfile.entitlements.keychain_access_groups[0],
            TEAM_IDENTIFIER = mbfile.team_identifier[0],
        ))
    }

    async fn write_app_manifest(&self, assets: &AssetManifest) -> Result<()> {
        let manifest = AppManifest {
            assets: assets.clone(),
            cli_version: crate::VERSION.to_string(),
            rust_version: self.workspace.rustc_version.clone(),
        };

        let manifest_path = self.app_manifest();
        std::fs::write(&manifest_path, serde_json::to_string_pretty(&manifest)?)?;

        Ok(())
    }
}
