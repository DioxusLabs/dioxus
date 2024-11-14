use super::*;
use once_cell::sync::OnceCell;
use std::path::Path;

/// Information about the target to build
#[derive(Clone, Debug, Default, Deserialize, Parser)]
pub(crate) struct TargetArgs {
    /// Build for nightly [default: false]
    #[clap(long)]
    pub(crate) nightly: bool,

    /// Build a example [default: ""]
    #[clap(long)]
    pub(crate) example: Option<String>,

    /// Build a binary [default: ""]
    #[clap(long)]
    pub(crate) bin: Option<String>,

    /// The package to build
    #[clap(short, long)]
    pub(crate) package: Option<String>,

    /// Space separated list of features to activate
    #[clap(long)]
    pub(crate) features: Vec<String>,

    /// The feature to use for the client in a fullstack app [default: "web"]
    #[clap(long)]
    pub(crate) client_features: Vec<String>,

    /// The feature to use for the server in a fullstack app [default: "server"]
    #[clap(long)]
    pub(crate) server_features: Vec<String>,

    /// Don't include the default features in the build
    #[clap(long)]
    pub(crate) no_default_features: bool,

    /// The architecture to build for.
    #[clap(long, value_enum)]
    pub(crate) arch: Option<Arch>,

    /// Are we building for a device or just the simulator.
    /// If device is false, then we'll build for the simulator
    #[clap(long)]
    pub(crate) device: Option<bool>,

    /// Rustc platform triple
    #[clap(long)]
    pub(crate) target: Option<String>,
}

#[derive(Debug, Default, Copy, Clone, PartialEq, Deserialize, clap::ValueEnum)]
#[non_exhaustive]
pub(crate) enum Arch {
    // Android: armv7l, armv7-linux-androideabi
    Arm,
    // Android: aarch64, aarch64-linux-android
    #[default]
    Arm64,
    // Android: i386, i686-linux-android
    X86,
    // Android: x86_64, x86_64-linux-android
    X64,
}

impl Arch {
    pub(crate) fn autodetect() -> Option<Self> {
        // Try auto detecting arch through adb.
        static AUTO_ARCH: OnceCell<Option<Arch>> = OnceCell::new();

        *AUTO_ARCH.get_or_init(|| {
            // TODO: Wire this up with --device flag. (add `-s serial`` flag before `shell` arg)
            let output = std::process::Command::new("adb")
                .arg("shell")
                .arg("uname")
                .arg("-m")
                .output();

            let out = match output {
                Ok(o) => o,
                Err(e) => {
                    tracing::debug!("ADB command failed: {:?}", e);
                    return None;
                }
            };

            let Ok(out) = String::from_utf8(out.stdout) else {
                tracing::debug!("ADB returned unexpected data.");
                return None;
            };
            let trimmed = out.trim().to_string();
            tracing::trace!("ADB Returned: `{trimmed:?}`");

            Arch::try_from(trimmed).ok()
        })
    }

    pub(crate) fn android_target_triplet(&self) -> &'static str {
        match self {
            Arch::Arm => "armv7-linux-androideabi",
            Arch::Arm64 => "aarch64-linux-android",
            Arch::X86 => "i686-linux-android",
            Arch::X64 => "x86_64-linux-android",
        }
    }

    pub(crate) fn android_jnilib(&self) -> &'static str {
        match self {
            Arch::Arm => "armeabi-v7a",
            Arch::Arm64 => "arm64-v8a",
            Arch::X86 => "x86",
            Arch::X64 => "x86_64",
        }
    }

    pub(crate) fn android_clang_triplet(&self) -> &'static str {
        match self {
            Self::Arm => "armv7a-linux-androideabi",
            _ => self.android_target_triplet(),
        }
    }

    pub(crate) fn android_linker(&self, ndk: &Path) -> PathBuf {
        // "/Users/jonkelley/Library/Android/sdk/ndk/25.2.9519653/toolchains/llvm/prebuilt/darwin-x86_64/bin/aarch64-linux-android24-clang"

        let toolchain_dir = ndk.join("toolchains").join("llvm").join("prebuilt");
        let triplet = self.android_clang_triplet();
        let clang_exec = format!("{}24-clang", triplet);

        if cfg!(target_os = "macos") {
            // for whatever reason, even on aarch64 macos, the linker is under darwin-x86_64
            return toolchain_dir
                .join("darwin-x86_64")
                .join("bin")
                .join(clang_exec);
        }

        if cfg!(target_os = "linux") {
            return toolchain_dir
                .join("linux-x86_64")
                .join("bin")
                .join(clang_exec);
        }

        if cfg!(target_os = "windows") {
            return toolchain_dir
                .join("windows-x86_64")
                .join("bin")
                .join(format!("{}.cmd", clang_exec));
        }

        unimplemented!("Unsupported target os for android toolchain auodetection")
    }
}

impl TryFrom<String> for Arch {
    type Error = ();

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_str() {
            "armv7l" => Ok(Self::Arm),
            "aarch64" => Ok(Self::Arm64),
            "i386" => Ok(Self::X86),
            "x86_64" => Ok(Self::X64),
            _ => Err(()),
        }
    }
}
