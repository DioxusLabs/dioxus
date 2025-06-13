use std::fs::{self, create_dir_all};
use std::path::{Path, PathBuf};

use clap::Parser;
use handlebars::Handlebars;
use serde_json::json;

use crate::{Result, StructuredOutput, Workspace};

/// Eject Android and iOS assets from the CLI to a local directory for customization
#[derive(Parser, Debug)]
pub struct Eject {
    /// Output directory for ejected assets (defaults to current directory)
    #[clap(short, long)]
    pub output_dir: Option<PathBuf>,

    /// Eject Android assets
    #[clap(long, default_value = "true")]
    pub android: bool,

    /// Eject iOS assets
    #[clap(long, default_value = "true")]
    pub ios: bool,

    /// Force overwrite existing files
    #[clap(short, long)]
    pub force: bool,
}

impl Eject {
    pub async fn eject(&self) -> Result<StructuredOutput> {
        // Check if we're in a Dioxus project
        let _workspace = Workspace::current().await?;
        if !self.is_dioxus_project(&_workspace) {
            return Err(
                "Not in a Dioxus project. Please run this command from a Dioxus project directory."
                    .into(),
            );
        }
        
        // Check if assets are already ejected
        let current_dir = std::env::current_dir()?;
        
        // Use the static has_ejected_assets method to check if assets are already ejected
        if crate::build::ejected_assets::EjectedAssets::has_ejected_assets(&current_dir) && !self.force {
            return Err(format!("Assets are already ejected. Use --force to overwrite.").into());
        }

        let output_dir = self
            .output_dir
            .clone()
            .unwrap_or_else(|| PathBuf::from("."));
        let assets_dir = output_dir.join("assets");

        println!("Ejecting assets to {}", assets_dir.display());

        // Create the assets directory if it doesn't exist
        create_dir_all(&assets_dir)?;

        // Eject Android assets if requested
        if self.android {
            self.eject_android_assets(&assets_dir).await?;
        }

        // Eject iOS assets if requested
        if self.ios {
            self.eject_ios_assets(&assets_dir).await?;
        }

        println!("Successfully ejected assets to {}", assets_dir.display());
        Ok(StructuredOutput::Success)
    }

    /// Check if the current directory is a Dioxus project
    fn is_dioxus_project(&self, workspace: &Workspace) -> bool {
        // Check for dioxus dependencies in the workspace
        !workspace.dioxus_versions().is_empty()
    }

    /// Eject Android assets
    async fn eject_android_assets(&self, assets_dir: &Path) -> Result<()> {
        let android_dir = assets_dir.join("android");
        create_dir_all(&android_dir)?;

        // Check if android assets directory already exists using platform_assets_dir
        let project_dir = std::env::current_dir()?;
        if crate::build::ejected_assets::EjectedAssets::platform_assets_dir(&project_dir, "android").is_some() && !self.force {
            println!("Using existing ejected Android assets in {}", android_dir.display());
        } else {
            // Copy assets from CLI package to output directory
            self.copy_assets_with_rendering(Path::new("../../assets/android"), &android_dir).await?
        }
        
        Ok(())
    }

    /// Eject iOS assets
    async fn eject_ios_assets(&self, assets_dir: &Path) -> Result<()> {
        let ios_dir = assets_dir.join("ios");
        create_dir_all(&ios_dir)?;

        // Check if iOS assets directory already exists using platform_assets_dir
        let project_dir = std::env::current_dir()?;
        if crate::build::ejected_assets::EjectedAssets::platform_assets_dir(&project_dir, "ios").is_some() && !self.force {
            println!("Using existing ejected iOS assets in {}", ios_dir.display());
        } else {
            // Copy assets from CLI package to output directory
            self.copy_assets_with_rendering(Path::new("../../assets/ios"), &ios_dir).await?
        }
        
        Ok(())
    }

    /// Copy assets from source to destination, rendering HBS templates
    async fn copy_assets_with_rendering(&self, src_dir: &Path, dest_dir: &Path) -> Result<()> {
        let src_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join(src_dir);
        if !src_dir.exists() {
            return Err(format!("Source directory {} does not exist", src_dir.display()).into());
        }

        // Get the workspace for project information
        let _workspace = Workspace::current().await?;

        // Setup Handlebars for template rendering
        let mut hbs = Handlebars::new();
        hbs.set_strict_mode(true);

        // Get the package name from the workspace
        let package_name = if let Ok(dir) = std::env::current_dir() {
            if let Some(name) = dir.file_name() {
                if let Some(name_str) = name.to_str() {
                    name_str.to_string()
                } else {
                    "dioxus-app".to_string()
                }
            } else {
                "dioxus-app".to_string()
            }
        } else {
            "dioxus-app".to_string()
        };

        // Platform-specific template data
        let hbs_data = if src_dir.ends_with("android") {
            // Android template data
            let application_id = format!("com.example.{}", package_name.replace("-", "_"));

            json!({
                "application_id": application_id,
                "app_name": package_name,
                "android_bundle": null
            })
        } else if src_dir.ends_with("ios") {
            // iOS template data
            let bundle_id = format!("com.example.{}", package_name.replace("-", "_"));

            json!({
                "bundle_id": bundle_id,
                "app_name": package_name
            })
        } else {
            // Generic template data for other platforms
            json!({
                "app_name": package_name
            })
        };

        // Walk the source directory and copy files
        self.copy_dir_recursive(&src_dir, dest_dir, &hbs, &hbs_data)?;

        Ok(())
    }

    /// Recursively copy a directory, rendering HBS templates
    fn copy_dir_recursive(
        &self,
        src: &Path,
        dest: &Path,
        hbs: &Handlebars,
        hbs_data: &serde_json::Value,
    ) -> Result<()> {
        if !dest.exists() {
            create_dir_all(dest)?;
        }

        for entry in fs::read_dir(src)? {
            let entry = entry?;
            let file_type = entry.file_type()?;
            let src_path = entry.path();
            let file_name = entry.file_name();
            let dest_path = dest.join(&file_name);

            if file_type.is_dir() {
                // Recursively copy subdirectories
                self.copy_dir_recursive(&src_path, &dest_path, hbs, hbs_data)?;
            } else {
                // Skip existing files unless force is specified
                if dest_path.exists() && !self.force {
                    println!("Skipping existing file: {}", dest_path.display());
                    continue;
                }

                // Check if this is an HBS template
                if src_path.extension().map_or(false, |ext| ext == "hbs") {
                    // Render the template
                    let template_content = fs::read_to_string(&src_path)?;
                    let rendered = hbs.render_template(&template_content, hbs_data)?;

                    // Write the rendered content to a file without the .hbs extension
                    let dest_path_without_hbs =
                        dest_path.with_file_name(file_name.to_string_lossy().replace(".hbs", ""));
                    fs::write(&dest_path_without_hbs, rendered)?;
                    println!(
                        "Rendered template: {} -> {}",
                        src_path.display(),
                        dest_path_without_hbs.display()
                    );
                } else {
                    // Regular file, just copy it
                    fs::copy(&src_path, &dest_path)?;
                    println!(
                        "Copied file: {} -> {}",
                        src_path.display(),
                        dest_path.display()
                    );
                }
            }
        }

        Ok(())
    }
}
