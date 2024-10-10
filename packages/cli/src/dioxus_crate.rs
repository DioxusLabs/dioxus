use crate::CliSettings;
use crate::{config::DioxusConfig, TargetArgs};
use crate::{Platform, Result};
use anyhow::Context;
use krates::{cm::Target, KrateDetails};
use krates::{cm::TargetKind, Cmd, Krates, NodeId};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;
use toml_edit::Item;

// Contains information about the crate we are currently in and the dioxus config for that crate
#[derive(Clone)]
pub(crate) struct DioxusCrate {
    pub(crate) krates: Arc<Krates>,
    pub(crate) package: NodeId,
    pub(crate) config: DioxusConfig,
    pub(crate) target: Target,
    pub(crate) settings: CliSettings,
}

pub(crate) static PROFILE_WASM: &str = "dioxus-wasm";
pub(crate) static PROFILE_ANDROID: &str = "dioxus-android";
pub(crate) static PROFILE_SERVER: &str = "dioxus-server";

impl DioxusCrate {
    pub(crate) fn new(target: &TargetArgs) -> Result<Self> {
        let mut cmd = Cmd::new();
        cmd.features(target.features.clone());
        let builder = krates::Builder::new();
        let krates = builder
            .build(cmd, |_| {})
            .context("Failed to run cargo metadata")?;

        let package = find_main_package(target.package.clone(), &krates)?;

        let dioxus_config = DioxusConfig::load(&krates, package)?.unwrap_or_default();

        let package_name = krates[package].name.clone();
        let target_kind = if target.example.is_some() {
            TargetKind::Example
        } else {
            TargetKind::Bin
        };

        let target_name = target
            .example
            .clone()
            .or(target.bin.clone())
            .unwrap_or(package_name);

        let main_package = &krates[package];
        let target = main_package
            .targets
            .iter()
            .find(|target| {
                target_name == target.name.as_str() && target.kind.contains(&target_kind)
            })
            .with_context(|| format!("Failed to find target {target_name}"))?
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

    /// Compose an asset directory. Represents the typical "public" directory
    /// with publicly available resources (configurable in the `Dioxus.toml`).
    pub(crate) fn legacy_asset_dir(&self) -> PathBuf {
        self.crate_dir().join(&self.config.application.asset_dir)
    }

    /// Get the list of files in the "legacy" asset directory
    pub(crate) fn legacy_asset_dir_files(&self) -> Vec<PathBuf> {
        let mut files = vec![];

        let Ok(read_dir) = self.legacy_asset_dir().read_dir() else {
            return files;
        };

        for entry in read_dir {
            if let Ok(entry) = entry {
                files.push(entry.path());
            }
        }

        files
    }

    /// Compose an out directory. Represents the typical "dist" directory that
    /// is "distributed" after building an application (configurable in the
    /// `Dioxus.toml`).
    fn out_dir(&self) -> PathBuf {
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
        self.out_dir()
            .join(self.config.application.name.clone())
            .join(if release { "release" } else { "debug" })
            .join(platform.build_folder_name())
    }

    /// target/dx/bundle/app/
    /// target/dx/bundle/app/blah.app
    /// target/dx/bundle/app/blah.exe
    /// target/dx/bundle/app/public/
    pub(crate) fn bundle_dir(&self, platform: Platform) -> PathBuf {
        self.out_dir()
            .join(self.config.application.name.clone())
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

    /// Get the features required to build for the given platform
    pub(crate) fn feature_for_platform(&self, platform: Platform) -> Option<String> {
        let package = self.package();

        // Try to find the feature that activates the dioxus feature for the given platform
        let dioxus_feature = platform.feature_name();

        package.features.iter().find_map(|(key, features)| {
            // Find a feature that starts with dioxus/ or dioxus?/
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
        })
    }

    /// Check if assets should be pre_compressed. This will only be true in release mode if the user
    /// has enabled pre_compress in the web config.
    pub(crate) fn should_pre_compress_web_assets(&self, release: bool) -> bool {
        self.config.web.pre_compress && release
    }

    // The `opt-level=2` increases build times, but can noticeably decrease time
    // between saving changes and being able to interact with an app (for wasm/web). The "overall"
    // time difference (between having and not having the optimization) can be
    // almost imperceptible (~1 s) but also can be very noticeable (~6 s) — depends
    // on setup (hardware, OS, browser, idle load).
    //
    // Find or create the client and server profiles in the .cargo/config.toml file
    pub(crate) fn initialize_profiles(&self) -> crate::Result<()> {
        let config_path = self.workspace_dir().join(".cargo/config.toml");
        let mut config = match std::fs::read_to_string(&config_path) {
            Ok(config) => config.parse::<toml_edit::DocumentMut>().map_err(|e| {
                crate::Error::Other(anyhow::anyhow!("Failed to parse .cargo/config.toml: {}", e))
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
                client.insert("opt-level", Item::Value(2.into()));
                entry.insert(Item::Table(client));
            }

            if let toml_edit::Entry::Vacant(entry) = table.entry(PROFILE_SERVER) {
                let mut server = toml_edit::Table::new();
                server.insert("inherits", Item::Value("dev".into()));
                server.insert("opt-level", Item::Value(2.into()));
                entry.insert(Item::Table(server));
            }

            if let toml_edit::Entry::Vacant(entry) = table.entry(PROFILE_ANDROID) {
                let mut android = toml_edit::Table::new();
                android.insert("inherits", Item::Value("dev".into()));
                android.insert("opt-level", Item::Value(2.into()));
                entry.insert(Item::Table(android));
            }
        }

        // Write the config back to the file
        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let file = std::fs::File::create(config_path)?;
        let mut buf_writer = std::io::BufWriter::new(file);
        write!(buf_writer, "{}", config)?;

        Ok(())
    }

    /// Create a new gitignore map for this target crate
    pub fn gitignore(&self) -> ignore::gitignore::Gitignore {
        let crate_dir = self.crate_dir();

        let mut ignore_builder = ignore::gitignore::GitignoreBuilder::new(&crate_dir);
        ignore_builder.add(crate_dir.join(".gitignore"));

        let workspace_dir = self.workspace_dir();
        ignore_builder.add(workspace_dir.join(".gitignore"));

        let excluded_paths = vec![
            ".git",
            ".github",
            ".vscode",
            "target",
            "node_modules",
            "dist",
        ];

        for path in excluded_paths {
            ignore_builder
                .add_line(None, path)
                .expect("failed to add path to file excluder");
        }

        ignore_builder.build().unwrap()
    }

    /// Return the version of the wasm-bindgen crate if it exists
    pub fn wasm_bindgen_version(&self) -> Option<String> {
        for krate in self.krates.krates_by_name("wasm-bindgen") {
            return Some(krate.krate.version.to_string());
        }

        None
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
fn find_main_package(package: Option<String>, krates: &Krates) -> Result<NodeId> {
    let kid = match package {
        Some(package) => {
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
                eprintln!("Could not find package {package} in the workspace. Did you forget to add it to the workspace?");
                eprintln!("Packages in the workspace:");
                for package in krates.workspace_members() {
                    if let krates::Node::Krate { krate, .. } = package {
                        eprintln!("{}", krate.name());
                    }
                }
            }

            found.ok_or_else(|| anyhow::anyhow!("Failed to find package {package}"))?
        }
        None => {
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
            closest_parent
                .map(|(id, _)| id)
                .context("Failed to find current package")?
        }
    };

    let package = krates.nid_for_kid(kid).unwrap();
    Ok(package)
}
