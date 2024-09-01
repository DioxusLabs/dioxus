use std::str::FromStr;

/// The target platform for the build
/// This is very similar to the Platform enum, but we need to be able to differentiate between the
/// server and web targets for the fullstack platform
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TargetPlatform {
    Web,
    Desktop,
    Mobile,
    Server,
    Liveview,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TargetArch {
    Linux,
    Mac,
    Windows,
    Ios,
    Android,
}

impl FromStr for TargetPlatform {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "web" => Ok(Self::Web),
            "desktop" => Ok(Self::Desktop),
            "axum" | "server" => Ok(Self::Server),
            "liveview" => Ok(Self::Liveview),
            _ => Err(()),
        }
    }
}

impl std::fmt::Display for TargetPlatform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TargetPlatform::Web => write!(f, "web"),
            TargetPlatform::Desktop => write!(f, "desktop"),
            TargetPlatform::Server => write!(f, "server"),
            TargetPlatform::Liveview => write!(f, "liveview"),
            TargetPlatform::Mobile => write!(f, "ios"),
        }
    }
}
