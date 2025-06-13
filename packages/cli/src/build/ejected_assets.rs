use std::path::{Path, PathBuf};

/// Utility struct for working with ejected assets
pub struct EjectedAssets {
    project_dir: PathBuf,
}

impl EjectedAssets {
    /// Create a new EjectedAssets instance with a specific project directory
    pub fn with_project_dir(project_dir: PathBuf) -> Self {
        Self { project_dir }
    }
    
    /// Get the ejected path for an asset if it exists
    pub fn get_ejected_path(&self, asset_path: &str) -> Option<PathBuf> {
        // Check if this is an Android or iOS asset
        if asset_path.contains("android") {
            if let Some(android_dir) = Self::android_assets_dir(&self.project_dir) {
                let relative_path = asset_path.split('/').last()?;
                let ejected_path = android_dir.join(relative_path);
                if ejected_path.exists() {
                    return Some(ejected_path);
                }
            }
        } else if asset_path.contains("ios") {
            if let Some(ios_dir) = Self::ios_assets_dir(&self.project_dir) {
                let relative_path = asset_path.split('/').last()?;
                let ejected_path = ios_dir.join(relative_path);
                if ejected_path.exists() {
                    return Some(ejected_path);
                }
            }
        }
        
        None
    }
    
    /// Check if there are ejected assets in the project directory
    pub fn has_ejected_assets(project_dir: &Path) -> bool {
        Self::android_assets_dir(project_dir).is_some() || Self::ios_assets_dir(project_dir).is_some()
    }

    /// Check if there are ejected Android assets in the project directory
    pub fn android_assets_dir(project_dir: &Path) -> Option<PathBuf> {
        // First check for root-level android folder
        let android_dir_root = project_dir.join("android");
        if android_dir_root.exists() && android_dir_root.is_dir() {
            return Some(android_dir_root);
        }
        
        // Fall back to assets/android for backward compatibility
        let android_dir = project_dir.join("assets").join("android");
        if android_dir.exists() && android_dir.is_dir() {
            Some(android_dir)
        } else {
            None
        }
    }

    /// Check if there are ejected iOS assets in the project directory
    pub fn ios_assets_dir(project_dir: &Path) -> Option<PathBuf> {
        // First check for root-level ios folder
        let ios_dir_root = project_dir.join("ios");
        if ios_dir_root.exists() && ios_dir_root.is_dir() {
            return Some(ios_dir_root);
        }
        
        // Fall back to assets/ios for backward compatibility
        let ios_dir = project_dir.join("assets").join("ios");
        if ios_dir.exists() && ios_dir.is_dir() {
            Some(ios_dir)
        } else {
            None
        }
    }

    /// Get the ejected assets directory for a specific platform
    /// 
    /// # Arguments
    /// 
    /// * `project_dir` - The project directory
    /// * `platform` - The platform name ("android" or "ios")
    pub fn platform_assets_dir(project_dir: &Path, platform: &str) -> Option<PathBuf> {
        // First check for root-level platform folder
        let platform_dir_root = project_dir.join(platform);
        if platform_dir_root.exists() && platform_dir_root.is_dir() {
            return Some(platform_dir_root);
        }
        
        // Fall back to assets/platform for backward compatibility
        let platform_dir = project_dir.join("assets").join(platform);
        if platform_dir.exists() && platform_dir.is_dir() {
            Some(platform_dir)
        } else {
            None
        }
    }
}
