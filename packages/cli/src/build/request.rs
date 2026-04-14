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
//! ### Windows / Linux:
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

use super::HotpatchModuleCache;
use crate::{
    opt::{process_file_to, AppManifest},
    WorkspaceRustcArgs,
};
use crate::{
    AndroidTools, BuildContext, BuildId, BundleFormat, DioxusConfig, LinkAction, Platform,
    Renderer, Result, RustcArgs, TargetArgs, Workspace, DX_RUSTC_WRAPPER_ENV_VAR,
};
use anyhow::{bail, Context};
use cargo_metadata::diagnostic::Diagnostic;
use cargo_toml::{Profile, Profiles, StripSetting};
use depinfo::RustcDepInfo;
use dioxus_cli_config::PRODUCT_NAME_ENV;
use dioxus_cli_config::{APP_TITLE_ENV, ASSET_ROOT_ENV};
use krates::{cm::TargetKind, NodeId};
use manganis::BundledAsset;
use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};
use serde::Deserialize;
use std::{borrow::Cow, collections::VecDeque, ffi::OsString};
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    process::Stdio,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::SystemTime,
};
use target_lexicon::{Architecture, OperatingSystem, Triple};
use tempfile::TempDir;
use tokio::{io::AsyncBufReadExt, process::Command};

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
    pub(crate) all_features: bool,
    pub(crate) target_dir: PathBuf,
    pub(crate) skip_assets: bool,
    pub(crate) wasm_split: bool,
    pub(crate) debug_symbols: bool,
    pub(crate) keep_names: bool,
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
#[allow(clippy::large_enum_variant)]
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
        /// List of changed files causing this rebuild. Mostly used for diagnostics
        changed_files: Vec<PathBuf>,

        /// The ASLR slide of the running program, used to hardcode symbol jumps
        aslr_reference: u64,

        /// The captured RustcArgs for every crate in the workspace, collected by RUSTC_WORKSPACE_WRAPPER
        /// This is used for replaying rustc invocations for workspace hotpatching
        workspace_rustc_args: WorkspaceRustcArgs,

        /// Cumulative set of all workspace crates modified since the fat build.
        modified_crates: HashSet<String>,

        /// Cache of initial binary parsing which speeds up stub creation
        cache: Arc<HotpatchModuleCache>,
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
    pub(crate) workspace_rustc: WorkspaceRustcArgs,
    pub(crate) time_start: SystemTime,
    pub(crate) time_end: SystemTime,
    pub(crate) assets: AppManifest,
    pub(crate) mode: BuildMode,
    pub(crate) patch_cache: Option<Arc<HotpatchModuleCache>>,
    pub(crate) depinfo: RustcDepInfo,
    pub(crate) build_id: BuildId,
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

        // Load config from Dioxus.toml and/or inline config in the target's source file.
        // Inline config in doc comments takes precedence over Dioxus.toml.
        let config = workspace
            .load_dioxus_config(crate_package, Some(crate_target.src_path.as_std_path()))?
            .unwrap_or_default();

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
                            AndroidTools::current()
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
        if matches!(bundle, BundleFormat::Ios)
            && matches!(
                triple.operating_system,
                target_lexicon::OperatingSystem::IOS(_)
            )
        {
            let xcode_path = Workspace::get_xcode_path()
                .await
                .unwrap_or_else(|| "/Applications/Xcode.app".to_string().into());

            let sysroot_location = match triple.environment {
                target_lexicon::Environment::Sim => xcode_path
                    .join("Platforms/iPhoneSimulator.platform/Developer/SDKs/iPhoneSimulator.sdk"),
                _ => {
                    // If the target has been determined as the iOS x86 simulator above
                    if triple.to_string() == "x86_64-apple-ios" {
                        xcode_path.join(
                            "Platforms/iPhoneSimulator.platform/Developer/SDKs/iPhoneSimulator.sdk",
                        )
                    } else {
                        xcode_path.join("Platforms/iPhoneOS.platform/Developer/SDKs/iPhoneOS.sdk")
                    }
                }
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
            keep_names: args.keep_names,
            inject_loading_scripts: args.inject_loading_scripts,
            apple_entitlements: args.apple_entitlements.clone(),
            apple_team_id: args.apple_team_id.clone(),
            raw_json_diagnostics: args.raw_json_diagnostics,
            windows_subsystem: args.windows_subsystem.clone(),
        })
    }

    pub(crate) async fn prebuild(&self, ctx: &BuildContext) -> Result<()> {
        ctx.profile_phase("Prebuild");

        // Create the session cache directory
        let cache_dir = self.session_cache_dir();
        _ = std::fs::create_dir_all(&cache_dir);
        _ = std::fs::create_dir_all(self.rustc_wrapper_args_dir());
        _ = std::fs::create_dir_all(self.rustc_wrapper_args_scope_dir(&ctx.mode)?);
        _ = std::fs::File::create(self.link_err_file());
        _ = std::fs::File::create(self.link_args_file());
        _ = std::fs::File::create(self.windows_command_file());

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

    pub(crate) async fn build(&self, ctx: BuildContext) -> Result<BuildArtifacts> {
        match &ctx.mode {
            // In hotpatch mode, we use the dedicated hotpatch flow
            BuildMode::Thin { .. } => self.compile_workspace_hotpatch(&ctx).await,

            // In base/fat mode, we do the full chain with a root `cargo rustc`
            BuildMode::Base { .. } | BuildMode::Fat => {
                let mut artifacts = self.cargo_build(&ctx).await?;

                ctx.profile_phase("Post-processing executable");
                self.post_process_executable(&artifacts).await?;

                ctx.profile_phase("Writing executable");
                self.write_executable(&ctx, &mut artifacts)
                    .await
                    .context("Failed to write executable")?;

                ctx.profile_phase("Writing frameworks");
                self.write_frameworks(&artifacts)
                    .await
                    .context("Failed to write frameworks")?;

                ctx.profile_phase("Writing assets");
                self.write_assets(&ctx, &artifacts.assets)
                    .await
                    .context("Failed to write assets")?;

                ctx.profile_phase("Writing metadata");
                self.write_metadata()
                    .await
                    .context("Failed to write metadata")?;

                ctx.profile_phase("Writing ffi");
                self.write_ffi_plugins(&ctx, &artifacts).await?;

                ctx.profile_phase("Running optimizer");
                self.optimize(&ctx)
                    .await
                    .context("Failed to optimize build")?;

                ctx.profile_phase("Running assemble");
                self.assemble(&ctx)
                    .await
                    .context("Failed to assemble build")?;

                ctx.profile_phase("Populating cache");
                self.fill_caches(&ctx, &mut artifacts).await?;

                tracing::debug!("Bundle created at {}", self.root_dir().display());

                Ok(artifacts)
            }
        }
    }

    /// Run the cargo build by assembling the build command and executing it.
    ///
    /// This method needs to be very careful with processing output since errors being swallowed will
    /// be very confusing to the user.
    ///
    /// This method is only meant to be run by fat/full builds - not by hotpatch builds
    pub async fn cargo_build(&self, ctx: &BuildContext) -> Result<BuildArtifacts> {
        let time_start = SystemTime::now();

        // If we forget to do this, then we won't get the linker args since rust skips the full build
        // We need to make sure to not react to this though, so the filemap must cache it
        _ = self.bust_fingerprint(ctx);

        // Extract the unit count of the crate graph so build_cargo has more accurate data
        // "Thin" builds only build the final exe, so we only need to build one crate
        let crate_count = match ctx.mode {
            BuildMode::Thin { .. } => 1,
            _ => self.get_unit_count_estimate(&ctx.mode).await,
        };

        // Spawn the `cargo rustc` or `rustc` command
        let mut child = self
            .cargo_build_command(&ctx.mode)?
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context("Failed to spawn cargo build")?;

        // Direct rustc thin builds don't emit Cargo-style unit progress messages, so if we don't
        // advance the profiler here the entire compile winds up attributed to "Starting Build".
        ctx.status_starting_build(crate_count);

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

                // Here, we record
                Message::CompilerArtifact(artifact) => {
                    units_compiled += 1;
                    let target_name = artifact.target.name.clone();
                    ctx.status_build_progress(
                        units_compiled,
                        crate_count,
                        target_name,
                        artifact.fresh,
                    );
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

        // If there's any warnings from the linker, we should print them out
        self.print_linker_warnings(&output_location);

        // Load the captured rustc args from the rustc_workspace_wrapper
        let workspace_rustc_args = self.load_rustc_argset()?;

        // Ensure the final exe exists - throw if it doesn't
        let exe = output_location.context("Cargo build failed - no output location. Toggle tracing mode (press `t`) for more information.")?;

        // Fat builds need to be linked with the fat linker. Would also like to link here for thin builds
        if matches!(ctx.mode, BuildMode::Fat) {
            self.run_fat_link(ctx, &exe, &workspace_rustc_args).await?;
        }

        // Asset extraction is starts bundle
        ctx.status_start_bundle();

        // Extract all linker metadata (assets, Android/iOS plugins, widget extensions) in a single pass.
        let assets = self.collect_assets_and_metadata(&exe, ctx).await?;

        let time_end = SystemTime::now();
        let mode = ctx.mode.clone();
        let depinfo = RustcDepInfo::from_file(&exe.with_extension("d")).unwrap_or_default();

        Ok(BuildArtifacts {
            time_end,
            exe,
            workspace_rustc: workspace_rustc_args,
            time_start,
            assets,
            mode,
            depinfo,
            root_dir: self.root_dir(),
            patch_cache: None,
            build_id: ctx.build_id,
        })
    }

    /// with our hotpatching setup since it uses linker interception.
    ///
    /// This is sadly a hack. I think there might be other ways of busting the fingerprint (rustc wrapper?)
    /// but that would require relying on cargo internals.
    ///
    /// This might stop working if/when cargo stabilizes contents-based fingerprinting.
    ///
    /// `dx` compiles everything with `--target` which ends up with a structure like:
    /// `target/<triple>/<profile>/.fingerprint/<package_name>-<hash>`
    ///
    /// Normally you can't rely on this structure (ie with `cargo build`) but the explicit
    /// target arg guarantees this will work.
    ///
    /// Each binary target has a fingerprinted location which we place under
    /// `target/dx/.args/name-hash.lib.json`
    ///
    /// The hash includes the various args profile info that provide entropy to disambiguate the crate.
    /// You might see `.args/serde-123.lib.json` in the same folder as `.args/serde-456.json` because
    /// they might have different config flags, different target triples, different opt levels, etc.
    /// Bust cargo fingerprints to force recompilation during the fat build.
    ///
    /// The tip crate is always busted so we get a fresh linker invocation. For workspace
    /// dependency crates, we check whether we already have cached rustc args in the scope
    /// directory from a previous run. If a crate's `<name>.lib.json` exists, its args are
    /// still valid and we skip busting so cargo can reuse its incremental artifacts. If the
    /// file is missing, we bust that crate's fingerprint to force the rustc wrapper to
    /// re-capture its args.
    fn bust_fingerprint(&self, ctx: &BuildContext) -> Result<()> {
        // Ensure the rustc args capture directory exists - only in fat/base builds.
        // This ensures we always capture fresh rustc args provided we're not hotpatching. This could
        // be annoying for regular dx build/bundle commands, but should generally be fine.
        // todo: think about how CI might interact with this if it's not persisting the dx folder
        if matches!(ctx.mode, BuildMode::Thin { .. }) {
            return Ok(());
        }

        // Always make sure the rustc arg dir is ready to receive rustc_wrapper emits
        _ = std::fs::create_dir_all(&self.rustc_wrapper_args_scope_dir(&ctx.mode)?)
            .context("Failed to create rustc wrapper args scope dir");

        // Only remove args in fat mode
        if !matches!(ctx.mode, BuildMode::Fat) {
            return Ok(());
        }

        // Always bust the tip crate.
        let mut bust = HashSet::new();
        bust.insert(self.package().name.clone());

        // Walk workspace deps of the tip crate. If we're missing cached args for any of
        // them, bust their fingerprint so the wrapper re-captures during this fat build.
        // Use the raw path (not canonicalized) since the scope dir may not exist yet.
        let scope_dir = self
            .rustc_wrapper_args_dir()
            .join(self.rustc_wrapper_scope_dir_name(&BuildMode::Fat)?);

        for dep_name in self.workspace_crate_dep_names() {
            if !scope_dir.join(format!("{dep_name}.lib.json")).exists() {
                bust.insert(dep_name);
            }
        }

        let fingerprint_dir = self.cargo_fingerprint_dir();
        for entry in std::fs::read_dir(&fingerprint_dir)
            .into_iter()
            .flatten()
            .flatten()
        {
            if let Some(fname) = entry.file_name().to_str() {
                if let Some((name, _)) = fname.rsplit_once('-') {
                    if bust.contains(name) {
                        _ = std::fs::remove_dir_all(entry.path());
                    }
                }
            }
        }

        Ok(())
    }

    /// Take the output of rustc and make it into the main exe of the bundle
    ///
    /// For wasm, we'll want to run `wasm-bindgen` to make it a wasm binary along with some other optimizations
    /// Other platforms we might do some stripping or other optimizations
    /// Move the executable to the workdir
    async fn write_executable(
        &self,
        ctx: &BuildContext,
        artifacts: &mut BuildArtifacts,
    ) -> Result<()> {
        match self.bundle {
            // Run wasm-bindgen on the wasm binary and set its output to be in the bundle folder
            // Also run wasm-opt on the wasm binary, and sets the index.html since that's also the "executable".
            //
            // The wasm stuff will be in a folder called "wasm" in the workdir.
            BundleFormat::Web => {
                self.bundle_web(ctx, &artifacts.exe, &mut artifacts.assets)
                    .await?;
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
                std::fs::copy(&artifacts.exe, self.main_exe())?;
            }
        }

        Ok(())
    }

    /// Bundle shared libraries into the app's frameworks folder so they sit next to the binary
    /// at runtime and the dynamic loader can find them.
    ///
    /// Scans the captured linker arguments for `.dylib` and `.so` paths and copies each one
    /// into `frameworks_folder()`. In dev builds on unix/windows the copy is a symlink so
    /// rebuilds don't re-copy large files; release builds do a real copy for distribution.
    ///
    /// Also handles platform-specific extras:
    /// - **Android**: always creates the jniLibs framework dir, copies `libc++_shared.so`
    ///   when `-lc++_shared` appears in the link args, and copies prebuilt OpenSSL libs
    ///   (`libssl.so`, `libcrypto.so`) when the link args reference the OpenSSL directory.
    ///
    /// Note: Windows `.dll` bundling is not yet implemented — system DLLs should not be
    /// bundled, and we don't yet distinguish user DLLs from system ones.
    async fn write_frameworks(&self, artifacts: &BuildArtifacts) -> Result<()> {
        let framework_dir = self.frameworks_folder();

        // We have some prebuilt stuff that needs to be copied into the framework dir
        let openssl_dir = AndroidTools::openssl_lib_dir(&self.triple);
        let openssl_dir_disp = openssl_dir.display().to_string();

        for arg in &artifacts.workspace_rustc.link_args {
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

    /// Collect assets and plugin metadata from the final executable in one pass
    ///
    /// This method extracts assets and FFI plugin metadata (Android/Swift) from the
    /// binary. Permissions are now read from Dioxus.toml, not extracted from the binary.
    pub async fn collect_assets_and_metadata(
        &self,
        exe: &Path,
        ctx: &BuildContext,
    ) -> Result<AppManifest> {
        use super::assets::extract_symbols_from_file;

        let skip_assets = self.skip_assets;
        let needs_android_artifacts = self.bundle == BundleFormat::Android;
        let needs_swift_packages = matches!(self.bundle, BundleFormat::Ios | BundleFormat::MacOS);

        if skip_assets && !needs_android_artifacts && !needs_swift_packages {
            return Ok(AppManifest::new());
        }

        ctx.status_extracting_assets();

        let mut manifest = extract_symbols_from_file(exe).await?;

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
                        manganis_core::AssetOptions::builder()
                            .with_hash_suffix(false)
                            .into_asset_options(),
                    ));
                }
            }
        }

        Ok(manifest)
    }

    /// Copy the assets out of the manifest and into the target location
    ///
    /// Should be the same on all platforms - just copy over the assets from the manifest into the output directory
    async fn write_assets(&self, ctx: &BuildContext, assets: &AppManifest) -> Result<()> {
        // Server doesn't need assets - web will provide them
        if !ctx.is_primary_build() {
            return Ok(());
        }

        let asset_dir = self.bundle_asset_dir();

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
            .load_bundle_manifest()
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

        // Queue the bundled assets (skip sidecar assets that require special processing)
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
        let progress = ctx.clone();
        let ws_dir = self.workspace_dir();
        let esbuild_path = crate::esbuild::Esbuild::path_if_installed();

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

                    let res = process_file_to(options, from, to, esbuild_path.as_deref());
                    if let Err(err) = res.as_ref() {
                        tracing::error!("Failed to copy asset {from:?}: {err}");
                    }

                    progress.status_copied_asset(
                        copied.fetch_add(1, Ordering::SeqCst),
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

    /// Assemble the `cargo rustc` / `rustc` command
    ///
    /// When building fat/base binaries, we use `cargo rustc`.
    /// When building thin binaries, we use `rustc` directly.
    ///
    /// When processing the output of this command, you need to make sure to handle both cases which
    /// both have different formats (but with json output for both).
    fn cargo_build_command(&self, build_mode: &BuildMode) -> Result<Command> {
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
                let rustc_args = workspace_rustc_args
                    .rustc_args
                    .get(&format!("{}.bin", self.tip_crate_name()))
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
            // capture per-crate args without interfering with external dependency compilation. Note
            // that this will also separate dx from cargo/clippy/check because the path to dx ends up
            // being used in the hash check. This means dx's output is always reliable!
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

                // Set the folder where dx will write its captured rustc args for replay.
                // Use the scoped directory so different build configurations (triples,
                // profiles, etc.) don't collide.
                cmd.env(
                    DX_RUSTC_WRAPPER_ENV_VAR,
                    dunce::canonicalize(self.rustc_wrapper_args_scope_dir(build_mode)?)
                        .context("Failed to canonicalize rustc wrapper args dir")?,
                );

                // And then set the wrapper itself as dx. This will ensure both that dx captures the
                // rustc args and the output hashes will not conflict with non-dx tools (cargo includes
                // the RUSTC_WORKSPACE_WRAPPER value *in the artifact hash*)
                cmd.env("RUSTC_WORKSPACE_WRAPPER", Workspace::path_to_dx()?);

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

        // Always push --no-default-features
        cargo_args.push("--no-default-features".to_string());

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

        // dx links android, thin builds, and fat builds with a custom linker.
        // Note: We don't intercept Darwin Base builds since Swift plugins are compiled as dynamic
        // frameworks that load at runtime, not linked statically into the binary.
        let use_dx_linker = self.custom_linker.is_some()
            || matches!(build_mode, BuildMode::Thin { .. } | BuildMode::Fat);

        if use_dx_linker {
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
        // This is dependent on the bundle structure - iOS uses a flat structure while macOS uses nested
        // todo: we need to figure out what to do for windows
        match self.triple.operating_system {
            OperatingSystem::Darwin(_) | OperatingSystem::MacOSX { .. } => {
                // macOS: App.app/Contents/MacOS/exe -> ../Frameworks/
                cargo_args.push("-Clink-arg=-Wl,-rpath,@executable_path/../Frameworks".to_string());
                cargo_args.push("-Clink-arg=-Wl,-rpath,@executable_path".to_string());
            }
            OperatingSystem::IOS(_) => {
                // iOS: App.app/exe -> Frameworks/ (flat bundle structure)
                cargo_args.push("-Clink-arg=-Wl,-rpath,@executable_path/Frameworks".to_string());
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

        // Always bake the product name into the binary so bundled apps can find their assets
        // at runtime regardless of build profile (the asset directory structure uses the product name).
        env_vars.push((PRODUCT_NAME_ENV.into(), self.bundled_app_name().into()));

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
        }

        // Assemble the rustflags by peering into the `.cargo/config.toml` file
        let rust_flags = self.rustflags.clone();

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
        // Note: We don't intercept Darwin Base builds since Swift plugins are compiled as dynamic
        // frameworks that load at runtime, not linked statically into the binary.
        let use_dx_linker = self.custom_linker.is_some()
            || matches!(build_mode, BuildMode::Thin { .. } | BuildMode::Fat);

        if use_dx_linker {
            // For Android, we pass the actual linker so cargo can still link normally.
            // For Fat/Thin builds, we use no-link mode (linker = None).
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

    /// Post-process the final binary.
    /// Strip the final binary after extracting all assets with rustc-objcopy
    async fn post_process_executable(&self, artifacts: &BuildArtifacts) -> Result<()> {
        // Never strip the binary if we are going to bundle split it
        if self.wasm_split {
            return Ok(());
        }

        // Use the same format that rust itself does
        // https://github.com/rust-lang/rust/blob/cb80ff132a0e9aa71529b701427e4e6c243b58df/compiler/rustc_codegen_ssa/src/back/linker.rs#L1433-L1443
        let strip_arg = match self.get_strip_setting() {
            StripSetting::Debuginfo => Some("--strip-debug"),
            StripSetting::Symbols => Some("--strip-all"),
            StripSetting::None => None,
        };

        if let Some(strip_arg) = strip_arg {
            let rustc_objcopy = self.workspace.rustc_objcopy();
            let dylib_path = self.workspace.rustc_objcopy_dylib_path();

            // Use rustc_objcopy in place.
            // todo: actually use this to copy over the binary to our staging
            let mut command = Command::new(rustc_objcopy);
            command.env("LD_LIBRARY_PATH", &dylib_path);
            command
                .arg(strip_arg)
                .arg(&artifacts.exe)
                .arg(&artifacts.exe);
            let output = command.output().await?;
            if !output.status.success() {
                if let Ok(stdout) = std::str::from_utf8(&output.stdout) {
                    tracing::error!("{}", stdout);
                }
                if let Ok(stderr) = std::str::from_utf8(&output.stderr) {
                    tracing::error!("{}", stderr);
                }
                bail!("Failed to strip binary");
            }
        }

        Ok(())
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

    /// Write out the manganis ffi plugins that we support
    /// - Kotlin / Java
    /// - Swift
    /// - TS: Todo
    /// - JS: Todo
    async fn write_ffi_plugins(
        &self,
        ctx: &BuildContext,
        artifacts: &BuildArtifacts,
    ) -> Result<()> {
        // Install prebuilt Android plugin artifacts (AARs + Gradle deps)
        if self.bundle == BundleFormat::Android && !artifacts.assets.android_artifacts.is_empty() {
            let names: Vec<_> = artifacts
                .assets
                .android_artifacts
                .iter()
                .map(|a| a.plugin_name.as_str().to_string())
                .collect();
            ctx.status_compiling_native_plugins(format!("Kotlin build: {}", names.join(", ")));
            self.install_android_artifacts(&artifacts.assets.android_artifacts)
                .context("Failed to install Android plugin artifacts")?;
        }

        if matches!(self.bundle, BundleFormat::Ios | BundleFormat::MacOS)
            && !artifacts.assets.swift_sources.is_empty()
        {
            let names: Vec<_> = artifacts
                .assets
                .swift_sources
                .iter()
                .map(|s| s.plugin_name.as_str().to_string())
                .collect();
            ctx.status_compiling_native_plugins(format!("Swift build: {}", names.join(", ")));

            // Compile Swift packages from source
            self.compile_swift_sources(&artifacts.assets.swift_sources)
                .await
                .context("Failed to compile Swift packages")?;

            // Then embed Swift standard libraries
            self.embed_swift_stdlibs(&artifacts.assets.swift_sources)
                .await
                .context("Failed to embed Swift standard libraries")?;
        }

        // Compile and install Apple Widget Extensions from Dioxus.toml config
        if matches!(self.bundle, BundleFormat::Ios | BundleFormat::MacOS)
            && !self.config.ios.widget_extensions.is_empty()
        {
            let names: Vec<_> = self
                .config
                .ios
                .widget_extensions
                .iter()
                .map(|w| w.display_name.clone())
                .collect();
            ctx.status_compiling_native_plugins(format!("Widget build: {}", names.join(", ")));
            self.compile_widget_extensions()
                .await
                .context("Failed to compile widget extensions")?;
        }

        Ok(())
    }

    /// Run the optimizers, obfuscators, minimizers, signers, etc
    async fn optimize(&self, ctx: &BuildContext) -> Result<()> {
        ctx.profile_phase("Optimizing Bundle");

        match self.bundle {
            BundleFormat::Web => {
                // Compress the asset dir
                // If pre-compressing is enabled, we can pre_compress the wasm-bindgen output
                let pre_compress = self.should_pre_compress_web_assets(self.release);

                if pre_compress {
                    ctx.status_compressing_assets();
                    let asset_dir = self.bundle_asset_dir();
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

    /// Run any final tools to produce apks or other artifacts we might need.
    ///
    /// This might include codesigning, zipping, creating an appimage, etc
    async fn assemble(&self, ctx: &BuildContext) -> Result<()> {
        ctx.profile_phase("Assembling Bundle");

        if let BundleFormat::Android = self.bundle {
            self.assemble_android(ctx).await?;
        }

        // if the triple is a ios or macos target, we need to codesign the binary
        if self.is_apple_target() && self.should_codesign {
            self.codesign_apple(ctx).await?;
        }

        Ok(())
    }

    /// We only really currently care about:
    ///
    /// - app dir (.app, .exe, .apk, etc)
    /// - assetas dir
    /// - exe dir (.exe, .app, .apk, etc)
    /// - extra scaffolding
    ///
    /// It's not guaranteed that they're different from any other folder
    fn prepare_build_dir(&self, ctx: &BuildContext) -> Result<()> {
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
            create_dir_all(self.bundle_asset_dir())?;

            tracing::debug!(
                r#"Initialized build dirs:
               • root dir: {:?}
               • exe dir: {:?}
               • asset dir: {:?}"#,
                self.root_dir(),
                self.exe_dir(),
                self.bundle_asset_dir(),
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

    pub(crate) fn load_bundle_manifest(&self) -> Result<AppManifest> {
        let manifest_path = self.bundle_manifest_file();
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
        ctx.profile_phase("Verify Tooling");
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

    async fn write_app_manifest(&self, manifest: &AppManifest) -> Result<()> {
        std::fs::write(
            self.bundle_manifest_file(),
            serde_json::to_string_pretty(&manifest)?,
        )?;
        Ok(())
    }

    async fn fill_caches(&self, ctx: &BuildContext, artifacts: &mut BuildArtifacts) -> Result<()> {
        // Populate the patch cache if we're in fat mode
        if matches!(ctx.mode, BuildMode::Fat) {
            ctx.profile_phase("Creating Patch Cache");
            let patch_exe = match self.bundle {
                BundleFormat::Web => self.wasm_bindgen_wasm_output_file(),
                _ => artifacts.exe.to_path_buf(),
            };
            let hotpatch_module_cache = HotpatchModuleCache::new(&patch_exe, &self.triple)?;
            artifacts.patch_cache = Some(Arc::new(hotpatch_module_cache));
        }

        Ok(())
    }

    /// Recursively copy a directory and its contents.
    #[allow(clippy::only_used_in_recursion)]
    pub(crate) fn copy_build_dir_recursive(&self, src: &Path, dst: &Path) -> Result<()> {
        std::fs::create_dir_all(dst)?;

        for entry in std::fs::read_dir(src)? {
            let entry = entry?;
            let src_path = entry.path();
            let dst_path = dst.join(entry.file_name());

            if src_path.is_dir() {
                // Skip build directories and hidden folders
                let name = entry.file_name();
                let name_str = name.to_string_lossy();
                if name_str == "build" || name_str == ".gradle" || name_str.starts_with('.') {
                    continue;
                }

                self.copy_build_dir_recursive(&src_path, &dst_path)?;
            } else {
                std::fs::copy(&src_path, &dst_path)?;
            }
        }

        Ok(())
    }

    /// Ensure the right dependencies are installed for linux apps.
    /// This varies by distro, so we just do nothing for now.
    ///
    /// Eventually, we want to check for the prereqs for wry/tao as outlined by tauri:
    ///     <https://tauri.app/start/prerequisites/>
    async fn verify_linux_tooling(&self) -> Result<()> {
        Ok(())
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

    fn print_linker_warnings(&self, exe_output_location: &Option<PathBuf>) {
        if let Ok(linker_warnings) = std::fs::read_to_string(self.link_err_file()) {
            if !linker_warnings.is_empty() {
                if exe_output_location.is_none() {
                    tracing::error!("Linker warnings: {}", linker_warnings);
                } else {
                    tracing::debug!("Linker warnings: {}", linker_warnings);
                }
            }
        }
    }

    /// Load per-crate rustc args from the wrapper directory.
    ///
    /// Each workspace crate compiled through the wrapper has its own JSON file:
    /// - "{crate_name}.lib.json" (key: "{crate_name}.lib") for lib targets and
    /// - "{crate_name}.bin.json" (key: "{crate_name}.bin") for bin targets.
    fn load_rustc_argset(&self) -> Result<WorkspaceRustcArgs> {
        let link_args = std::fs::read_to_string(self.link_args_file())
            .context("Failed to read link args from file")?
            .lines()
            .map(|s| s.to_string())
            .collect();

        let mut workspace_rustc_args = WorkspaceRustcArgs::new(link_args);

        // Always read from the fat build's scope dir — the rustc wrapper only captures
        // args during fat/base builds, not thin builds.
        let args_dir = self.rustc_wrapper_args_scope_dir(&BuildMode::Fat)?;
        if let Ok(entries) = std::fs::read_dir(&args_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().is_some_and(|e| e == "json") {
                    if let Ok(contents) = std::fs::read_to_string(&path) {
                        if let Ok(args) = serde_json::from_str::<RustcArgs>(&contents) {
                            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                                workspace_rustc_args
                                    .rustc_args
                                    .insert(stem.to_string(), args);
                            }
                        }
                    }
                }
            }
        }

        Ok(workspace_rustc_args)
    }

    pub(crate) fn all_target_features(&self) -> Vec<String> {
        let mut features = self.features.clone();
        features.dedup();
        features
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
            profile.strip.as_ref().copied().or_else(|| {
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

    pub(crate) fn platform_exe_name(&self) -> String {
        match self.bundle {
            // mac/ios are unixy and dont have an exe extension
            BundleFormat::MacOS | BundleFormat::Ios => self.executable_name().to_string(),

            // the server binary is always called "server" to avoid antivirus issues when the
            // binary name changes between builds (the folder name already identifies the project)
            BundleFormat::Server => match self.triple.operating_system {
                OperatingSystem::Windows => "server.exe".to_string(),
                _ => "server".to_string(),
            },

            BundleFormat::Windows => match self.triple.operating_system {
                OperatingSystem::Windows => format!("{}.exe", self.executable_name()),
                _ => self.executable_name().to_string(),
            },

            // from the apk spec, the root exe is a shared library
            // defaults to "main" (libmain.so) per NativeActivity convention, overridable in Dioxus.toml
            BundleFormat::Android => {
                let lib_name = self.android_lib_name();
                format!("lib{lib_name}.so")
            }

            // this will be wrong, I think, but not important?
            BundleFormat::Web => format!("{}_bg.wasm", self.executable_name()),

            // todo: maybe this should be called AppRun?
            BundleFormat::Linux => self.executable_name().to_string(),
        }
    }

    /// Get the directory where this app can write to for this session that's guaranteed to be stable
    /// for the same app. This is useful for emitting state like window position and size.
    ///
    /// The directory is specific for this app and might be
    pub(crate) fn session_cache_dir(&self) -> PathBuf {
        self.session_cache_dir.join(self.bundle.to_string())
    }

    pub(crate) fn rustc_wrapper_args_dir(&self) -> PathBuf {
        self.target_dir.join("dx").join(".captured-args")
    }

    pub(crate) fn rustc_wrapper_args_scope_dir(&self, build_mode: &BuildMode) -> Result<PathBuf> {
        Ok(self
            .rustc_wrapper_args_dir()
            .join(self.rustc_wrapper_scope_dir_name(build_mode)?))
    }

    /// The crate name that rustc uses for the tip crate (hyphens replaced with underscores).
    pub(crate) fn tip_crate_name(&self) -> String {
        self.main_target.replace('-', "_")
    }

    /// Stderr captured from the linker during the last build. Written by the linker
    /// interception in `rustcwrapper` and read back to surface warnings/errors to the user.
    fn link_err_file(&self) -> PathBuf {
        self.session_cache_dir().join("link_err.txt")
    }

    /// The linker arguments captured from the tip crate's final link invocation.
    /// Used to replay the link step during thin (hotpatch) builds.
    fn link_args_file(&self) -> PathBuf {
        self.session_cache_dir().join("link_args.json")
    }

    /// A response file for MSVC's `link.exe`. Windows command lines have a ~32k character
    /// limit, so we write linker arguments to this file and pass `@<path>` instead.
    pub(crate) fn windows_command_file(&self) -> PathBuf {
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

    pub(crate) fn bundled_app_name(&self) -> String {
        use convert_case::{Case, Casing};
        self.executable_name().to_case(Case::Pascal)
    }

    /// Get the crate version from Cargo.toml (e.g., "0.1.0")
    pub(crate) fn crate_version(&self) -> String {
        self.workspace.krates[self.crate_package]
            .version
            .to_string()
    }

    pub(crate) fn bundle_identifier(&self) -> String {
        use crate::config::BundlePlatform;

        // Check platform-specific identifier override first, then fall back to base bundle
        let platform: BundlePlatform = self.bundle.into();
        if let Some(identifier) = self.config.resolved_identifier(platform) {
            let identifier = identifier.to_string();
            if identifier.contains('.')
                && !identifier.starts_with('.')
                && !identifier.ends_with('.')
                && !identifier.contains("..")
            {
                return identifier;
            } else {
                tracing::error!(
                    "Invalid bundle identifier: {identifier:?}. Must contain at least one '.' and not start/end with '.'. E.g. `com.example.app`"
                );
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

    pub(crate) fn is_wasm_or_wasi(&self) -> bool {
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

    pub(crate) fn bundle_asset_dir(&self) -> PathBuf {
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

    /// Get the path to the app manifest file
    ///
    /// This includes metadata about the build such as the bundle format, target triple, features, etc.
    /// Manifests are only written by the `PRIMARY` build.
    fn bundle_manifest_file(&self) -> PathBuf {
        self.platform_dir().join(".manifest.json")
    }

    /// Blow away the fingerprint for this package, forcing rustc to recompile it.
    ///
    /// This prevents rustc from using the cached version of the binary, which can cause issues
    /// Find workspace crates that directly depend on the given crate.
    ///
    /// Returns underscore-normalized crate names of workspace members that have `crate_name`
    /// as a dependency. Used for cascade detection — when a dep's public symbols change,
    /// its dependents need recompilation too.
    pub(crate) fn workspace_dependents_of(&self, crate_name: &str) -> Vec<String> {
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

    /// Get the folder where Apple Widget Extensions (.appex bundles) are installed.
    /// This is only applicable to iOS and macOS bundles.
    pub(crate) fn plugins_folder(&self) -> PathBuf {
        match self.triple.operating_system {
            OperatingSystem::Darwin(_) | OperatingSystem::MacOSX(_) => {
                self.root_dir().join("Contents").join("PlugIns")
            }
            OperatingSystem::IOS(_) => self.root_dir().join("PlugIns"),
            _ => self.root_dir().join("PlugIns"),
        }
    }

    pub(crate) fn frameworks_folder(&self) -> PathBuf {
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
            BundleFormat::Ios => self.start_ios_sim().await?,

            BundleFormat::Android => self.start_android_sim()?,

            // nothing - maybe on the web we should open the browser?
            _ => {}
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

    fn cargo_fingerprint_dir(&self) -> PathBuf {
        self.target_dir
            .join(self.triple.to_string())
            .join(&self.profile)
            .join(".fingerprint")
    }

    fn is_apple_target(&self) -> bool {
        matches!(
            self.triple.operating_system,
            OperatingSystem::Darwin(_) | OperatingSystem::IOS(_)
        )
    }

    /// All workspace crate names that the tip crate transitively depends on
    /// (underscore-normalized, excluding the tip itself).
    fn workspace_crate_dep_names(&self) -> Vec<String> {
        let krates = &self.workspace.krates;

        let workspace_names: HashSet<String> = krates
            .workspace_members()
            .filter_map(|m| match m {
                krates::Node::Krate { krate, .. } => Some(krate.name.replace('-', "_")),
                _ => None,
            })
            .collect();

        let tip = self.tip_crate_name();
        let Some(tip_nid) = krates.workspace_members().find_map(|m| match m {
            krates::Node::Krate { id, krate, .. } if krate.name.replace('-', "_") == tip => {
                krates.nid_for_kid(id)
            }
            _ => None,
        }) else {
            return Vec::new();
        };

        // BFS forward through workspace deps.
        let mut visited = HashSet::new();
        let mut queue = VecDeque::from([tip_nid]);
        let mut deps = Vec::new();

        while let Some(nid) = queue.pop_front() {
            for dep in krates.direct_dependencies(nid) {
                let name = dep.krate.name.replace('-', "_");
                if workspace_names.contains(&name) && visited.insert(dep.node_id) {
                    deps.push(name);
                    queue.push_back(dep.node_id);
                }
            }
        }

        deps
    }
}
