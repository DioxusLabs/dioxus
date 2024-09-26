use serde::{Deserialize, Serialize};

/// Represents configuration items for the desktop platform.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub(crate) struct DesktopConfig {}
