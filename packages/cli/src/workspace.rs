use crate::styles::GLOW_STYLE;
use crate::CliSettings;
use crate::Result;
use crate::{config::DioxusConfig, AndroidTools};
use anyhow::{bail, Context};
use ignore::gitignore::Gitignore;
use krates::{semver::Version, KrateDetails, LockOptions};
use krates::{Cmd, Krates, NodeId};
use std::sync::Arc;
use std::{collections::HashSet, path::Path};
use std::{path::PathBuf, time::Duration};
use target_lexicon::Triple;
use tokio::process::Command;

pub struct Workspace {
    pub(crate) krates: Krates,
    pub(crate) settings: CliSettings,
    pub(crate) wasm_opt: Option<PathBuf>,
    pub(crate) sysroot: PathBuf,
    pub(crate) rustc_version: String,
    pub(crate) ignore: Gitignore,
    pub(crate) cargo_toml: cargo_toml::Manifest,
    pub(crate) android_tools: Option<Arc<AndroidTools>>,
}

impl Workspace {
    /// Load the workspace from the current directory. This is cached and will only be loaded once.
    pub async fn current() -> Result<Arc<Workspace>> {
        static WS: tokio::sync::Mutex<Option<Arc<Workspace>>> = tokio::sync::Mutex::const_new(None);

        // Lock the workspace to prevent multiple threads from loading it at the same time
        // If loading the workspace failed the first time, it won't be set and therefore permeate an error.
        let mut lock = WS.lock().await;
        if let Some(ws) = lock.as_ref() {
            return Ok(ws.clone());
        }

        let krates_future = tokio::task::spawn_blocking(|| {
            let manifest_options = crate::logging::VERBOSITY.get().unwrap();
            let lock_options = LockOptions {
                frozen: manifest_options.frozen,
                locked: manifest_options.locked,
                offline: manifest_options.offline,
            };

            let mut cmd = Cmd::new();
            cmd.lock_opts(lock_options);

            let mut builder = krates::Builder::new();
            builder.workspace(true);
            let res = builder.build(cmd, |_| {})?;

            if !lock_options.offline {
                if let Ok(res) = std::env::var("SIMULATE_SLOW_NETWORK") {
                    std::thread::sleep(Duration::from_secs(res.parse().unwrap_or(5)));
                }
            }

            Ok(res) as Result<Krates, krates::Error>
        });

        let spin_future = async move {
            tokio::time::sleep(Duration::from_millis(1000)).await;
            eprintln!("{GLOW_STYLE}warning{GLOW_STYLE:#}: Waiting for cargo-metadata...");
            tokio::time::sleep(Duration::from_millis(2000)).await;
            for x in 1..=100 {
                tokio::time::sleep(Duration::from_millis(2000)).await;
                eprintln!("{GLOW_STYLE}warning{GLOW_STYLE:#}: (Try {x}) Taking a while...");

                if x % 10 == 0 {
                    eprintln!("{GLOW_STYLE}warning{GLOW_STYLE:#}: maybe check your network connection or build lock?");
                }
            }
        };

        let krates = tokio::select! {
            f = krates_future => {
                let res = f?;
                if let Err(krates::Error::Metadata(e)) = res {
                    bail!("{e}");
                }
                res?
            },
            _ = spin_future => bail!("cargo metadata took too long to respond, try again with --offline"),
        };

        let settings = CliSettings::global_or_default();
        let sysroot = Self::get_rustc_sysroot()
            .await
            .context("Failed to get rustc sysroot")?;
        let rustc_version = Self::get_rustc_version()
            .await
            .context("Failed to get rustc version")?;

        let wasm_opt = which::which("wasm-opt").ok();

        let ignore = Self::workspace_gitignore(krates.workspace_root().as_std_path());

        let cargo_toml = crate::cargo_toml::load_manifest_from_path(
            krates.workspace_root().join("Cargo.toml").as_std_path(),
        )
        .context("Failed to load Cargo.toml")?;

        let android_tools = crate::build::get_android_tools();

        let workspace = Arc::new(Self {
            krates,
            settings,
            wasm_opt,
            sysroot: sysroot.trim().into(),
            rustc_version: rustc_version.trim().into(),
            ignore,
            cargo_toml,
            android_tools,
        });

        tracing::debug!(
            r#"Initialized workspace:
               • sysroot: {sysroot}
               • rustc version: {rustc_version}
               • workspace root: {workspace_root}
               • dioxus versions: [{dioxus_versions:?}]"#,
            sysroot = workspace.sysroot.display(),
            rustc_version = workspace.rustc_version,
            workspace_root = workspace.workspace_root().display(),
            dioxus_versions = workspace
                .dioxus_versions()
                .iter()
                .map(|v| v.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        );

        lock.replace(workspace.clone());

        Ok(workspace)
    }

    pub fn android_tools(&self) -> Result<Arc<AndroidTools>> {
        self
            .android_tools
            .clone()
            .context("Android not installed properly. Please set the `ANDROID_NDK_HOME` environment variable to the root of your NDK installation.")
    }

    pub fn is_release_profile(&self, profile: &str) -> bool {
        // If the profile is "release" or ends with "-release" like the default platform release profiles,
        // always put it in the release category.
        if profile == "release" || profile.ends_with("-release") {
            return true;
        }

        // Check if the profile inherits from release by traversing the `inherits` chain
        let mut current_profile_name = profile;

        // Try to find the current profile in the custom profiles section
        while let Some(profile_settings) = self.cargo_toml.profile.custom.get(current_profile_name)
        {
            // Check what this profile inherits from
            match &profile_settings.inherits {
                // Otherwise, continue checking the profile it inherits from
                Some(inherits_name) => current_profile_name = inherits_name,

                // This profile doesn't explicitly inherit anything, so the chain ends here.
                // Since it didn't lead to "release", return false.
                None => break,
            }

            if current_profile_name == "release" {
                return true;
            }
        }

        false
    }

    pub fn check_dioxus_version_against_cli(&self) {
        let dx_semver = Version::parse(env!("CARGO_PKG_VERSION")).unwrap();
        let dioxus_versions = self.dioxus_versions();

        tracing::trace!("dx version: {}", dx_semver);
        tracing::trace!("dioxus versions: {:?}", dioxus_versions);

        // if there are no dioxus versions in the workspace, we don't need to check anything
        // dx is meant to be compatible with non-dioxus projects too.
        if dioxus_versions.is_empty() {
            return;
        }

        let min = dioxus_versions.iter().min().unwrap();
        let max = dioxus_versions.iter().max().unwrap();

        // If the minimum dioxus version is greater than the current cli version, warn the user
        if min > &dx_semver
            || max < &dx_semver
            || dioxus_versions.iter().any(|f| f.pre != dx_semver.pre)
        {
            tracing::error!(
                r#"🚫dx and dioxus versions are incompatible!
                  • dx version: {dx_semver}
                  • dioxus versions: [{}]"#,
                dioxus_versions
                    .iter()
                    .map(|v| v.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }
    }

    /// Get all the versions of dioxus in the workspace
    pub fn dioxus_versions(&self) -> Vec<Version> {
        let mut versions = HashSet::new();
        for krate in self.krates.krates() {
            if krate.name == "dioxus" {
                versions.insert(krate.version.clone());
            }
        }
        let mut versions = versions.into_iter().collect::<Vec<_>>();
        versions.sort();
        versions
    }

    #[allow(unused)]
    pub fn rust_lld(&self) -> PathBuf {
        self.sysroot
            .join("lib")
            .join("rustlib")
            .join(Triple::host().to_string())
            .join("bin")
            .join("rust-lld")
    }

    /// Return the path to the `cc` compiler
    ///
    /// This is used for the patching system to run the linker.
    /// We could also just use lld given to us by rust itself.
    pub fn cc(&self) -> PathBuf {
        PathBuf::from("cc")
    }

    /// The windows linker
    pub fn lld_link(&self) -> PathBuf {
        self.gcc_ld_dir().join("lld-link")
    }

    pub fn wasm_ld(&self) -> PathBuf {
        self.gcc_ld_dir().join("wasm-ld")
    }

    pub fn select_ranlib() -> Option<PathBuf> {
        // prefer the modern llvm-ranlib if they have it
        which::which("llvm-ranlib")
            .or_else(|_| which::which("ranlib"))
            .ok()
    }

    /// Return the version of the wasm-bindgen crate if it exists
    pub fn wasm_bindgen_version(&self) -> Option<String> {
        self.krates
            .krates_by_name("wasm-bindgen")
            .next()
            .map(|krate| krate.krate.version.to_string())
    }

    // wasm-ld: ./rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/wasm-ld
    // rust-lld: ./rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rust-lld
    fn gcc_ld_dir(&self) -> PathBuf {
        self.sysroot
            .join("lib")
            .join("rustlib")
            .join(Triple::host().to_string())
            .join("bin")
            .join("gcc-ld")
    }

    // wasm-ld: ./rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/wasm-ld
    // rust-lld: ./rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rust-lld
    pub fn rustc_objcopy(&self) -> PathBuf {
        self.sysroot
            .join("lib")
            .join("rustlib")
            .join(Triple::host().to_string())
            .join("bin")
            .join("rust-objcopy")
    }

    // ./rustup/toolchains/nightly-x86_64-unknown-linux-gnu/lib
    pub fn rustc_objcopy_dylib_path(&self) -> PathBuf {
        self.sysroot.join("lib")
    }

    /// Find the "main" package in the workspace. There might not be one!
    pub fn find_main_package(&self, package: Option<String>) -> Result<NodeId> {
        if let Some(package) = package {
            let mut workspace_members = self.krates.workspace_members();
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
                for package in self.krates.workspace_members() {
                    if let krates::Node::Krate { krate, .. } = package {
                        tracing::error!("{}", krate.name());
                    }
                }
            }

            let kid = found.ok_or_else(|| anyhow::anyhow!("Failed to find package {package}"))?;

            return Ok(self.krates.nid_for_kid(kid).unwrap());
        };

        // if we have default members specified, try them first
        if let Some(ws) = &self.cargo_toml.workspace {
            for default in &ws.default_members {
                let mut workspace_members = self.krates.workspace_members();
                if let Ok(default_member_path) = std::fs::canonicalize(default) {
                    let found = workspace_members.find_map(|node| {
                        if let krates::Node::Krate { id, krate, .. } = node {
                            // Skip this default member if it doesn't have any binary targets
                            if !krate
                                .targets
                                .iter()
                                .any(|t| t.kind.contains(&krates::cm::TargetKind::Bin))
                            {
                                return None;
                            }
                            if std::fs::canonicalize(krate.manifest_path.parent().unwrap())
                                .is_ok_and(|member_path| member_path == default_member_path)
                            {
                                return Some(id);
                            }
                        }
                        None
                    });

                    if let Some(kid) = found {
                        return Ok(self.krates.nid_for_kid(kid).unwrap());
                    }
                }
            }
        }

        // Otherwise find the package that is the closest parent of the current directory
        let current_dir = std::env::current_dir()?;
        let current_dir = current_dir.as_path();

        // Go through each member and find the path that is a parent of the current directory
        let mut closest_parent = None;
        for member in self.krates.workspace_members() {
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
            let bin_targets = self.krates.workspace_members().filter_map(|krate|match krate {
                krates::Node::Krate { krate, .. } if krate.targets.iter().any(|t| t.kind.contains(&krates::cm::TargetKind::Bin))=> {
                    Some(format!("- {}", krate.name))
                }
                _ => None
            }).collect::<Vec<_>>();
            format!("Failed to find binary package to build.\nYou need to either run dx from inside a binary crate or specify a binary package to build with the `--package` flag. Try building again with one of the binary packages in the workspace:\n{}", bin_targets.join("\n"))
        })?;

        let package = self.krates.nid_for_kid(kid).unwrap();
        Ok(package)
    }

    /// Load the Dioxus.toml configuration for a package.
    ///
    /// Optionally accepts a source file path to extract inline configuration from doc comments.
    /// Inline config is merged with the base Dioxus.toml, with inline values taking precedence.
    ///
    /// This allows examples and binaries to embed config in their doc comments:
    /// ```rust,ignore
    /// //! ```dioxus.toml
    /// //! [bundle]
    /// //! identifier = "com.example.app"
    /// //! ```
    /// ```
    pub fn load_dioxus_config(
        &self,
        package: NodeId,
        source_file: Option<&Path>,
    ) -> Result<Option<DioxusConfig>> {
        // Walk up from the cargo.toml to the root of the workspace looking for Dioxus.toml
        let mut current_dir = self.krates[package]
            .manifest_path
            .parent()
            .unwrap()
            .as_std_path()
            .to_path_buf()
            .canonicalize()?;

        let workspace_path = self
            .krates
            .workspace_root()
            .as_std_path()
            .to_path_buf()
            .canonicalize()?;

        let mut dioxus_conf_file = None;
        while current_dir.starts_with(&workspace_path) {
            let config = ["Dioxus.toml", "dioxus.toml"]
                .into_iter()
                .map(|file| current_dir.join(file))
                .find(|path| path.is_file());

            // Try to find Dioxus.toml in the current directory
            if let Some(new_config) = config {
                dioxus_conf_file = Some(new_config.as_path().to_path_buf());
                break;
            }
            // If we can't find it, go up a directory
            current_dir = current_dir
                .parent()
                .context("Failed to find Dioxus.toml")?
                .to_path_buf();
        }

        // Load base config from Dioxus.toml (if it exists)
        let base_config: Option<DioxusConfig> = match &dioxus_conf_file {
            Some(path) => {
                let content = std::fs::read_to_string(path)?;
                Some(toml::from_str(&content).map_err(|err| {
                    anyhow::anyhow!("Failed to parse Dioxus.toml at {path:?}: {err}")
                })?)
            }
            None => None,
        };

        // Extract inline config from source file (if provided)
        let inline_config = source_file.and_then(crate::config::extract_inline_config_from_file);

        // Merge configs: inline overrides base
        match (base_config, inline_config) {
            (Some(base), Some(inline)) => crate::config::merge_with_inline_config(&base, inline)
                .map(Some)
                .map_err(|err| anyhow::anyhow!("Failed to merge inline config: {err}")),
            (Some(base), None) => Ok(Some(base)),
            (None, Some(inline)) => {
                // No Dioxus.toml, but we have inline config - use defaults + inline
                let base = DioxusConfig::default();
                crate::config::merge_with_inline_config(&base, inline)
                    .map(Some)
                    .map_err(|err| anyhow::anyhow!("Failed to merge inline config: {err}"))
            }
            (None, None) => Ok(None),
        }
    }

    /// Create a new gitignore map for this target crate
    ///
    /// todo(jon): this is a bit expensive to build, so maybe we should cache it?
    pub fn workspace_gitignore(workspace_dir: &Path) -> Gitignore {
        let mut ignore_builder = ignore::gitignore::GitignoreBuilder::new(workspace_dir);
        ignore_builder.add(workspace_dir.join(".gitignore"));

        for path in Self::default_ignore_list() {
            ignore_builder
                .add_line(None, path)
                .expect("failed to add path to file excluded");
        }

        ignore_builder.build().unwrap()
    }

    pub fn ignore_for_krate(&self, path: &Path) -> ignore::gitignore::Gitignore {
        let mut ignore_builder = ignore::gitignore::GitignoreBuilder::new(path);
        for path in Self::default_ignore_list() {
            ignore_builder
                .add_line(None, path)
                .expect("failed to add path to file excluded");
        }
        ignore_builder.build().unwrap()
    }

    pub fn default_ignore_list() -> Vec<&'static str> {
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

    pub(crate) fn workspace_root(&self) -> PathBuf {
        self.krates.workspace_root().as_std_path().to_path_buf()
    }

    /// Returns the root of the crate that the command is run from, without calling `cargo metadata`
    ///
    /// If the command is run from the workspace root, this will return the top-level Cargo.toml
    pub(crate) fn crate_root_from_path() -> Result<PathBuf> {
        /// How many parent folders are searched for a `Cargo.toml`
        const MAX_ANCESTORS: u32 = 10;

        /// Checks if the directory contains `Cargo.toml`
        fn contains_manifest(path: &Path) -> bool {
            std::fs::read_dir(path)
                .map(|entries| {
                    entries
                        .filter_map(Result::ok)
                        .any(|ent| &ent.file_name() == "Cargo.toml")
                })
                .unwrap_or(false)
        }

        // From the current directory we work our way up, looking for `Cargo.toml`
        std::env::current_dir()
            .ok()
            .and_then(|mut wd| {
                for _ in 0..MAX_ANCESTORS {
                    if contains_manifest(&wd) {
                        return Some(wd);
                    }
                    if !wd.pop() {
                        break;
                    }
                }
                None
            })
            .context("Failed to find directory containing Cargo.toml")
    }

    pub async fn get_xcode_path() -> Option<PathBuf> {
        let xcode = Command::new("xcode-select")
            .arg("-p")
            .output()
            .await
            .ok()
            .map(|s| String::from_utf8_lossy(&s.stdout).trim().to_string().into());
        xcode
    }

    pub async fn get_rustc_sysroot() -> Result<String, anyhow::Error> {
        let sysroot = Command::new("rustc")
            .args(["--print", "sysroot"])
            .output()
            .await
            .map(|out| String::from_utf8(out.stdout).map(|s| s.trim().to_string()))?
            .context("Failed to extract rustc sysroot output")?;
        Ok(sysroot)
    }

    pub async fn get_rustc_version() -> Result<String> {
        let rustc_version = Command::new("rustc")
            .args(["--version"])
            .output()
            .await
            .map(|out| String::from_utf8(out.stdout))?
            .context("Failed to extract rustc version output")?;
        Ok(rustc_version)
    }

    /// Returns the properly canonicalized path to the dx executable, used for linking and wrapping rustc
    pub(crate) fn path_to_dx() -> Result<PathBuf> {
        dunce::canonicalize(std::env::current_exe().context("Failed to find dx")?)
            .context("Failed to find dx")
    }

    /// Returns the path to the dioxus data directory, used to install tools, store configs, and other things
    ///
    /// On macOS, we prefer to not put this dir in Application Support, but rather in the home directory.
    /// On Windows, we prefer to keep it in the home directory so the `dx` install dir matches the install script.
    pub(crate) fn dioxus_data_dir() -> PathBuf {
        static DX_HOME: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
        DX_HOME
            .get_or_init(|| {
                if let Some(path) = std::env::var_os("DX_HOME") {
                    return PathBuf::from(path);
                }

                if cfg!(target_os = "macos") || cfg!(target_os = "windows") {
                    dirs::home_dir().unwrap().join(".dx")
                } else {
                    dirs::data_dir()
                        .or_else(dirs::home_dir)
                        .unwrap()
                        .join(".dx")
                }
            })
            .to_path_buf()
    }

    pub(crate) fn global_settings_file() -> PathBuf {
        Self::dioxus_data_dir().join("settings.toml")
    }

    /// The path where components downloaded from git are cached
    pub(crate) fn component_cache_dir() -> PathBuf {
        Self::dioxus_data_dir().join("components")
    }

    /// Get the path to a specific component in the cache
    pub(crate) fn component_cache_path(git: &str, rev: Option<&str>) -> PathBuf {
        use std::hash::Hasher;

        let mut hasher = std::hash::DefaultHasher::new();
        std::hash::Hash::hash(git, &mut hasher);
        if let Some(rev) = rev {
            std::hash::Hash::hash(rev, &mut hasher);
        }
        let hash = hasher.finish();
        Self::component_cache_dir().join(format!("{hash:016x}"))
    }
}

impl std::fmt::Debug for Workspace {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Workspace")
            .field("krates", &"..")
            .field("settings", &self.settings)
            .field("rustc_version", &self.rustc_version)
            .field("sysroot", &self.sysroot)
            .field("wasm_opt", &self.wasm_opt)
            .finish()
    }
}
