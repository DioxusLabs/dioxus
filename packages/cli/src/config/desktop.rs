use serde::{Deserialize, Serialize};

/// Represents configuration items for the desktop platform.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesktopConfig {
    /// Describes whether a debug-mode desktop app should be always-on-top.
    #[serde(default)]
    pub always_on_top: bool,
}

impl Default for DesktopConfig {
    fn default() -> Self {
        Self {
            always_on_top: true,
        }
    }
}
