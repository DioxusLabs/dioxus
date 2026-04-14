use super::HotpatchModuleCache;
use crate::BuildRequest;
use crate::{
    opt::{process_file_to, AppManifest},
    BuildMode,
};
use crate::{
    AndroidTools, BuildContext, BuildId, BundleFormat, DioxusConfig, LinkAction, ObjectCache,
    Platform, Renderer, Result, RustcArgs, TargetArgs, Workspace, DX_RUSTC_WRAPPER_ENV_VAR,
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
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{borrow::Cow, ffi::OsString};
use std::{
    collections::{HashMap, HashSet, VecDeque},
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

#[derive(Clone, Debug)]
struct WorkspaceRustcCaptureRequirement {
    capture_key: String,
    fingerprint_name: String,
    requires_link_args: bool,
}

impl BuildRequest {
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
    pub fn bust_fingerprint(&self, ctx: &BuildContext) -> Result<()> {
        // Only bust fingerpint when doing builds
        if !matches!(ctx.mode, BuildMode::Fat) {
            return Ok(());
        }

        // We need to find the workspace crates in our disk args index that don't have a cache entry.
        // If a crate is missing, then we need to bust its fingerprint to ensure its args get captured.
        let busted = self.missing_workspace_rustc_capture_fingerprints(&ctx.mode)?;
        if busted.is_empty() {
            tracing::debug!(
                "Rustc wrapper cache already complete for scope {}",
                self.rustc_wrapper_scope_dir_name(&ctx.mode)?
            );
            return Ok(());
        }

        tracing::debug!(
            "Busting fingerprints for crates missing rustc wrapper captures: {:?}",
            busted
        );

        // split at the last `-` used to separate the hash from the name
        // This causes to more aggressively bust hashes for all combinations of features
        // and fingerprints for this package since we're just ignoring the hash
        if let Ok(entries) = std::fs::read_dir(self.cargo_fingeprint_dir()) {
            let mut removed = Vec::new();
            for entry in entries.flatten() {
                if let Some(fname) = entry.file_name().to_str() {
                    if let Some((name, _)) = fname.rsplit_once('-') {
                        if busted.contains(name) {
                            removed.push(fname.to_string());
                            _ = std::fs::remove_dir_all(entry.path());
                        }
                    }
                }
            }
            tracing::debug!("Removed fingerprint directories: {:?}", removed);
        }

        Ok(())
    }

    fn persist_rustc_args(&self, args_dir: &Path, key: &str, args: &RustcArgs) -> Result<()> {
        std::fs::create_dir_all(args_dir).context("Failed to create rustc wrapper cache dir")?;
        let path = args_dir.join(format!("{key}.json"));
        let contents =
            serde_json::to_string(args).context("Failed to serialize rustc wrapper args")?;
        std::fs::write(path, contents).context("Failed to write rustc wrapper args")?;
        Ok(())
    }

    fn required_workspace_rustc_captures(&self) -> Vec<WorkspaceRustcCaptureRequirement> {
        let mut requirements = vec![WorkspaceRustcCaptureRequirement {
            capture_key: format!("{}.bin", self.tip_crate_name()),
            fingerprint_name: self.package().name.clone(),
            requires_link_args: true,
        }];

        if Self::package_has_lib_target(self.package()) {
            requirements.push(WorkspaceRustcCaptureRequirement {
                capture_key: format!("{}.lib", self.tip_crate_name()),
                fingerprint_name: self.package().name.clone(),
                requires_link_args: false,
            });
        }

        let workspace_members: HashSet<NodeId> = self
            .workspace
            .krates
            .workspace_members()
            .filter_map(|member| match member {
                krates::Node::Krate { id, .. } => self.workspace.krates.nid_for_kid(id),
                _ => None,
            })
            .collect();

        let mut visited = HashSet::new();
        let mut queue = VecDeque::from([self.crate_package]);
        visited.insert(self.crate_package);

        while let Some(current) = queue.pop_front() {
            for (dep, _) in self.workspace.krates.get_deps(current) {
                let (dep_nid, krate) = match dep {
                    krates::Node::Krate { id, krate, .. } => {
                        let Some(dep_nid) = self.workspace.krates.nid_for_kid(id) else {
                            continue;
                        };
                        (dep_nid, krate)
                    }
                    _ => continue,
                };

                if !workspace_members.contains(&dep_nid) || !visited.insert(dep_nid) {
                    continue;
                }

                queue.push_back(dep_nid);

                let normalized_name = krate.name.replace('-', "_");
                if krate
                    .targets
                    .iter()
                    .any(|target| target.kind.contains(&TargetKind::Lib))
                {
                    requirements.push(WorkspaceRustcCaptureRequirement {
                        capture_key: format!("{normalized_name}.lib"),
                        fingerprint_name: krate.name.clone(),
                        requires_link_args: false,
                    });
                }
            }
        }

        requirements
    }

    pub fn missing_workspace_rustc_capture_fingerprints(
        &self,
        build_mode: &BuildMode,
    ) -> Result<HashSet<String>> {
        let args_dir = self.rustc_wrapper_args_scope_dir(build_mode)?;
        let captured = self.load_workspace_rustc_args_from_dir(&args_dir);
        let requirements = self.required_workspace_rustc_captures();

        let missing = requirements
            .into_iter()
            .filter(|requirement| {
                let Some(args) = captured.get(&requirement.capture_key) else {
                    return true;
                };

                requirement.requires_link_args && args.link_args.is_empty()
            })
            .map(|requirement| requirement.fingerprint_name)
            .collect::<HashSet<_>>();

        if matches!(build_mode, BuildMode::Fat) {
            // The tip crate's linker response file references object files from the current build,
            // so we still need Cargo to rerun the tip package on every fat build even when the
            // cached rustc wrapper metadata for the package is otherwise valid.
            let mut missing = missing;
            missing.insert(self.package().name.clone());
            return Ok(missing);
        }

        Ok(missing)
    }

    fn rustc_wrapper_capture_mode(&self, build_mode: &BuildMode) -> &'static str {
        match build_mode {
            BuildMode::Fat => "fat",
            BuildMode::Base { run: true } => "base-run",
            BuildMode::Base { run: false } => "base",
            BuildMode::Thin { .. } => "thin",
        }
    }

    pub fn rustc_wrapper_scope_dir_name(&self, build_mode: &BuildMode) -> Result<String> {
        #[derive(Debug, Serialize)]
        struct RustcWrapperScope {
            version: u8,
            capture_mode: &'static str,
            bundle: String,
            triple: String,
            profile: String,
            package: String,
            main_target: String,
            executable_type: String,
            rustc_version: String,
            features: Vec<String>,
            all_features: bool,
            rustflags: Vec<String>,
            extra_cargo_args: Vec<String>,
            extra_rustc_args: Vec<String>,
        }

        let scope = RustcWrapperScope {
            version: 1,
            capture_mode: self.rustc_wrapper_capture_mode(build_mode),
            bundle: self.bundle.to_string(),
            triple: self.triple.to_string(),
            profile: self.profile.clone(),
            package: self.package.clone(),
            main_target: self.main_target.clone(),
            executable_type: format!("{:?}", self.executable_type()),
            rustc_version: self.workspace.rustc_version.clone(),
            features: self.features.clone(),
            all_features: self.all_features,
            rustflags: self.rustflags.flags.clone(),
            extra_cargo_args: self.extra_cargo_args.clone(),
            extra_rustc_args: self.extra_rustc_args.clone(),
        };

        let encoded =
            serde_json::to_vec(&scope).context("Failed to serialize rustc wrapper scope")?;
        let mut hasher = Sha256::new();
        hasher.update(encoded);
        let scope_hash = format!("{:x}", hasher.finalize());
        Ok(format!(
            "scope-v{}-{}-{}-{}",
            scope.version,
            scope.capture_mode,
            self.tip_crate_name(),
            &scope_hash[..16]
        ))
    }

    pub fn load_workspace_rustc_args_from_dir(
        &self,
        args_dir: &Path,
    ) -> HashMap<String, RustcArgs> {
        let mut workspace_rustc_args = HashMap::new();

        if let Ok(entries) = std::fs::read_dir(args_dir) {
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

        workspace_rustc_args
    }

    fn compiler_artifact_key(&self, artifact: &cargo_metadata::Artifact) -> Option<String> {
        let normalized_name = artifact.target.name.replace('-', "_");
        let kinds = &artifact.target.kind;

        if kinds.iter().any(|kind| {
            matches!(
                kind,
                cargo_metadata::TargetKind::Lib | cargo_metadata::TargetKind::RLib
            )
        }) {
            Some(format!("{normalized_name}.lib"))
        } else if kinds
            .iter()
            .any(|kind| matches!(kind, cargo_metadata::TargetKind::Bin))
        {
            Some(format!("{normalized_name}.bin"))
        } else {
            None
        }
    }

    fn record_workspace_artifact_path(
        &self,
        artifact_paths: &mut HashMap<String, PathBuf>,
        artifact: &cargo_metadata::Artifact,
    ) {
        let Some(key) = self.compiler_artifact_key(artifact) else {
            return;
        };

        let path = if key.ends_with(".lib") {
            artifact
                .filenames
                .iter()
                .find(|path| path.extension().is_some_and(|ext| ext == "rlib"))
                .map(|path| PathBuf::from(path.as_std_path()))
        } else {
            artifact
                .executable
                .as_ref()
                .map(|path| PathBuf::from(path.as_std_path()))
        };

        if let Some(path) = path {
            artifact_paths.insert(key, path);
        }
    }

    fn package_has_lib_target(package: &krates::cm::Package) -> bool {
        package
            .targets
            .iter()
            .any(|target| target.kind.contains(&TargetKind::Lib))
    }
}
