use std::{
    collections::HashMap,
    ops::Deref,
    path::{Path, PathBuf},
};

use crate::{verbosity_or_default, Result, StructuredOutput, Workspace};
use anyhow::Context;
use clap::Parser;
use dioxus_component_manifest::{
    component_manifest_schema, CargoDependency, Component, ComponentDependency,
};
use git2::Repository;
use tokio::{process::Command, task::JoinSet};
use tracing::debug;

/// Arguments for the default or custom remote registry
/// If both values are None, the default registry will be used
#[derive(Clone, Debug, Parser, Default)]
pub struct RemoteComponentRegisteryArgs {
    /// The url of the the component registry
    #[arg(long)]
    git: Option<String>,
    /// The revision of the the component registry
    #[arg(long)]
    rev: Option<String>,
}

impl RemoteComponentRegisteryArgs {
    /// If a git url is provided use that (plus optional rev)
    /// Otherwise use the built-in registry
    fn resolve_or_default(&self) -> (String, Option<String>) {
        if let Some(git) = &self.git {
            (git.clone(), self.rev.clone())
        } else {
            (
                "https://github.com/ealmloff/components".into(),
                Some("origin/components-cli".into()),
            )
        }
    }

    async fn resolve(&self) -> Result<PathBuf> {
        // If a git url is provided use that (plus optional rev)
        // Otherwise use the built-in registry
        let (git, rev) = self.resolve_or_default();

        let repo_dir = Workspace::component_cache_path(&git, rev.as_deref());
        // If the repo already exists, use it otherwise clone it
        if !repo_dir.exists() {
            // If offline, we cannot download the registry
            if verbosity_or_default().offline {
                return Err(anyhow::anyhow!(
                    "Cannot download component registry '{}' while offline",
                    git
                ));
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
                        checkout_rev(&repo, &git, rev)?;
                    }
                    anyhow::Ok(())
                }
            })
            .await??;
        }

        Ok(repo_dir)
    }

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
                    checkout_rev(&repo, &git, rev)?;
                }
                anyhow::Ok(())
            }
        })
        .await??;

        Ok(())
    }
}

/// Checkout the given rev in the given repo
fn checkout_rev(repo: &Repository, git: &str, rev: &str) -> Result<()> {
    let (object, reference) = repo
        .revparse_ext(rev)
        .with_context(|| format!("Failed to find revision '{}' in '{}'", rev, git))?;
    repo.checkout_tree(&object, None)?;
    if let Some(gref) = reference {
        repo.set_head(gref.name().unwrap())?;
    } else {
        repo.set_head_detached(object.id())?;
    }
    Ok(())
}

/// Arguments for a component registry
/// Either a path to a local directory or a remote git repo (with optional rev)
#[derive(Clone, Debug, Parser, Default)]
pub struct ComponentRegisteryArgs {
    /// The remote repo args
    #[clap(flatten)]
    remote: RemoteComponentRegisteryArgs,
    /// The path to the components directory
    #[arg(long)]
    path: Option<String>,
}

impl ComponentRegisteryArgs {
    /// Resolve the path to the component registry, downloading the remote registry if needed
    async fn resolve(&self) -> Result<PathBuf> {
        // If a path is provided, use that
        if let Some(path) = &self.path {
            return Ok(PathBuf::from(path));
        }

        // Otherwise use the remote/default registry
        self.remote.resolve().await
    }

    async fn read_components(&self) -> Result<Vec<ResolvedComponent>> {
        let path = self.resolve().await?;

        let root = read_component(&path).await?;
        discover_components(root).await
    }

    fn is_default(&self) -> bool {
        self.path.is_none() && self.remote.git.is_none() && self.remote.rev.is_none()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct ResolvedComponent {
    path: PathBuf,
    component: Component,
}

impl Deref for ResolvedComponent {
    type Target = Component;

    fn deref(&self) -> &Self::Target {
        &self.component
    }
}

impl ResolvedComponent {
    fn member_paths(&self) -> Vec<PathBuf> {
        self.component
            .members
            .iter()
            .map(|m| self.path.join(m))
            .collect()
    }
}

/// Arguments for a component and component module location
#[derive(Clone, Debug, Parser)]
pub struct ComponentArgs {
    /// The component to add
    component: String,
    /// The location of the component module in your project (default: src/components)
    #[clap(long)]
    module_path: Option<PathBuf>,
    /// The location of the global assets in your project (default: assets)
    #[clap(long)]
    assets_path: Option<PathBuf>,
}

#[derive(Clone, Debug, Parser)]
pub enum ComponentCommand {
    /// Add a component from a registry
    #[clap(name = "add")]
    Add {
        #[clap(flatten)]
        component: ComponentArgs,
        /// The registry to use
        #[clap(flatten)]
        registry: ComponentRegisteryArgs,
        /// Overwrite the component if it already exists
        #[clap(long)]
        force: bool,
    },
    /// Remove a component
    #[clap(name = "remove")]
    Remove {
        #[clap(flatten)]
        component: ComponentArgs,
    },
    /// Update a component registry
    #[clap(name = "update")]
    Update {
        /// The registry to update
        #[clap(flatten)]
        registry: RemoteComponentRegisteryArgs,
    },
    /// List available components in a registry
    #[clap(name = "list")]
    List {
        /// The registry to list components in
        #[clap(flatten)]
        registry: ComponentRegisteryArgs,
    },
    /// Clear the component registry cache
    #[clap(name = "clean")]
    Clean,
    /// Print the schema for component manifests
    #[clap(name = "schema")]
    Schema,
}

impl ComponentCommand {
    pub async fn run(self) -> Result<StructuredOutput> {
        match self {
            Self::List { registry } => {
                let mut components = registry.read_components().await?;
                components.sort_by_key(|c| c.name.clone());
                for component in components {
                    println!("- {}: {}", component.name, component.description);
                }
            }
            Self::Add {
                component: component_args,
                registry,
                force,
            } => {
                let components = registry.read_components().await?;
                let mode = if force {
                    ComponentExistsBehavior::Overwrite
                } else {
                    ComponentExistsBehavior::Error
                };
                let component = find_component(components, &component_args.component).await?;

                let components_root = components_root(component_args.module_path.as_deref())?;
                let new_components_module =
                    ensure_components_module_exists(&components_root).await?;

                // Recursively add dependencies
                // A map of the components that have been added or are queued to be added
                let mut required_components = HashMap::new();
                required_components.insert(component.clone(), mode);
                // A stack of components to process
                let mut queued_components = vec![component];
                while let Some(queued_component) = queued_components.pop() {
                    for dependency in &queued_component.component_dependencies {
                        let (registry, name) = match dependency {
                            ComponentDependency::Builtin(name) => {
                                (ComponentRegisteryArgs::default(), name)
                            }
                            ComponentDependency::ThirdParty { name, git, rev } => (
                                ComponentRegisteryArgs {
                                    remote: RemoteComponentRegisteryArgs {
                                        git: Some(git.clone()),
                                        rev: rev.clone(),
                                    },
                                    path: None,
                                },
                                name,
                            ),
                        };
                        let registry_components = registry.read_components().await?;
                        let dependency_component =
                            find_component(registry_components, name).await?;
                        if !required_components
                            .insert(
                                dependency_component.clone(),
                                ComponentExistsBehavior::Return,
                            )
                            .is_some()
                        {
                            queued_components.push(dependency_component);
                        }
                    }
                }

                // Once we have all required components, add them
                for (component, mode) in required_components {
                    add_component(
                        component_args.assets_path.as_deref(),
                        component_args.module_path.as_deref(),
                        &component,
                        mode,
                    )
                    .await?;
                }

                if new_components_module {
                    println!(
                        "Created new components module at {}.",
                        components_root.display()
                    );
                    println!("To finish setting up components, you will need to:");
                    println!("- manually reference the module by adding `mod component;` to your `main.rs` file");
                    if registry.is_default() {
                        println!("- add a reference to `asset!(\"/assets/dx-components-theme.css\")` as a stylesheet in your app");
                    }
                }
            }
            Self::Update { registry } => {
                registry.update().await?;
            }
            Self::Remove { component } => {
                remove_component(&component).await?;
            }
            Self::Clean => {
                let cache_dir = Workspace::component_cache_dir();
                if cache_dir.exists() {
                    tokio::fs::remove_dir_all(&cache_dir).await?;
                }
            }
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
}

async fn find_component(
    components: Vec<ResolvedComponent>,
    component: &str,
) -> Result<ResolvedComponent> {
    components
        .into_iter()
        .find(|c| c.name == component)
        .ok_or_else(|| anyhow::anyhow!("Component '{}' not found in registry", component))
}

fn components_root(module_path: Option<&Path>) -> Result<PathBuf> {
    if let Some(module_path) = module_path {
        return Ok(PathBuf::from(module_path));
    }

    let root = Workspace::crate_root_from_path()?;

    Ok(root.join("src").join("components"))
}

fn global_assets_root(assets_path: Option<&Path>) -> Result<PathBuf> {
    if let Some(assets_path) = assets_path {
        return Ok(PathBuf::from(assets_path));
    }

    let root = Workspace::crate_root_from_path()?;

    Ok(root.join("assets"))
}

/// Remove a component from the managed component module
async fn remove_component(component: &ComponentArgs) -> Result<()> {
    let components_root = components_root(component.module_path.as_deref())?;

    // Remove the component module
    tokio::fs::remove_dir_all(&components_root.join(&component.component)).await?;

    // Remove the module from the components mod.rs
    let mod_rs_path = components_root.join("mod.rs");
    let mod_rs_content = tokio::fs::read_to_string(&mod_rs_path)
        .await
        .with_context(|| format!("Failed to read {}", mod_rs_path.display()))?;
    let mod_line = format!("pub mod {};\n", component.component);
    let new_mod_rs_content = mod_rs_content.replace(&mod_line, "");
    tokio::fs::write(&mod_rs_path, new_mod_rs_content)
        .await
        .with_context(|| format!("Failed to write to {}", mod_rs_path.display()))?;
    Ok(())
}

/// Add any rust dependencies required for a component
async fn add_rust_dependencies(dependencies: &[CargoDependency]) -> Result<()> {
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
            return Err(anyhow::anyhow!(
                "Failed to add dependency {} to Cargo.toml",
                dep.name()
            ));
        }
    }

    Ok(())
}

#[derive(Clone, Copy, Debug)]
enum ComponentExistsBehavior {
    /// Return an error (default)
    Error,
    /// Return early for component dependencies
    Return,
    /// Overwrite the existing component
    Overwrite,
}

async fn add_component(
    assets_path: Option<&Path>,
    component_path: Option<&Path>,
    component: &ResolvedComponent,
    behavior: ComponentExistsBehavior,
) -> Result<()> {
    add_rust_dependencies(&component.cargo_dependencies).await?;

    // Copy the folder content to the components directory
    let components_root = components_root(component_path)?;

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

    let assets_root = global_assets_root(assets_path)?;
    copy_global_assets(&assets_root, &component).await?;

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
                return Err(anyhow::anyhow!(
                    "Destination directory '{}' already exists",
                    dest.display()
                ));
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
            let path_relative_to_src = path.strip_prefix(&src).unwrap();
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
    tokio::fs::write(&mod_rs_path, "// AUTOGENERTED Components module\n").await?;

    Ok(true)
}

/// Read a component from the given path
async fn read_component(path: &PathBuf) -> Result<ResolvedComponent> {
    let json_path = path.join("component.json");
    let bytes = tokio::fs::read(&json_path).await.with_context(|| {
        format!(
            "Failed to open component manifest at {}",
            json_path.display()
        )
    })?;
    let component = serde_json::from_slice(&bytes)?;
    Ok(ResolvedComponent {
        path: path.clone(),
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
async fn copy_global_assets(assets_root: &Path, component: &ResolvedComponent) -> Result<()> {
    let cache_dir = Workspace::component_cache_dir();

    for path in &component.global_assets {
        let src = component.path.join(&path);
        let absolute_source = dunce::canonicalize(&src).with_context(|| {
            format!(
                "Failed to find global asset '{}' for component '{}'",
                src.display(),
                component.name
            )
        })?;
        if !absolute_source.starts_with(&cache_dir) {
            return Err(anyhow::anyhow!(
                "Cannot copy global asset '{}' for component '{}' because it is outside of the component registry",
                src.display(),
                component.name
            ));
        }
        let file = absolute_source
            .components()
            .last()
            .context("Global assets must have at least one file component")?;
        let dest = assets_root.join(&file);
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
