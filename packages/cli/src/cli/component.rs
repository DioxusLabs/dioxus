use std::{
    collections::{HashMap, HashSet},
    ops::Deref,
    path::{Path, PathBuf},
};

use crate::{verbosity_or_default, DioxusConfig, Result, StructuredOutput, Workspace};
use anyhow::{bail, Context};
use clap::Parser;
use dioxus_component_manifest::{
    component_manifest_schema, CargoDependency, Component, ComponentDependency,
};
use git2::Repository;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tokio::{process::Command, task::JoinSet};
use tracing::debug;

#[derive(Clone, Debug, Parser)]
pub enum ComponentCommand {
    /// Add a component from a registry
    Add {
        #[clap(flatten)]
        component: ComponentArgs,

        /// The registry to use
        #[clap(flatten)]
        registry: ComponentRegistry,

        /// Overwrite the component if it already exists
        #[clap(long)]
        force: bool,
    },

    /// Remove a component
    Remove {
        #[clap(flatten)]
        component: ComponentArgs,

        /// The registry to use
        #[clap(flatten)]
        registry: ComponentRegistry,
    },

    /// Update a component registry
    Update {
        /// The registry to update
        #[clap(flatten)]
        registry: Option<RemoteComponentRegistry>,
    },

    /// List available components in a registry
    List {
        /// The registry to list components in
        #[clap(flatten)]
        registry: ComponentRegistry,
    },

    /// Clear the component registry cache
    Clean,

    /// Print the schema for component manifests
    Schema,
}

/// Arguments for a component and component module location
#[derive(Clone, Debug, Parser, Serialize)]
pub struct ComponentArgs {
    /// The components to add or remove
    #[clap(required_unless_present = "all", value_delimiter = ',')]
    components: Vec<String>,

    /// The location of the component module in your project (default: src/components)
    #[clap(long)]
    module_path: Option<PathBuf>,

    /// The location of the global assets in your project (default: assets)
    #[clap(long)]
    global_assets_path: Option<PathBuf>,

    /// Include all components in the registry
    #[clap(long)]
    all: bool,
}

impl ComponentCommand {
    /// Run the component command
    pub async fn run(self) -> Result<StructuredOutput> {
        match self {
            // List all components in the registry
            Self::List { registry } => {
                let config = Self::resolve_config().await?;
                let registry = Self::resolve_registry(registry, &config)?;
                let mut components = registry.read_components().await?;
                components.sort_by_key(|c| c.name.clone());
                for component in components {
                    println!("- {}: {}", component.name, component.description);
                }
            }

            // Add a component to the managed component module
            Self::Add {
                component: component_args,
                registry,
                force,
            } => {
                // Resolve the config
                let config = Self::resolve_config().await?;

                // Resolve the registry
                let registry = Self::resolve_registry(registry, &config)?;

                // Get the registry root. Components can't copy files outside of this path
                let registry_root = registry.resolve().await?;

                // Read all components from the registry
                let components = registry.read_components().await?;
                let mode = if force {
                    ComponentExistsBehavior::Overwrite
                } else {
                    ComponentExistsBehavior::Error
                };

                // Find the requested components
                let components = if component_args.all {
                    components
                } else {
                    component_args
                        .components
                        .iter()
                        .map(|component| find_component(&components, component))
                        .collect::<Result<Vec<_>>>()?
                };

                // Find and initialize the components module if it doesn't exist
                let components_root =
                    components_root(component_args.module_path.as_deref(), &config)?;
                let new_components_module =
                    ensure_components_module_exists(&components_root).await?;

                // Recursively add dependencies
                // A map of the components that have been added or are queued to be added
                let mut required_components = HashMap::new();
                required_components.extend(components.iter().cloned().map(|c| (c, mode)));
                // A stack of components to process
                let mut queued_components = components;
                while let Some(queued_component) = queued_components.pop() {
                    for dependency in &queued_component.component_dependencies {
                        let (registry, name) = match dependency {
                            ComponentDependency::Builtin(name) => {
                                (ComponentRegistry::default(), name)
                            }
                            ComponentDependency::ThirdParty { name, git, rev } => (
                                ComponentRegistry {
                                    remote: RemoteComponentRegistry {
                                        git: Some(git.clone()),
                                        rev: rev.clone(),
                                    },
                                    path: None,
                                },
                                name,
                            ),
                        };
                        let registry_components = registry.read_components().await?;
                        let dependency_component = find_component(&registry_components, name)?;
                        if required_components
                            .insert(
                                dependency_component.clone(),
                                ComponentExistsBehavior::Return,
                            )
                            .is_none()
                        {
                            queued_components.push(dependency_component);
                        }
                    }
                }

                // Then collect all required rust dependencies
                let mut rust_dependencies = HashSet::new();
                for component in required_components.keys() {
                    rust_dependencies.extend(component.cargo_dependencies.iter().cloned());
                }

                // And add them to Cargo.toml
                Self::add_rust_dependencies(&rust_dependencies).await?;

                // Once we have all required components, add them
                for (component, mode) in required_components {
                    add_component(
                        &registry_root,
                        component_args.global_assets_path.as_deref(),
                        component_args.module_path.as_deref(),
                        &component,
                        mode,
                        &config,
                    )
                    .await?;
                }

                // If we created a new components module, print instructions about the final setup steps required
                if new_components_module {
                    println!(
                        "Created new components module at {}.",
                        components_root.display()
                    );
                    println!("To finish setting up components, you will need to:");
                    println!("- manually reference the module by adding `mod components;` to your `main.rs` file");
                    if registry.is_default() {
                        println!("- add a reference to `asset!(\"/assets/dx-components-theme.css\")` as a stylesheet in your app");
                    }
                }
            }

            // Update the remote component registry
            Self::Update { registry } => {
                let config = Self::resolve_config().await?;
                registry
                    .unwrap_or(config.components.registry.remote)
                    .update()
                    .await?;
            }

            // Remove a component from the managed component module
            Self::Remove {
                component,
                registry,
            } => {
                Self::remove_component(&component, registry).await?;
            }

            // Clear the component registry cache
            Self::Clean => {
                _ = tokio::fs::remove_dir_all(&Workspace::component_cache_dir()).await;
            }

            // Print the schema for component manifests
            Self::Schema => {
                let schema = component_manifest_schema();
                println!(
                    "{}",
                    serde_json::to_string_pretty(&schema).unwrap_or_default()
                );
            }
        }

        Ok(StructuredOutput::Success)
    }

    /// Remove a component from the managed component module
    async fn remove_component(
        component_args: &ComponentArgs,
        registry: ComponentRegistry,
    ) -> Result<()> {
        let config = Self::resolve_config().await?;
        let registry = Self::resolve_registry(registry, &config)?;

        let components_root = components_root(component_args.module_path.as_deref(), &config)?;

        // Find the requested components
        let components = if component_args.all {
            registry
                .read_components()
                .await?
                .into_iter()
                .map(|c| c.component.name)
                .collect()
        } else {
            component_args.components.clone()
        };

        for component_name in components {
            // Remove the component module
            _ = tokio::fs::remove_dir_all(&components_root.join(&component_name)).await;

            // Remove the module from the components mod.rs
            let mod_rs_path = components_root.join("mod.rs");
            let mod_rs_content = tokio::fs::read_to_string(&mod_rs_path)
                .await
                .with_context(|| format!("Failed to read {}", mod_rs_path.display()))?;
            let mod_line = format!("pub mod {};\n", component_name);
            let new_mod_rs_content = mod_rs_content.replace(&mod_line, "");
            tokio::fs::write(&mod_rs_path, new_mod_rs_content)
                .await
                .with_context(|| format!("Failed to write to {}", mod_rs_path.display()))?;
        }
        Ok(())
    }

    /// Load the config
    async fn resolve_config() -> Result<DioxusConfig> {
        let workspace = Workspace::current().await?;

        let crate_package = workspace.find_main_package(None)?;

        Ok(workspace
            .load_dioxus_config(crate_package, None)?
            .unwrap_or_default())
    }

    /// Resolve a registry from the config if none is provided
    fn resolve_registry(
        registry: ComponentRegistry,
        config: &DioxusConfig,
    ) -> Result<ComponentRegistry> {
        if !registry.is_default() {
            return Ok(registry);
        }

        Ok(config.components.registry.clone())
    }

    /// Add any rust dependencies required for a component
    async fn add_rust_dependencies(dependencies: &HashSet<CargoDependency>) -> Result<()> {
        for dep in dependencies {
            let status = Command::from(dep.add_command())
                .status()
                .await
                .with_context(|| {
                    format!(
                        "Failed to run command to add dependency {} to Cargo.toml",
                        dep.name()
                    )
                })?;
            if !status.success() {
                bail!("Failed to add dependency {} to Cargo.toml", dep.name());
            }
        }

        Ok(())
    }
}

/// Arguments for the default or custom remote registry
/// If both values are None, the default registry will be used
#[derive(Clone, Debug, Parser, Default, Serialize, Deserialize, JsonSchema)]
pub struct RemoteComponentRegistry {
    /// The url of the the component registry
    #[arg(long)]
    git: Option<String>,

    /// The revision of the the component registry
    #[arg(long)]
    rev: Option<String>,
}

impl RemoteComponentRegistry {
    /// Resolve the path to the component registry, downloading the remote registry if needed
    async fn resolve(&self) -> Result<PathBuf> {
        // If a git url is provided use that (plus optional rev)
        // Otherwise use the built-in registry
        let (git, rev) = self.resolve_or_default();

        let repo_dir = Workspace::component_cache_path(&git, rev.as_deref());

        // If the repo already exists, use it otherwise clone it
        if !repo_dir.exists() {
            // If offline, we cannot download the registry
            if verbosity_or_default().offline {
                bail!("Cannot download component registry '{}' while offline", git);
            }

            // Make sure the parent directory exists
            tokio::fs::create_dir_all(&repo_dir).await?;
            tokio::task::spawn_blocking({
                let git = git.clone();
                let repo_dir = repo_dir.clone();
                move || {
                    println!("Downloading {git}...");

                    // Clone the repo
                    let repo = Repository::clone(&git, repo_dir)?;

                    // If a rev is provided, checkout that rev
                    if let Some(rev) = &rev {
                        Self::checkout_rev(&repo, &git, rev)?;
                    }

                    anyhow::Ok(())
                }
            })
            .await??;
        }

        Ok(repo_dir)
    }

    /// Update the component registry by fetching the latest changes from the remote
    async fn update(&self) -> Result<()> {
        let (git, rev) = self.resolve_or_default();

        // Make sure the repo is cloned
        let path = self.resolve().await?;

        // Open the repo and update it
        tokio::task::spawn_blocking({
            let path = path.clone();
            move || {
                let repo = Repository::open(path)?;
                let mut remote = repo.find_remote("origin")?;
                // Fetch all remote branches with the same name as local branches
                remote.fetch(&["refs/heads/*:refs/heads/*"], None, None)?;
                // If a rev is provided, checkout that rev
                if let Some(rev) = &rev {
                    Self::checkout_rev(&repo, &git, rev)?;
                }
                // Otherwise, just checkout the latest commit on the default branch
                else {
                    let head = repo.head()?;
                    let branch = head.shorthand().unwrap_or("main");
                    let oid = repo.refname_to_id(&format!("refs/remotes/origin/{branch}"))?;
                    let object = repo.find_object(oid, None).unwrap();
                    repo.reset(&object, git2::ResetType::Hard, None)?;
                }
                anyhow::Ok(())
            }
        })
        .await??;

        Ok(())
    }

    /// If a git url is provided use that (plus optional rev)
    /// Otherwise use the built-in registry
    fn resolve_or_default(&self) -> (String, Option<String>) {
        if let Some(git) = &self.git {
            (git.clone(), self.rev.clone())
        } else {
            ("https://github.com/dioxuslabs/components".into(), None)
        }
    }

    /// Checkout the given rev in the given repo
    fn checkout_rev(repo: &Repository, git: &str, rev: &str) -> Result<()> {
        let (object, reference) = repo
            .revparse_ext(rev)
            .with_context(|| format!("Failed to find revision '{}' in '{}'", rev, git))?;
        repo.checkout_tree(&object, None)?;

        if let Some(gref) = reference {
            if let Some(name) = gref.name() {
                repo.set_head(name)?;
            }
        } else {
            repo.set_head_detached(object.id())?;
        }

        Ok(())
    }
}

/// Arguments for a component registry
/// Either a path to a local directory or a remote git repo (with optional rev)
#[derive(Clone, Debug, Parser, Default, Serialize, Deserialize, JsonSchema)]
pub struct ComponentRegistry {
    /// The remote repo args
    #[clap(flatten)]
    #[serde(flatten)]
    remote: RemoteComponentRegistry,

    /// The path to the components directory
    #[arg(long)]
    path: Option<String>,
}

impl ComponentRegistry {
    /// Resolve the path to the component registry, downloading the remote registry if needed
    async fn resolve(&self) -> Result<PathBuf> {
        // If a path is provided, use that
        if let Some(path) = &self.path {
            return Ok(PathBuf::from(path));
        }

        // Otherwise use the remote/default registry
        self.remote.resolve().await
    }

    /// Read all components that are part of this registry
    async fn read_components(&self) -> Result<Vec<ResolvedComponent>> {
        let path = self.resolve().await?;

        let root = read_component(&path).await?;
        let mut components = discover_components(root).await?;

        // Filter out any virtual components with members
        components.retain(|c| c.members.is_empty());

        Ok(components)
    }

    /// Check if this is the default registry
    fn is_default(&self) -> bool {
        self.path.is_none() && self.remote.git.is_none() && self.remote.rev.is_none()
    }
}

/// A component that has been downloaded and resolved at a specific path
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct ResolvedComponent {
    path: PathBuf,
    component: Component,
}

impl ResolvedComponent {
    /// Get the absolute paths to members of this component
    fn member_paths(&self) -> Vec<PathBuf> {
        self.component
            .members
            .iter()
            .map(|m| self.path.join(m))
            .collect()
    }
}

impl Deref for ResolvedComponent {
    type Target = Component;

    fn deref(&self) -> &Self::Target {
        &self.component
    }
}

// Find a component by name in a list of components
fn find_component(components: &[ResolvedComponent], component: &str) -> Result<ResolvedComponent> {
    components
        .iter()
        .find(|c| c.name == component)
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("Component '{}' not found in registry", component))
}

/// Get the path to the components module, defaulting to src/components
fn components_root(module_path: Option<&Path>, config: &DioxusConfig) -> Result<PathBuf> {
    if let Some(module_path) = module_path {
        return Ok(PathBuf::from(module_path));
    }

    let root = Workspace::crate_root_from_path()?;

    if let Some(component_path) = &config.components.components_dir {
        return Ok(root.join(component_path));
    }

    Ok(root.join("src").join("components"))
}

/// Get the path to the global assets directory, defaulting to assets
async fn global_assets_root(assets_path: Option<&Path>, config: &DioxusConfig) -> Result<PathBuf> {
    if let Some(assets_path) = assets_path {
        return Ok(PathBuf::from(assets_path));
    }

    if let Some(asset_dir) = &config.application.asset_dir {
        return Ok(asset_dir.clone());
    }

    let root = Workspace::crate_root_from_path()?;

    Ok(root.join("assets"))
}

/// How should we handle the component if it already exists
#[derive(Clone, Copy, Debug)]
enum ComponentExistsBehavior {
    /// Return an error (default)
    Error,

    /// Return early for component dependencies
    Return,

    /// Overwrite the existing component
    Overwrite,
}

/// Add a component to the managed component module
async fn add_component(
    registry_root: &Path,
    assets_path: Option<&Path>,
    component_path: Option<&Path>,
    component: &ResolvedComponent,
    behavior: ComponentExistsBehavior,
    config: &DioxusConfig,
) -> Result<()> {
    // Copy the folder content to the components directory
    let components_root = components_root(component_path, config)?;
    let copied = copy_component_files(
        &component.path,
        &components_root.join(&component.name),
        &component.exclude,
        behavior,
    )
    .await?;
    if !copied {
        debug!(
            "Component '{}' already exists, skipping copy",
            component.name
        );
        return Ok(());
    }

    // Copy any global assets
    let assets_root = global_assets_root(assets_path, config).await?;
    copy_global_assets(registry_root, &assets_root, component).await?;

    // Add the module to the components mod.rs
    let mod_rs_path = components_root.join("mod.rs");
    let mut mod_rs = tokio::fs::OpenOptions::new()
        .append(true)
        .read(true)
        .open(&mod_rs_path)
        .await
        .with_context(|| format!("Failed to open {}", mod_rs_path.display()))?;

    // Check if the module already exists
    let mod_rs_content = tokio::fs::read_to_string(&mod_rs_path)
        .await
        .with_context(|| format!("Failed to read {}", mod_rs_path.display()))?;
    if !mod_rs_content.contains(&format!("mod {};", component.name)) {
        let mod_line = format!("pub mod {};\n", component.name);
        tokio::io::AsyncWriteExt::write_all(&mut mod_rs, mod_line.as_bytes())
            .await
            .with_context(|| format!("Failed to write to {}", mod_rs_path.display()))?;
    }

    Ok(())
}

/// Copy the component files. Returns true if the component was copied, false if it was skipped.
async fn copy_component_files(
    src: &Path,
    dest: &Path,
    exclude: &[String],
    behavior: ComponentExistsBehavior,
) -> Result<bool> {
    async fn read_dir_paths(src: &Path) -> Result<Vec<PathBuf>> {
        let mut entries = tokio::fs::read_dir(src).await?;
        let mut paths = vec![];
        while let Some(entry) = entries.next_entry().await? {
            paths.push(entry.path());
        }
        Ok(paths)
    }

    // If the directory already exists, return an error, return silently or overwrite it depending on the behavior
    if dest.exists() {
        match behavior {
            // The default behavior is to return an error
            ComponentExistsBehavior::Error => {
                bail!("Destination directory '{}' already exists", dest.display());
            }
            // For dependencies, we return early
            ComponentExistsBehavior::Return => {
                debug!(
                    "Destination directory '{}' already exists, returning early",
                    dest.display()
                );
                return Ok(false);
            }
            // If the force flag is set, we overwrite the existing component
            ComponentExistsBehavior::Overwrite => {
                debug!(
                    "Destination directory '{}' already exists, overwriting",
                    dest.display()
                );
                tokio::fs::remove_dir_all(dest).await?;
            }
        }
    }

    tokio::fs::create_dir_all(dest).await?;

    let exclude = exclude
        .iter()
        .map(|exclude| dunce::canonicalize(src.join(exclude)))
        .collect::<Result<Vec<_>, _>>()?;

    // Set set of tasks to read directories
    let mut read_folder_tasks = JoinSet::new();
    // Set set of tasks to copy files
    let mut copy_tasks = JoinSet::new();

    // Start by reading the source directory
    let src = src.to_path_buf();
    read_folder_tasks.spawn({
        let src = src.clone();
        async move { read_dir_paths(&src).await }
    });

    // Continue while there are read tasks
    while let Some(res) = read_folder_tasks.join_next().await {
        let paths = res??;
        for path in paths {
            let path = dunce::canonicalize(path)?;

            // Skip excluded paths
            if exclude.iter().any(|e| *e == path || path.starts_with(e)) {
                debug!("Excluding path {}", path.display());
                continue;
            }

            // Find the path in the destination directory
            let Ok(path_relative_to_src) = path.strip_prefix(&src) else {
                continue;
            };
            let dest = dest.join(path_relative_to_src);

            // If it's a directory, read it, otherwise copy the file
            if path.is_dir() {
                read_folder_tasks.spawn(async move { read_dir_paths(&path).await });
            } else {
                copy_tasks.spawn(async move {
                    if let Some(parent) = dest.parent() {
                        if !parent.exists() {
                            tokio::fs::create_dir_all(parent).await?;
                        }
                    }
                    tokio::fs::copy(&path, &dest).await
                });
            }
        }
    }

    // Wait for all copy tasks to finish
    while let Some(res) = copy_tasks.join_next().await {
        res??;
    }

    Ok(true)
}

/// Make sure the components directory and a mod.rs file exists. Returns true if the directory was created, false if it already existed.
async fn ensure_components_module_exists(components_dir: &Path) -> Result<bool> {
    if components_dir.exists() {
        return Ok(false);
    }
    tokio::fs::create_dir_all(&components_dir).await?;
    let mod_rs_path = components_dir.join("mod.rs");
    if mod_rs_path.exists() {
        return Ok(false);
    }
    tokio::fs::write(&mod_rs_path, "// AUTOGENERATED Components module\n").await?;

    Ok(true)
}

/// Read a component from the given path
async fn read_component(path: &Path) -> Result<ResolvedComponent> {
    let json_path = path.join("component.json");
    let bytes = tokio::fs::read(&json_path).await.with_context(|| {
        format!(
            "Failed to open component manifest at {}",
            json_path.display()
        )
    })?;

    let component = serde_json::from_slice(&bytes)?;
    let absolute_path = dunce::canonicalize(path)?;
    Ok(ResolvedComponent {
        path: absolute_path,
        component,
    })
}

/// Recursively discover all components starting from the root component
async fn discover_components(root: ResolvedComponent) -> Result<Vec<ResolvedComponent>> {
    // Create a queue of members to read
    let mut queue = root.member_paths();
    // The list of discovered components
    let mut components = vec![root];
    // The set of pending read tasks
    let mut pending = JoinSet::new();
    loop {
        // First, spawn tasks for all queued paths
        while let Some(root_path) = queue.pop() {
            pending.spawn(async move { read_component(&root_path).await });
        }
        // Then try to join the next task
        let Some(component) = pending.join_next().await else {
            break;
        };
        let component = component??;
        // And add the result to the queue and list
        queue.extend(component.member_paths());
        components.push(component);
    }
    Ok(components)
}

/// Copy any global assets for the component
async fn copy_global_assets(
    registry_root: &Path,
    assets_root: &Path,
    component: &ResolvedComponent,
) -> Result<()> {
    let canonical_registry_root = dunce::canonicalize(registry_root)?;
    for path in &component.global_assets {
        let src = component.path.join(path);
        let absolute_source = dunce::canonicalize(&src).with_context(|| {
            format!(
                "Failed to find global asset '{}' for component '{}'",
                src.display(),
                component.name
            )
        })?;

        // Make sure the source is inside the component registry somewhere
        if !absolute_source.starts_with(&canonical_registry_root) {
            bail!(
                "Cannot copy global asset '{}' for component '{}' because it is outside of the component registry '{}'",
                absolute_source.display(),
                component.name,
                canonical_registry_root.display()
            );
        }

        // Copy the file into the assets directory, preserving the file name and extension
        let dest = assets_root.join(
            absolute_source
                .components()
                .next_back()
                .context("Global assets must have at least one file component")?,
        );

        // Make sure the asset dir exists
        if let Some(parent) = dest.parent() {
            if !parent.exists() {
                tokio::fs::create_dir_all(parent).await?;
            }
        }

        tokio::fs::copy(&src, &dest).await.with_context(|| {
            format!(
                "Failed to copy global asset from {} to {}",
                src.display(),
                dest.display()
            )
        })?;
    }

    Ok(())
}
