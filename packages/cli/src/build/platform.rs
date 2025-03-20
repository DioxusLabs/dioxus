use std::path::PathBuf;

use itertools::Itertools;
use target_lexicon::Triple;

/// The tools for Android (ndk, sdk, etc)
#[derive(Debug, Clone)]
pub struct AndroidTools {
    ndk: Option<PathBuf>,
    sdk: Option<PathBuf>,
    adb: Option<PathBuf>,
    java_home: Option<PathBuf>,
}

#[memoize::memoize]
pub fn android_tools() -> AndroidTools {
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
        });

    // Look for ADB in the SDK. If it's not there we'll use `adb` from the PATH
    let adb = sdk.as_ref().and_then(|sdk| {
        let tools = sdk.join("platform-tools");
        if tools.join("adb").exists() {
            return Some(tools.join("adb"));
        }
        if tools.join("adb.exe").exists() {
            return Some(tools.join("adb.exe"));
        }
        None
    });

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

    AndroidTools {
        sdk,
        ndk,
        adb,
        java_home,
    }
}

impl AndroidTools {
    pub(crate) fn ndk_exists(&self) -> bool {
        self.ndk.is_some()
    }

    pub(crate) fn adb(&self) -> PathBuf {
        self.adb.clone().unwrap_or_else(|| PathBuf::from("adb"))
    }

    pub(crate) fn android_tools_dir(&self) -> PathBuf {
        let prebuilt = self
            .ndk
            .as_ref()
            .expect("Android NDK not found")
            .join("toolchains")
            .join("llvm")
            .join("prebuilt");

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

    pub(crate) fn linker(&self, triple: &Triple) -> PathBuf {
        // "/Users/jonkelley/Library/Android/sdk/ndk/25.2.9519653/toolchains/llvm/prebuilt/darwin-x86_64/bin/aarch64-linux-android24-clang"
        let suffix = if cfg!(target_os = "windows") {
            ".cmd"
        } else {
            ""
        };

        self.android_tools_dir()
            .join(format!("{}24-clang{}", triple, suffix))
    }

    pub(crate) fn min_sdk_version(&self) -> u32 {
        // todo(jon): this should be configurable
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

    // pub(crate) fn android_target_triplet(&self) -> &'static str {
    //     match self {
    //         Arch::Arm => "armv7-linux-androideabi",
    //         Arch::Arm64 => "aarch64-linux-android",
    //         Arch::X86 => "i686-linux-android",
    //         Arch::X64 => "x86_64-linux-android",
    //     }
    // }

    // pub(crate) fn android_jnilib(&self) -> &'static str {
    //     match self {
    //         Arch::Arm => "armeabi-v7a",
    //         Arch::Arm64 => "arm64-v8a",
    //         Arch::X86 => "x86",
    //         Arch::X64 => "x86_64",
    //     }
    // }

    // pub(crate) fn android_clang_triplet(&self) -> &'static str {
    //     match self {
    //         Self::Arm => "armv7a-linux-androideabi",
    //         _ => self.android_target_triplet(),
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
