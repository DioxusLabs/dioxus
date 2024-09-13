use crate::builder::TargetPlatform;
use std::fmt::Display;
use tracing::Level;

#[derive(Clone, PartialEq)]
pub struct Message {
    pub source: TraceSrc,
    pub level: Level,
    pub content: String,
}

impl Message {
    pub fn new(source: TraceSrc, level: Level, content: String) -> Self {
        Self {
            source,
            level,
            content,
        }
    }
}

#[derive(Clone, PartialEq)]
pub enum TraceSrc {
    App(TargetPlatform),
    Dev,
    Build,
    /// Provides no formatting.
    Cargo,
    /// Avoid using this
    Unknown,
}

impl std::fmt::Debug for TraceSrc {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let as_string = self.to_string();
        write!(f, "{as_string}")
    }
}

impl From<String> for TraceSrc {
    fn from(value: String) -> Self {
        match value.as_str() {
            "dev" => Self::Dev,
            "build" => Self::Build,
            "cargo" => Self::Cargo,
            "web" => Self::App(TargetPlatform::Web),
            "desktop" => Self::App(TargetPlatform::Desktop),
            "server" => Self::App(TargetPlatform::Server),
            "liveview" => Self::App(TargetPlatform::Liveview),
            _ => Self::Unknown,
        }
    }
}

impl Display for TraceSrc {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::App(platform) => match platform {
                TargetPlatform::Web => write!(f, "web"),
                TargetPlatform::Desktop => write!(f, "desktop"),
                TargetPlatform::Server => write!(f, "server"),
                TargetPlatform::Liveview => write!(f, "server"),
            },
            Self::Dev => write!(f, "dev"),
            Self::Build => write!(f, "build"),
            Self::Cargo => write!(f, "cargo"),
            Self::Unknown => write!(f, "n/a"),
        }
    }
}
