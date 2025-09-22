use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::component::ComponentRegistry;

/// Configuration for the `dioxus component` commands
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub(crate) struct ComponentConfig {
    /// The component registry to default to when adding components
    #[serde(default)]
    pub(crate) registry: ComponentRegistry,
    /// The path where components are stored when adding or removing components
    #[serde(default)]
    pub(crate) component_path: Option<PathBuf>,
}
