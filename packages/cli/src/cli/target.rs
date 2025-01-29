use super::*;
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

impl TargetArgs {
    pub(crate) fn arch(&self) -> Arch {
        self.arch.unwrap_or_default()
    }
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

    pub(crate) fn android_tools_dir(&self, ndk: &Path) -> PathBuf {
        let prebuilt = ndk.join("toolchains").join("llvm").join("prebuilt");

        if cfg!(target_os = "macos") {
            // for whatever reason, even on aarch64 macos, the linker is under darwin-x86_64
            return prebuilt.join("darwin-x86_64").join("bin");
        }

        if cfg!(target_os = "linux") {
            return prebuilt.join("linux-x86_64").join("bin");
        }

        if cfg!(target_os = "windows") {
            return prebuilt.join("windows-x86_64").join("bin");
        }

        unimplemented!("Unsupported target os for android toolchain autodetection")
    }

    pub(crate) fn android_linker(&self, ndk: &Path) -> PathBuf {
        // "/Users/jonkelley/Library/Android/sdk/ndk/25.2.9519653/toolchains/llvm/prebuilt/darwin-x86_64/bin/aarch64-linux-android24-clang"
        let triplet = self.android_clang_triplet();
        let suffix = if cfg!(target_os = "windows") {
            ".cmd"
        } else {
            ""
        };

        self.android_tools_dir(ndk)
            .join(format!("{}24-clang{}", triplet, suffix))
    }

    pub(crate) fn android_min_sdk_version(&self) -> u32 {
        // todo(jon): this should be configurable
        24
    }

    pub(crate) fn android_ar_path(&self, ndk: &Path) -> PathBuf {
        self.android_tools_dir(ndk).join("llvm-ar")
    }

    pub(crate) fn target_cc(&self, ndk: &Path) -> PathBuf {
        self.android_tools_dir(ndk).join("clang")
    }

    pub(crate) fn target_cxx(&self, ndk: &Path) -> PathBuf {
        self.android_tools_dir(ndk).join("clang++")
    }

    pub(crate) fn java_home(&self) -> Option<PathBuf> {
        // wrap in a lazy so we don't accidentally keep probing for java home and potentially thrash env vars
        once_cell::sync::Lazy::new(|| {
            // https://stackoverflow.com/questions/71381050/java-home-is-set-to-an-invalid-directory-android-studio-flutter
            // always respect the user's JAVA_HOME env var above all other options
            //
            // we only attempt autodetection if java_home is not set
            //
            // this is a better fallback than falling onto the users' system java home since many users might
            // not even know which java that is - they just know they have android studio installed
            if let Some(java_home) = std::env::var_os("JAVA_HOME") {
                return Some(PathBuf::from(java_home));
            }

            // Attempt to autodetect java home from the android studio path or jdk path on macos
            #[cfg(target_os = "macos")]
            {
                let jbr_home =
                    PathBuf::from("/Applications/Android Studio.app/Contents/jbr/Contents/Home/");
                if jbr_home.exists() {
                    return Some(jbr_home);
                }

                let jre_home =
                    PathBuf::from("/Applications/Android Studio.app/Contents/jre/Contents/Home");
                if jre_home.exists() {
                    return Some(jre_home);
                }

                let jdk_home =
                    PathBuf::from("/Library/Java/JavaVirtualMachines/openjdk.jdk/Contents/Home/");
                if jdk_home.exists() {
                    return Some(jdk_home);
                }
            }

            #[cfg(target_os = "windows")]
            {
                let jbr_home = PathBuf::from("C:\\Program Files\\Android\\Android Studio\\jbr");
                if jbr_home.exists() {
                    return Some(jbr_home);
                }
            }

            // todo(jon): how do we detect java home on linux?
            #[cfg(target_os = "linux")]
            {
                let jbr_home = PathBuf::from("/usr/lib/jvm/java-11-openjdk-amd64");
                if jbr_home.exists() {
                    return Some(jbr_home);
                }
            }

            None
        })
        .clone()
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

impl std::fmt::Display for Arch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Arch::Arm => "armv7l",
            Arch::Arm64 => "aarch64",
            Arch::X86 => "i386",
            Arch::X64 => "x86_64",
        }
        .fmt(f)
    }
}
