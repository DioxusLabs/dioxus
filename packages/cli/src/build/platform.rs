use crate::Result;
use anyhow::Context;
use itertools::Itertools;
use std::{path::PathBuf, sync::Arc};
use target_lexicon::Triple;

/// The tools for Android (ndk, sdk, etc)
#[derive(Debug, Clone)]
pub(crate) struct AndroidTools {
    pub(crate) ndk: PathBuf,
    pub(crate) adb: PathBuf,
    pub(crate) java_home: Option<PathBuf>,
}

#[memoize::memoize]
pub fn android_tools() -> Option<AndroidTools> {
    // We check for SDK first since users might install Android Studio and then install the SDK
    // After that they might install the NDK, so the SDK drives the source of truth.
    let sdk = var_or_debug("ANDROID_SDK_ROOT")
        .or_else(|| var_or_debug("ANDROID_SDK"))
        .or_else(|| var_or_debug("ANDROID_HOME"));

    // Check the ndk. We look for users's overrides first and then look into the SDK.
    // Sometimes users set only the NDK (especially if they're somewhat advanced) so we need to look for it manually
    //
    // Might look like this, typically under "sdk":
    // "/Users/jonkelley/Library/Android/sdk/ndk/25.2.9519653/toolchains/llvm/prebuilt/darwin-x86_64/bin/aarch64-linux-android24-clang"
    let ndk = var_or_debug("NDK_HOME")
        .or_else(|| var_or_debug("ANDROID_NDK_HOME"))
        .or_else(|| {
            // Look for the most recent NDK in the event the user has installed multiple NDKs
            // Eventually we might need to drive this from Dioxus.toml
            let sdk = sdk.as_ref()?;
            let ndk_dir = sdk.join("ndk").read_dir().ok()?;
            ndk_dir
                .flatten()
                .map(|dir| (dir.file_name(), dir.path()))
                .sorted()
                .last()
                .map(|(_, path)| path.to_path_buf())
        })?;

    // Look for ADB in the SDK. If it's not there we'll use `adb` from the PATH
    let adb = sdk
        .as_ref()
        .and_then(|sdk| {
            let tools = sdk.join("platform-tools");
            if tools.join("adb").exists() {
                return Some(tools.join("adb"));
            }
            if tools.join("adb.exe").exists() {
                return Some(tools.join("adb.exe"));
            }
            None
        })
        .unwrap_or_else(|| PathBuf::from("adb"));

    // https://stackoverflow.com/questions/71381050/java-home-is-set-to-an-invalid-directory-android-studio-flutter
    // always respect the user's JAVA_HOME env var above all other options
    //
    // we only attempt autodetection if java_home is not set
    //
    // this is a better fallback than falling onto the users' system java home since many users might
    // not even know which java that is - they just know they have android studio installed
    let java_home = std::env::var_os("JAVA_HOME")
        .map(PathBuf::from)
        .or_else(|| {
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
        });

    Some(AndroidTools {
        ndk,
        adb,
        java_home,
    })
}

impl AndroidTools {
    pub(crate) fn android_tools_dir(&self) -> PathBuf {
        let prebuilt = self.ndk.join("toolchains").join("llvm").join("prebuilt");

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

        // Otherwise return the first entry in the prebuilt directory
        prebuilt
            .read_dir()
            .expect("Failed to read android toolchains directory")
            .next()
            .expect("Failed to find android toolchains directory")
            .expect("Failed to read android toolchain file")
            .path()
    }

    pub(crate) fn linker(&self, triple: &Triple) -> PathBuf {
        // "/Users/jonkelley/Library/Android/sdk/ndk/25.2.9519653/toolchains/llvm/prebuilt/darwin-x86_64/bin/aarch64-linux-android24-clang"
        let suffix = if cfg!(target_os = "windows") {
            ".cmd"
        } else {
            ""
        };

        self.android_tools_dir().join(format!(
            "{}{}-clang{}",
            triple,
            self.min_sdk_version(),
            suffix
        ))
    }

    // todo(jon): this should be configurable
    pub(crate) fn min_sdk_version(&self) -> u32 {
        24
    }

    pub(crate) fn ar_path(&self) -> PathBuf {
        self.android_tools_dir().join("llvm-ar")
    }

    pub(crate) fn target_cc(&self) -> PathBuf {
        self.android_tools_dir().join("clang")
    }

    pub(crate) fn target_cxx(&self) -> PathBuf {
        self.android_tools_dir().join("clang++")
    }

    pub(crate) fn java_home(&self) -> Option<PathBuf> {
        self.java_home.clone()
        // copilot suggested this??
        // self.ndk.join("platforms").join("android-24").join("arch-arm64").join("usr").join("lib")
        //     .join("jvm")
        //     .join("default")
        //     .join("lib")
        //     .join("server")
        //     .join("libjvm.so")
    }

    pub(crate) fn android_jnilib(triple: &Triple) -> &'static str {
        use target_lexicon::Architecture;
        match triple.architecture {
            Architecture::Aarch64(_) => "arm64-v8a",
            Architecture::Arm(_) => "armeabi-v7a",
            Architecture::X86_32(_) => "x86",
            Architecture::X86_64 => "x86_64",
            _ => todo!("Unsupported architecture"),
        }
    }

    // todo: the new Triple type might be able to handle the different arm flavors
    // ie armv7 vs armv7a
    pub(crate) fn android_clang_triplet(triple: &Triple) -> String {
        use target_lexicon::Architecture;
        match triple.architecture {
            Architecture::Arm(_) => "armv7a-linux-androideabi".to_string(),
            _ => triple.to_string(),
        }
    }

    // pub(crate) fn android_target_triplet(&self) -> &'static str {
    //     match self {
    //         Arch::Arm => "armv7-linux-androideabi",
    //         Arch::Arm64 => "aarch64-linux-android",
    //         Arch::X86 => "i686-linux-android",
    //         Arch::X64 => "x86_64-linux-android",
    //     }
    // }
}

fn var_or_debug(name: &str) -> Option<PathBuf> {
    use std::env::var;
    use tracing::debug;

    var(name)
        .inspect_err(|_| debug!("{name} not set"))
        .ok()
        .map(PathBuf::from)
}
