use crate::{config::DioxusConfig, TargetArgs};
use crate::{Arch, CliSettings};
use crate::{Platform, Result};
use anyhow::Context;
use itertools::Itertools;
use krates::{cm::Target, KrateDetails};
use krates::{cm::TargetKind, Cmd, Krates, NodeId};
use once_cell::sync::OnceCell;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::process::Command;
use toml_edit::Item;

// Contains information about the crate we are currently in and the dioxus config for that crate
#[derive(Clone)]
pub(crate) struct DioxusCrate {
    pub(crate) krates: Arc<Krates>,
    pub(crate) package: NodeId,
    pub(crate) config: DioxusConfig,
    pub(crate) target: Target,
    pub(crate) settings: Arc<CliSettings>,
}

pub(crate) static PROFILE_WASM: &str = "wasm-dev";
pub(crate) static PROFILE_ANDROID: &str = "android-dev";
pub(crate) static PROFILE_SERVER: &str = "server-dev";

impl DioxusCrate {
    pub(crate) fn new(target: &TargetArgs) -> Result<Self> {
        tracing::debug!("Loading crate");
        let cmd = Cmd::new();
        let builder = krates::Builder::new();
        let krates = builder
            .build(cmd, |_| {})
            .context("Failed to run cargo metadata")?;

        let package = find_main_package(&krates, target.package.clone())?;
        tracing::debug!("Found package {package:?}");

        let dioxus_config = DioxusConfig::load(&krates, package)?.unwrap_or_default();

        let package_name = krates[package].name.clone();
        let target_kind = if target.example.is_some() {
            TargetKind::Example
        } else {
            TargetKind::Bin
        };

        let main_package = &krates[package];

        let target_name = target
            .example
            .clone()
            .or(target.bin.clone())
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
            .unwrap_or(package_name);

        let target = main_package
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
                if let Some(example) = &target.example {
                    let examples = target_of_kind(&TargetKind::Example);
                    format!("Failed to find example {example}. \nAvailable examples are:\n{}", examples)
                } else if let Some(bin) = &target.bin {
                    let binaries = target_of_kind(&TargetKind::Bin);
                    format!("Failed to find binary {bin}. \nAvailable binaries are:\n{}", binaries)
                } else {
                    format!("Failed to find target {target_name}. \nIt looks like you are trying to build dioxus in a library crate. \
                    You either need to run dx from inside a binary crate or build a specific example with the `--example` flag. \
                    Available examples are:\n{}", target_of_kind(&TargetKind::Example))
                }
            })?
            .clone();

        let settings = CliSettings::load();

        Ok(Self {
            krates: Arc::new(krates),
            package,
            config: dioxus_config,
            target,
            settings,
        })
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
        self.krates.workspace_root().as_std_path().to_path_buf()
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
        self.target.src_path.as_std_path().to_path_buf()
    }

    /// Get the package we are currently in
    pub(crate) fn package(&self) -> &krates::cm::Package {
        &self.krates[self.package]
    }

    /// Get the name of the package we are compiling
    pub(crate) fn executable_name(&self) -> &str {
        &self.target.name
    }

    /// Get the type of executable we are compiling
    pub(crate) fn executable_type(&self) -> krates::cm::TargetKind {
        self.target.kind[0].clone()
    }

    /// Try to autodetect the platform from the package by reading its features
    ///
    /// Read the default-features list and/or the features list on dioxus to see if we can autodetect the platform
    pub(crate) fn autodetect_platform(&self) -> Option<(Platform, String)> {
        let krate = self.krates.krates_by_name("dioxus").next()?;

        // We're going to accumulate the platforms that are enabled
        // This will let us create a better warning if multiple platforms are enabled
        let manually_enabled_platforms = self
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
        let possible_platforms = self
            .package()
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

    /// Check if dioxus is being built with a particular feature
    pub(crate) fn has_dioxus_feature(&self, filter: &str) -> bool {
        self.krates.krates_by_name("dioxus").any(|dioxus| {
            self.krates
                .get_enabled_features(dioxus.kid)
                .map(|features| features.contains(filter))
                .unwrap_or_default()
        })
    }

    /// Get the features required to build for the given platform
    pub(crate) fn feature_for_platform(&self, platform: Platform) -> String {
        let package = self.package();

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

    fn default_ignore_list(&self) -> Vec<&'static str> {
        vec![
            ".git",
            ".github",
            ".vscode",
            "target",
            "node_modules",
            "dist",
            "*~",
            ".*",
            "*.lock",
            "*.log",
        ]
    }

    /// Create a new gitignore map for this target crate
    ///
    /// todo(jon): this is a bit expensive to build, so maybe we should cache it?
    pub fn workspace_gitignore(&self) -> ignore::gitignore::Gitignore {
        let crate_dir = self.crate_dir();

        let mut ignore_builder = ignore::gitignore::GitignoreBuilder::new(&crate_dir);
        ignore_builder.add(crate_dir.join(".gitignore"));

        let workspace_dir = self.workspace_dir();
        ignore_builder.add(workspace_dir.join(".gitignore"));

        for path in self.default_ignore_list() {
            ignore_builder
                .add_line(None, path)
                .expect("failed to add path to file excluded");
        }

        ignore_builder.build().unwrap()
    }

    /// Return the version of the wasm-bindgen crate if it exists
    pub fn wasm_bindgen_version(&self) -> Option<String> {
        self.krates
            .krates_by_name("wasm-bindgen")
            .next()
            .map(|krate| krate.krate.version.to_string())
    }

    pub(crate) fn default_platform(&self) -> Option<Platform> {
        let default = self.package().features.get("default")?;

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
            let internal_feature = self.package().features.get(feature);
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
    pub(crate) fn platformless_features(&self) -> Vec<String> {
        let default = self.package().features.get("default").unwrap();
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
            if let Some(internal_feature) = self.package().features.get(feature) {
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

    /// Return the list of paths that we should watch for changes.
    pub(crate) fn watch_paths(&self) -> Vec<PathBuf> {
        let mut watched_paths = vec![];

        // Get a list of *all* the crates with Rust code that we need to watch.
        // This will end up being dependencies in the workspace and non-workspace dependencies on the user's computer.
        let mut watched_crates = self.local_dependencies();
        watched_crates.push(self.crate_dir());

        // Now, watch all the folders in the crates, but respecting their respective ignore files
        for krate_root in watched_crates {
            // Build the ignore builder for this crate, but with our default ignore list as well
            let ignore = self.ignore_for_krate(&krate_root);

            for entry in krate_root.read_dir().unwrap() {
                let Ok(entry) = entry else {
                    continue;
                };

                if ignore
                    .matched(entry.path(), entry.path().is_dir())
                    .is_ignore()
                {
                    continue;
                }

                watched_paths.push(entry.path().to_path_buf());
            }
        }

        watched_paths.dedup();

        watched_paths
    }

    fn ignore_for_krate(&self, path: &Path) -> ignore::gitignore::Gitignore {
        let mut ignore_builder = ignore::gitignore::GitignoreBuilder::new(path);
        for path in self.default_ignore_list() {
            ignore_builder
                .add_line(None, path)
                .expect("failed to add path to file excluded");
        }
        ignore_builder.build().unwrap()
    }

    /// Get all the Manifest paths for dependencies that we should watch. Will not return anything
    /// in the `.cargo` folder - only local dependencies will be watched.
    ///
    /// This returns a list of manifest paths
    ///
    /// Extend the watch path to include:
    ///
    /// - the assets directory - this is so we can hotreload CSS and other assets by default
    /// - the Cargo.toml file - this is so we can hotreload the project if the user changes dependencies
    /// - the Dioxus.toml file - this is so we can hotreload the project if the user changes the Dioxus config
    pub(crate) fn local_dependencies(&self) -> Vec<PathBuf> {
        let mut paths = vec![];

        for (dependency, _edge) in self.krates.get_deps(self.package) {
            let krate = match dependency {
                krates::Node::Krate { krate, .. } => krate,
                krates::Node::Feature { krate_index, .. } => &self.krates[krate_index.index()],
            };

            if krate
                .manifest_path
                .components()
                .any(|c| c.as_str() == ".cargo")
            {
                continue;
            }

            paths.push(
                krate
                    .manifest_path
                    .parent()
                    .unwrap()
                    .to_path_buf()
                    .into_std_path_buf(),
            );
        }

        paths
    }

    pub(crate) fn all_watched_crates(&self) -> Vec<PathBuf> {
        let mut krates: Vec<PathBuf> = self
            .local_dependencies()
            .into_iter()
            .map(|p| {
                p.parent()
                    .expect("Local manifest to exist and have a parent")
                    .to_path_buf()
            })
            .chain(Some(self.crate_dir()))
            .collect();

        krates.dedup();

        krates
    }

    /// Attempt to retrieve the path to ADB
    pub(crate) fn android_adb() -> PathBuf {
        static PATH: once_cell::sync::Lazy<PathBuf> = once_cell::sync::Lazy::new(|| {
            let Some(sdk) = DioxusCrate::android_sdk() else {
                return PathBuf::from("adb");
            };

            let tools = sdk.join("platform-tools");

            if tools.join("adb").exists() {
                return tools.join("adb");
            }

            if tools.join("adb.exe").exists() {
                return tools.join("adb.exe");
            }

            PathBuf::from("adb")
        });

        PATH.clone()
    }

    pub(crate) fn android_sdk() -> Option<PathBuf> {
        var_or_debug("ANDROID_SDK_ROOT")
            .or_else(|| var_or_debug("ANDROID_SDK"))
            .or_else(|| var_or_debug("ANDROID_HOME"))
    }

    pub(crate) fn android_ndk(&self) -> Option<PathBuf> {
        // "/Users/jonkelley/Library/Android/sdk/ndk/25.2.9519653/toolchains/llvm/prebuilt/darwin-x86_64/bin/aarch64-linux-android24-clang"
        static PATH: once_cell::sync::Lazy<Option<PathBuf>> = once_cell::sync::Lazy::new(|| {
            // attempt to autodetect the ndk path from env vars (usually set by the shell)
            let auto_detected_ndk =
                var_or_debug("NDK_HOME").or_else(|| var_or_debug("ANDROID_NDK_HOME"));

            if let Some(home) = auto_detected_ndk {
                return Some(home);
            }

            let sdk = var_or_debug("ANDROID_SDK_ROOT")
                .or_else(|| var_or_debug("ANDROID_SDK"))
                .or_else(|| var_or_debug("ANDROID_HOME"))?;

            let ndk = sdk.join("ndk");

            ndk.read_dir()
                .ok()?
                .flatten()
                .map(|dir| (dir.file_name(), dir.path()))
                .sorted()
                .last()
                .map(|(_, path)| path.to_path_buf())
        });

        PATH.clone()
    }

    pub(crate) async fn autodetect_android_arch() -> Option<Arch> {
        // Try auto detecting arch through adb.
        static AUTO_ARCH: OnceCell<Option<Arch>> = OnceCell::new();

        match AUTO_ARCH.get() {
            Some(a) => *a,
            None => {
                // TODO: Wire this up with --device flag. (add `-s serial`` flag before `shell` arg)
                let output = Command::new("adb")
                    .arg("shell")
                    .arg("uname")
                    .arg("-m")
                    .output()
                    .await;

                let out = match output {
                    Ok(o) => o,
                    Err(e) => {
                        tracing::debug!("ADB command failed: {:?}", e);
                        return None;
                    }
                };

                // Parse ADB output
                let Ok(out) = String::from_utf8(out.stdout) else {
                    tracing::debug!("ADB returned unexpected data.");
                    return None;
                };
                let trimmed = out.trim().to_string();
                tracing::trace!("ADB Returned: `{trimmed:?}`");

                // Set the cell
                let arch = Arch::try_from(trimmed).ok();
                AUTO_ARCH
                    .set(arch)
                    .expect("the cell should have been checked empty by the match condition");

                arch
            }
        }
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
}

impl std::fmt::Debug for DioxusCrate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DioxusCrate")
            .field("package", &self.krates[self.package])
            .field("dioxus_config", &self.config)
            .field("target", &self.target)
            .finish()
    }
}

// Find the main package in the workspace
fn find_main_package(krates: &Krates, package: Option<String>) -> Result<NodeId> {
    if let Some(package) = package {
        let mut workspace_members = krates.workspace_members();
        let found = workspace_members.find_map(|node| {
            if let krates::Node::Krate { id, krate, .. } = node {
                if krate.name == package {
                    return Some(id);
                }
            }
            None
        });

        if found.is_none() {
            tracing::error!("Could not find package {package} in the workspace. Did you forget to add it to the workspace?");
            tracing::error!("Packages in the workspace:");
            for package in krates.workspace_members() {
                if let krates::Node::Krate { krate, .. } = package {
                    tracing::error!("{}", krate.name());
                }
            }
        }

        let kid = found.ok_or_else(|| anyhow::anyhow!("Failed to find package {package}"))?;

        return Ok(krates.nid_for_kid(kid).unwrap());
    };

    // Otherwise find the package that is the closest parent of the current directory
    let current_dir = std::env::current_dir()?;
    let current_dir = current_dir.as_path();

    // Go through each member and find the path that is a parent of the current directory
    let mut closest_parent = None;
    for member in krates.workspace_members() {
        if let krates::Node::Krate { id, krate, .. } = member {
            let member_path = krate.manifest_path.parent().unwrap();
            if let Ok(path) = current_dir.strip_prefix(member_path.as_std_path()) {
                let len = path.components().count();
                match closest_parent {
                    Some((_, closest_parent_len)) => {
                        if len < closest_parent_len {
                            closest_parent = Some((id, len));
                        }
                    }
                    None => {
                        closest_parent = Some((id, len));
                    }
                }
            }
        }
    }

    let kid = closest_parent
        .map(|(id, _)| id)
        .with_context(|| {
            let bin_targets = krates.workspace_members().filter_map(|krate|match krate {
                krates::Node::Krate { krate, .. } if krate.targets.iter().any(|t| t.kind.contains(&krates::cm::TargetKind::Bin))=> {
                    Some(format!("- {}", krate.name))
                }
                _ => None
            }).collect::<Vec<_>>();
            format!("Failed to find binary package to build.\nYou need to either run dx from inside a binary crate or specify a binary package to build with the `--package` flag. Try building again with one of the binary packages in the workspace:\n{}", bin_targets.join("\n"))
        })?;

    let package = krates.nid_for_kid(kid).unwrap();
    Ok(package)
}

fn var_or_debug(name: &str) -> Option<PathBuf> {
    use std::env::var;
    use tracing::debug;

    var(name)
        .inspect_err(|_| debug!("{name} not set"))
        .ok()
        .map(PathBuf::from)
}
