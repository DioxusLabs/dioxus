use std::{
    ops::Deref,
    path::{Path, PathBuf},
};

use crate::{Result, StructuredOutput, Workspace};
use anyhow::Context;
use clap::Parser;
use dioxus_component_manifest::Component;
use tokio::task::JoinSet;
use tracing::debug;

#[derive(Clone, Debug, Parser)]
pub struct ComponentRegisteryArgs {
    /// The url of the the component registry
    #[arg(long, conflicts_with = "path")]
    git: Option<String>,
    /// The path to the components directory
    #[arg(long, conflicts_with = "git")]
    path: Option<String>,
}

impl ComponentRegisteryArgs {
    async fn resolve(&self) -> Result<PathBuf> {
        // If a path is provided, use that
        if let Some(path) = &self.path {
            return Ok(PathBuf::from(path));
        }

        todo!()
    }

    async fn read_components(&self) -> Result<Vec<ResolvedComponent>> {
        let path = self.resolve().await?;

        let root = read_component(&path).await?;
        discover_components(root).await
    }
}

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

#[derive(Clone, Debug, Parser)]
pub enum ComponentCommand {
    #[clap(name = "add")]
    Add {
        /// The component to add
        component: String,
        /// The registry to use
        #[clap(flatten)]
        registry: ComponentRegisteryArgs,
    },
    #[clap(name = "remove")]
    Remove {
        /// The component to remove
        component: String,
    },
    #[clap(name = "list")]
    List {
        /// The registry to use
        #[clap(flatten)]
        registry: ComponentRegisteryArgs,
    },
}

impl ComponentCommand {
    pub async fn run(self) -> Result<StructuredOutput> {
        match self {
            Self::List { registry } => {
                let components = registry.read_components().await?;
                for component in components {
                    println!("- {}: {}", component.name, component.description);
                }
            }
            Self::Add {
                component,
                registry,
            } => {
                let components = registry.read_components().await?;
                add_component(&components, &component).await?;
            }
            Self::Remove { component } => {
                remove_component(&component).await?;
            }
        }

        Ok(StructuredOutput::Success)
    }
}

async fn find_component<'a>(
    components: &'a [ResolvedComponent],
    component: &str,
) -> Result<&'a ResolvedComponent> {
    components
        .iter()
        .find(|c| c.name == component)
        .ok_or_else(|| anyhow::anyhow!("Component '{}' not found in registry", component))
}

fn components_root() -> Result<PathBuf> {
    let root = Workspace::crate_root_from_path()?;

    Ok(root.join("src").join("components"))
}

async fn remove_component(component: &str) -> Result<()> {
    let components_root = components_root()?;
    tokio::fs::remove_dir_all(&components_root.join(component)).await?;
    // Remove the module from the components mod.rs
    let mod_rs_path = components_root.join("mod.rs");
    let mod_rs_content = tokio::fs::read_to_string(&mod_rs_path)
        .await
        .with_context(|| format!("Failed to read {}", mod_rs_path.display()))?;
    let mod_line = format!("pub mod {};\n", component);
    let new_mod_rs_content = mod_rs_content.replace(&mod_line, "");
    tokio::fs::write(&mod_rs_path, new_mod_rs_content)
        .await
        .with_context(|| format!("Failed to write to {}", mod_rs_path.display()))?;
    Ok(())
}

async fn add_component(components: &[ResolvedComponent], component: &str) -> Result<()> {
    let component = find_component(components, component).await?;

    // Copy the folder content to the components directory
    let components_root = components_root()?;
    ensure_components_module_exists(&components_root).await?;

    copy_component_files(
        &component.path,
        &components_root.join(&component.name),
        &component.exclude,
    )
    .await?;

    // Add the module to the components mod.rs
    let mod_rs_path = components_root.join("mod.rs");
    let mut mod_rs = tokio::fs::OpenOptions::new()
        .append(true)
        .open(&mod_rs_path)
        .await
        .with_context(|| format!("Failed to open {}", mod_rs_path.display()))?;
    let mod_line = format!("pub mod {};\n", component.name);
    tokio::io::AsyncWriteExt::write_all(&mut mod_rs, mod_line.as_bytes())
        .await
        .with_context(|| format!("Failed to write to {}", mod_rs_path.display()))?;

    Ok(())
}

async fn copy_component_files(src: &Path, dest: &Path, exclude: &[String]) -> Result<()> {
    async fn read_dir_paths(src: &Path) -> Result<Vec<PathBuf>> {
        let mut entries = tokio::fs::read_dir(src).await?;
        let mut paths = vec![];
        while let Some(entry) = entries.next_entry().await? {
            paths.push(entry.path());
        }
        Ok(paths)
    }

    if dest.exists() {
        return Err(anyhow::anyhow!(
            "Destination directory '{}' already exists",
            dest.display()
        ));
    }
    tokio::fs::create_dir_all(dest).await?;

    let exclude = exclude
        .iter()
        .map(|exclude| dunce::canonicalize(src.join(exclude)))
        .collect::<Result<Vec<_>, _>>()?;

    let mut read_folder_tasks = JoinSet::new();
    let mut copy_tasks = JoinSet::new();

    let src = src.to_path_buf();
    read_folder_tasks.spawn({
        let src = src.clone();
        async move { read_dir_paths(&src).await }
    });

    loop {
        if let Some(res) = read_folder_tasks.join_next().await {
            let paths = res??;
            for path in paths {
                let path = dunce::canonicalize(path)?;
                if exclude.iter().any(|e| *e == path || path.starts_with(e)) {
                    debug!("Excluding path {}", path.display());
                    continue;
                }
                let path_relative_to_src = path.strip_prefix(&src).unwrap();
                let dest = dest.join(path_relative_to_src);
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
        } else {
            break;
        }
    }

    // Join all copy tasks
    while let Some(res) = copy_tasks.join_next().await {
        res??;
    }

    Ok(())
}

async fn ensure_components_module_exists(components_dir: &Path) -> Result<()> {
    if !components_dir.exists() {
        tokio::fs::create_dir_all(&components_dir).await?;
    }

    let mod_rs_path = components_dir.join("mod.rs");
    if !mod_rs_path.exists() {
        tokio::fs::write(&mod_rs_path, "// Components module\n").await?;
    }

    Ok(())
}

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

async fn discover_components(root: ResolvedComponent) -> Result<Vec<ResolvedComponent>> {
    let mut queue = root.member_paths();
    let mut components = vec![root];
    let mut pending = JoinSet::new();
    loop {
        while let Some(root_path) = queue.pop() {
            pending.spawn(async move { read_component(&root_path).await });
        }
        let Some(component) = pending.join_next().await else {
            break;
        };
        let component = component??;
        queue.extend(component.member_paths());
        components.push(component);
    }
    Ok(components)
}
