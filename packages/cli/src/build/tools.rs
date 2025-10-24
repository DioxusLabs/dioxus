use crate::Result;
use anyhow::Context;
use itertools::Itertools;
use std::{path::PathBuf, sync::Arc};
use target_lexicon::{
    Aarch64Architecture, Architecture, ArmArchitecture, Triple, X86_32Architecture,
};
use tokio::process::Command;

/// The tools for Android (ndk, sdk, etc)
///
/// <https://gist.github.com/Pulimet/5013acf2cd5b28e55036c82c91bd56d8?permalink_comment_id=3678614>
#[derive(Debug, Clone)]
pub(crate) struct AndroidTools {
    pub(crate) sdk: Option<PathBuf>,
    pub(crate) ndk: PathBuf,
    pub(crate) adb: PathBuf,
    pub(crate) java_home: Option<PathBuf>,
}

pub fn get_android_tools() -> Option<Arc<AndroidTools>> {
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
            // Look for the most recent NDK in the event the user has installed multiple NDK
            // Eventually we might need to drive this from Dioxus.toml
            let sdk = sdk.as_ref()?;
            let ndk_dir = sdk.join("ndk").read_dir().ok()?;
            ndk_dir
                .flatten()
                .map(|dir| (dir.file_name(), dir.path()))
                .sorted()
                .next_back()
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

    Some(Arc::new(AndroidTools {
        ndk,
        adb,
        java_home,
        sdk,
    }))
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

    /// Return the location of the clang toolchain for the given target triple.
    ///
    /// Note that we use clang:
    /// "~/Library/Android/sdk/ndk/25.2.9519653/toolchains/llvm/prebuilt/darwin-x86_64/bin/aarch64-linux-android24-clang"
    ///
    /// But if we needed the linker, we would use:
    /// "~/Library/Android/sdk/ndk/25.2.9519653/toolchains/llvm/prebuilt/darwin-x86_64/bin/ld"
    ///
    /// However, for our purposes, we only go through the cc driver and not the linker directly.
    pub(crate) fn android_cc(&self, triple: &Triple, sdk_version: u32) -> PathBuf {
        let suffix = if cfg!(target_os = "windows") {
            ".cmd"
        } else {
            ""
        };

        let target = match triple.architecture {
            Architecture::Arm(_) => "armv7a-linux-androideabi",
            _ => &triple.to_string(),
        };

        self.android_tools_dir()
            .join(format!("{}{}-clang{}", target, sdk_version, suffix))
    }

    pub(crate) fn sysroot(&self) -> PathBuf {
        // The sysroot is usually located in the NDK under:
        // "~/Library/Android/sdk/ndk/25.2.9519653/toolchains/llvm/prebuilt/darwin-x86_64/sysroot"
        // or similar, depending on the platform.
        self.android_tools_dir().parent().unwrap().join("sysroot")
    }

    pub(crate) fn sdk(&self) -> PathBuf {
        // /Users/jonathankelley/Library/Android/sdk/ndk/25.2/... (25.2 is the ndk here)
        // /Users/jonathankelley/Library/Android/sdk/
        self.sdk
            .clone()
            .unwrap_or_else(|| self.ndk.parent().unwrap().parent().unwrap().to_path_buf())
    }

    pub(crate) fn emulator(&self) -> PathBuf {
        self.sdk().join("emulator").join("emulator")
    }

    pub(crate) fn clang_folder(&self) -> PathBuf {
        // The clang folder is usually located in the NDK under:
        // "~/Library/Android/sdk/ndk/25.2.9519653/toolchains/llvm/prebuilt/darwin-x86_64/lib/clang/<version>"
        // or similar, depending on the platform.
        self.android_tools_dir()
            .parent()
            .unwrap()
            .join("lib")
            .join("clang")
    }

    pub(crate) fn ranlib(&self) -> PathBuf {
        self.android_tools_dir().join("llvm-ranlib")
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
    }

    pub(crate) fn android_jnilib(triple: &Triple) -> &'static str {
        use target_lexicon::Architecture;
        match triple.architecture {
            Architecture::Arm(_) => "armeabi-v7a",
            Architecture::Aarch64(_) => "arm64-v8a",
            Architecture::X86_32(_) => "x86",
            Architecture::X86_64 => "x86_64",
            _ => unimplemented!("Unsupported architecture"),
        }
    }

    pub(crate) async fn autodetect_android_device_triple(&self) -> Triple {
        // Use the host's triple and then convert field by field
        // ie, the "best" emulator for an m1 mac would be: "aarch64-linux-android"
        //  - We assume android is always "linux"
        //  - We try to match the architecture unless otherwise specified. This is because
        //    emulators that match the host arch are usually faster.
        let mut triple = "aarch64-linux-android".parse::<Triple>().unwrap();

        // TODO: Wire this up with --device flag. (add `-s serial`` flag before `shell` arg)
        let output = Command::new(&self.adb)
            .arg("shell")
            .arg("uname")
            .arg("-m")
            .output()
            .await
            .map(|out| String::from_utf8(out.stdout));

        match output {
            Ok(Ok(out)) => match out.trim() {
                "armv7l" | "armv8l" => {
                    triple.architecture = Architecture::Arm(ArmArchitecture::Arm)
                }
                "aarch64" => {
                    triple.architecture = Architecture::Aarch64(Aarch64Architecture::Aarch64)
                }
                "i386" => triple.architecture = Architecture::X86_32(X86_32Architecture::I386),
                "x86_64" => {
                    triple.architecture = Architecture::X86_64;
                }
                "" => {
                    tracing::debug!("No device running - probably waiting for emulator");
                }
                other => {
                    tracing::debug!("Unknown architecture from adb: {other}");
                }
            },
            Ok(Err(err)) => {
                tracing::debug!("Failed to parse adb output: {err}");
            }
            Err(err) => {
                tracing::debug!("ADB command failed: {:?}", err);
            }
        };

        triple
    }

    pub(crate) fn libcpp_shared(&self, triple: &Triple) -> PathBuf {
        // The libc++_shared.so is usually located in the sysroot under:
        // "~/Library/Android/sdk/ndk/25.2.9519653/toolchains/llvm/prebuilt/darwin-x86_64/sysroot/usr/lib/<arch>/libc++_shared.so"
        // or similar, depending on the platform.
        self.sysroot()
            .join("usr")
            .join("lib")
            .join(Self::sysroot_target(&triple.to_string()))
            .join("libc++_shared.so")
    }

    pub(crate) fn sysroot_target(rust_target: &str) -> &str {
        (match rust_target {
            "armv7-linux-androideabi" => "arm-linux-androideabi",
            _ => rust_target,
        }) as _
    }

    pub(crate) fn openssl_prebuilt_aar() -> &'static [u8] {
        include_bytes!("../../assets/android/prebuilt/openssl-1.1.1q-beta-1.tar.gz")
    }

    pub(crate) fn openssl_prebuilt_dest() -> PathBuf {
        crate::Workspace::dioxus_data_dir()
            .join("prebuilt")
            .join("openssl-1.1.1q-beta-1")
    }

    pub(crate) fn openssl_lib_dir(arch: &Triple) -> PathBuf {
        let libs_dir = Self::openssl_prebuilt_dest().join("ssl").join("libs");

        match arch.architecture {
            Architecture::Arm(_) => libs_dir.join("android.armeabi-v7a"),
            Architecture::Aarch64(_) => libs_dir.join("android.arm64-v8a"),
            Architecture::X86_32(_) => libs_dir.join("android.x86"),
            Architecture::X86_64 => libs_dir.join("android.x86_64"),
            _ => libs_dir.join("android.arm64-v8a"), // Default to arm64-v8a
        }
    }

    pub(crate) fn openssl_include_dir() -> PathBuf {
        Self::openssl_prebuilt_dest().join("ssl").join("include")
    }

    /// Unzip the prebuilt OpenSSL AAR file into the `.dx/prebuilt/openssl-<version>` directory
    pub(crate) fn unpack_prebuilt_openssl() -> Result<()> {
        let raw_aar = AndroidTools::openssl_prebuilt_aar();
        let aar_dest = AndroidTools::openssl_prebuilt_dest();

        if aar_dest.exists() {
            tracing::trace!("Prebuilt OpenSSL already exists at {:?}", aar_dest);
            return Ok(());
        }

        std::fs::create_dir_all(aar_dest.parent().context("no parent for aar")?)
            .context("failed to create prebuilt OpenSSL directory")?;

        // Unpack the entire tar.gz file into the destination directory
        let mut archive = tar::Archive::new(flate2::read::GzDecoder::new(raw_aar as &[u8]));
        archive
            .unpack(aar_dest.parent().context("no parent for aar dest")?)
            .context("failed to unpack prebuilt OpenSSL archive")?;

        tracing::debug!("Unpacked prebuilt OpenSSL to {:?}", aar_dest);

        Ok(())
    }
}

fn var_or_debug(name: &str) -> Option<PathBuf> {
    use std::env::var;

    var(name)
        .inspect_err(|_| tracing::trace!("{name} not set"))
        .ok()
        .map(PathBuf::from)
}
