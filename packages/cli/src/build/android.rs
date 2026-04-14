//! ### Android:
//!
//! Currently we need to generate a `src` type structure, not a pre-packaged apk structure, since
//! we need to compile kotlin and java. This pushes us into using gradle and following a structure
//! similar to that of cargo mobile2. Eventually I'd like to slim this down (drop buildSrc) and
//! drive the kotlin build ourselves. This would let us drop gradle (yay! no plugins!) but requires
//! us to manage dependencies (like kotlinc) ourselves (yuck!).
//!
//! <https://github.com/WanghongLin/miscellaneous/blob/master/tools/build-apk-manually.sh>
//!
//! Unfortunately, it seems that while we can drop the `android` build plugin, we still will need
//! gradle since kotlin is basically gradle-only.
//!
//! Pre-build:
//! ```
//! app.apk/
//!     .gradle
//!     app/
//!         src/
//!             main/
//!                 assets/
//!                 jniLibs/
//!                 java/
//!                 kotlin/
//!                 res/
//!                 AndroidManifest.xml
//!             build.gradle.kts
//!             proguard-rules.pro
//!         buildSrc/
//!             build.gradle.kts
//!             src/
//!                 main/
//!                     kotlin/
//!                          BuildTask.kt
//!     build.gradle.kts
//!     gradle.properties
//!     gradlew
//!     gradlew.bat
//!     settings.gradle
//! ```
//!
//! Final build:
//! ```
//! app.apk/
//!   AndroidManifest.xml
//!   classes.dex
//!   assets/
//!       logo.png
//!   lib/
//!       armeabi-v7a/
//!           libmyapp.so
//!       arm64-v8a/
//!           libmyapp.so
//!       x86/
//!           libmyapp.so
//!       x86_64/
//!           libmyapp.so
//! ```
//! Notice that we *could* feasibly build this ourselves :)

use crate::BuildRequest;
use crate::{BuildContext, Result};
use anyhow::{bail, Context};
use itertools::Itertools;
use manganis_core::AndroidArtifactMetadata;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::{borrow::Cow, ffi::OsString};
use target_lexicon::{
    Aarch64Architecture, Architecture, ArmArchitecture, Triple, X86_32Architecture,
};
use tokio::process::Command;

impl BuildRequest {
    /// Assemble the android app dir.
    ///
    /// This is a bit of a mess since we need to create a lot of directories and files. Other approaches
    /// would be to unpack some zip folder or something stored via `include_dir!()`. However, we do
    /// need to customize the whole setup a bit, so it's just simpler (though messier) to do it this way.
    pub(crate) fn build_android_app_dir(&self) -> Result<()> {
        use std::fs::{create_dir_all, write};
        let root = self.root_dir();

        // gradle
        let wrapper = root.join("gradle").join("wrapper");
        create_dir_all(&wrapper)?;

        // app
        let app = root.join("app");
        let app_main = app.join("src").join("main");
        let app_kotlin = app_main.join("kotlin");
        let app_java = app_main.join("java");
        let app_jnilibs = app_main.join("jniLibs");
        let app_assets = app_main.join("assets");
        let app_kotlin_out = self.wry_android_kotlin_files_out_dir();
        create_dir_all(&app)?;
        create_dir_all(&app_main)?;
        create_dir_all(&app_kotlin)?;
        create_dir_all(&app_java)?;
        create_dir_all(&app_jnilibs)?;
        create_dir_all(&app_assets)?;
        create_dir_all(&app_kotlin_out)?;

        tracing::debug!(
            r#"Initialized android dirs:
- gradle:              {wrapper:?}
- app/                 {app:?}
- app/src:             {app_main:?}
- app/src/kotlin:      {app_kotlin:?}
- app/src/jniLibs:     {app_jnilibs:?}
- app/src/assets:      {app_assets:?}
- app/src/kotlin/main: {app_kotlin_out:?}
"#
        );

        // handlebars
        #[derive(Serialize)]
        struct AndroidHandlebarsObjects {
            application_id: String,
            app_name: String,
            version: String,
            android_bundle: Option<crate::AndroidSettings>,
            /// Android SDK version settings
            min_sdk: u32,
            target_sdk: u32,
            compile_sdk: u32,
            /// Android permission strings (e.g., "android.permission.CAMERA")
            permissions: Vec<String>,
            /// Android hardware features (e.g., "android.hardware.location.gps")
            features: Vec<String>,
            /// Raw manifest XML to inject
            raw_manifest: String,
            /// URL schemes for deep linking
            url_schemes: Vec<String>,
            /// App link hosts for auto-verified deep links
            app_link_hosts: Vec<String>,
            /// Pipe-joined foreground service type string (e.g., "location|mediaPlayback")
            foreground_service_type: String,
            /// Extra Gradle dependencies from [android] config
            gradle_dependencies: Vec<String>,
            /// Extra Gradle plugins from [android] config
            gradle_plugins: Vec<String>,
            /// Application-level manifest attributes from [android.application]
            uses_cleartext_traffic: Option<bool>,
            app_theme: Option<String>,
            supports_rtl: Option<bool>,
            large_heap: Option<bool>,
            /// Native library name (without lib prefix and .so extension)
            lib_name: String,
        }

        // Get permission mapper from config
        let mapper = crate::ManifestMapper::from_config(
            &self.config.permissions,
            &self.config.deep_links,
            &self.config.background,
            &self.config.android,
            &self.config.ios,
            &self.config.macos,
        );

        // Collect Android permissions
        let permissions: Vec<String> = mapper
            .android_permissions
            .iter()
            .map(|p| p.permission.clone())
            .collect();

        // Collect Android features from config
        let features = self.config.android.features.clone();

        // Get raw manifest XML
        let raw_manifest = self.config.android.raw.manifest.clone().unwrap_or_default();

        // Foreground service types as pipe-separated string
        let foreground_service_type = mapper.android_foreground_service_types.join("|");

        let hbs_data = AndroidHandlebarsObjects {
            application_id: self.bundle_identifier(),
            app_name: self.bundled_app_name(),
            version: self.crate_version(),
            android_bundle: self.config.bundle.android.clone(),
            min_sdk: self.config.android.min_sdk.unwrap_or(24),
            target_sdk: self.config.android.target_sdk.unwrap_or(34),
            compile_sdk: self.config.android.compile_sdk.unwrap_or(34),
            permissions,
            features,
            raw_manifest,
            url_schemes: mapper.android_url_schemes,
            app_link_hosts: mapper.android_app_link_hosts,
            foreground_service_type,
            gradle_dependencies: self.config.android.gradle_dependencies.clone(),
            gradle_plugins: self.config.android.gradle_plugins.clone(),
            uses_cleartext_traffic: self.config.android.application.uses_cleartext_traffic,
            app_theme: self.config.android.application.theme.clone(),
            supports_rtl: self.config.android.application.supports_rtl,
            large_heap: self.config.android.application.large_heap,
            lib_name: self.android_lib_name(),
        };
        let hbs = handlebars::Handlebars::new();

        // Top-level gradle config
        write(
            root.join("build.gradle.kts"),
            include_bytes!("../../assets/android/gen/build.gradle.kts"),
        )?;
        write(
            root.join("gradle.properties"),
            include_bytes!("../../assets/android/gen/gradle.properties"),
        )?;
        write(
            root.join("gradlew"),
            include_bytes!("../../assets/android/gen/gradlew"),
        )?;
        write(
            root.join("gradlew.bat"),
            include_bytes!("../../assets/android/gen/gradlew.bat"),
        )?;
        write(
            root.join("settings.gradle"),
            include_bytes!("../../assets/android/gen/settings.gradle"),
        )?;

        // Then the wrapper and its properties
        write(
            wrapper.join("gradle-wrapper.properties"),
            include_bytes!("../../assets/android/gen/gradle/wrapper/gradle-wrapper.properties"),
        )?;
        write(
            wrapper.join("gradle-wrapper.jar"),
            include_bytes!("../../assets/android/gen/gradle/wrapper/gradle-wrapper.jar"),
        )?;

        // Now the app directory
        write(
            app.join("build.gradle.kts"),
            hbs.render_template(
                include_str!("../../assets/android/gen/app/build.gradle.kts.hbs"),
                &hbs_data,
            )?,
        )?;
        write(
            app.join("proguard-rules.pro"),
            include_bytes!("../../assets/android/gen/app/proguard-rules.pro"),
        )?;

        // Copy additional ProGuard rule files from Dioxus.toml [android] config
        for rule_file in &self.config.android.proguard_rules {
            let src = self.package_manifest_dir().join(rule_file);
            if src.exists() {
                let dest_name = src
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                std::fs::copy(&src, app.join(&dest_name))?;
                tracing::debug!("Copied ProGuard rules: {}", dest_name);
            } else {
                tracing::warn!("ProGuard rules file not found: {}", src.display());
            }
        }

        let manifest_xml = match self.config.application.android_manifest.as_deref() {
            Some(manifest) => std::fs::read_to_string(self.package_manifest_dir().join(manifest))
                .context("Failed to locate custom AndroidManifest.xml")?,
            _ => hbs.render_template(
                include_str!("../../assets/android/gen/app/src/main/AndroidManifest.xml.hbs"),
                &hbs_data,
            )?,
        };

        write(
            app.join("src").join("main").join("AndroidManifest.xml"),
            manifest_xml,
        )?;

        // Write the main activity manually since tao dropped support for it
        let main_activity = match self.config.application.android_main_activity.as_deref() {
            Some(activity) => std::fs::read_to_string(self.package_manifest_dir().join(activity))
                .context("Failed to locate custom MainActivity.kt")?,
            _ => hbs.render_template(
                include_str!("../../assets/android/MainActivity.kt.hbs"),
                &hbs_data,
            )?,
        };
        write(
            self.wry_android_kotlin_files_out_dir()
                .join("MainActivity.kt"),
            main_activity,
        )?;

        // Write the res folder, containing stuff like default icons, colors, and menubars.
        let res = app_main.join("res");
        create_dir_all(&res)?;
        create_dir_all(res.join("values"))?;
        write(
            res.join("values").join("strings.xml"),
            hbs.render_template(
                include_str!("../../assets/android/gen/app/src/main/res/values/strings.xml.hbs"),
                &hbs_data,
            )?,
        )?;
        write(
            res.join("values").join("colors.xml"),
            include_bytes!("../../assets/android/gen/app/src/main/res/values/colors.xml"),
        )?;
        write(
            res.join("values").join("styles.xml"),
            include_bytes!("../../assets/android/gen/app/src/main/res/values/styles.xml"),
        )?;

        create_dir_all(res.join("xml"))?;
        write(
            res.join("xml").join("network_security_config.xml"),
            include_bytes!(
                "../../assets/android/gen/app/src/main/res/xml/network_security_config.xml"
            ),
        )?;

        create_dir_all(res.join("drawable"))?;
        write(
            res.join("drawable").join("ic_launcher_background.xml"),
            include_bytes!(
                "../../assets/android/gen/app/src/main/res/drawable/ic_launcher_background.xml"
            ),
        )?;
        create_dir_all(res.join("drawable-v24"))?;
        write(
            res.join("drawable-v24").join("ic_launcher_foreground.xml"),
            include_bytes!(
                "../../assets/android/gen/app/src/main/res/drawable-v24/ic_launcher_foreground.xml"
            ),
        )?;
        create_dir_all(res.join("mipmap-anydpi-v26"))?;
        write(
            res.join("mipmap-anydpi-v26").join("ic_launcher.xml"),
            include_bytes!(
                "../../assets/android/gen/app/src/main/res/mipmap-anydpi-v26/ic_launcher.xml"
            ),
        )?;
        create_dir_all(res.join("mipmap-hdpi"))?;
        write(
            res.join("mipmap-hdpi").join("ic_launcher.webp"),
            include_bytes!(
                "../../assets/android/gen/app/src/main/res/mipmap-hdpi/ic_launcher.webp"
            ),
        )?;
        create_dir_all(res.join("mipmap-mdpi"))?;
        write(
            res.join("mipmap-mdpi").join("ic_launcher.webp"),
            include_bytes!(
                "../../assets/android/gen/app/src/main/res/mipmap-mdpi/ic_launcher.webp"
            ),
        )?;
        create_dir_all(res.join("mipmap-xhdpi"))?;
        write(
            res.join("mipmap-xhdpi").join("ic_launcher.webp"),
            include_bytes!(
                "../../assets/android/gen/app/src/main/res/mipmap-xhdpi/ic_launcher.webp"
            ),
        )?;
        create_dir_all(res.join("mipmap-xxhdpi"))?;
        write(
            res.join("mipmap-xxhdpi").join("ic_launcher.webp"),
            include_bytes!(
                "../../assets/android/gen/app/src/main/res/mipmap-xxhdpi/ic_launcher.webp"
            ),
        )?;
        create_dir_all(res.join("mipmap-xxxhdpi"))?;
        write(
            res.join("mipmap-xxxhdpi").join("ic_launcher.webp"),
            include_bytes!(
                "../../assets/android/gen/app/src/main/res/mipmap-xxxhdpi/ic_launcher.webp"
            ),
        )?;

        Ok(())
    }

    pub async fn assemble_android(&self, ctx: &BuildContext) -> Result<()> {
        ctx.status_running_gradle();

        // When the build mode is set to release and there is an Android signature configuration, use assembleRelease
        let build_type = if self.release && self.config.bundle.android.is_some() {
            "assembleRelease"
        } else {
            "assembleDebug"
        };

        let output = Command::new(self.gradle_exe()?)
            .arg(build_type)
            .current_dir(self.root_dir())
            .output()
            .await
            .context("Failed to run gradle")?;

        if !output.status.success() {
            bail!(
                "Failed to assemble apk: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        Ok(())
    }

    /// Run bundleRelease and return the path to the `.aab` file
    ///
    /// <https://stackoverflow.com/questions/57072558/whats-the-difference-between-gradlewassemblerelease-gradlewinstallrelease-and>
    pub(crate) async fn android_gradle_bundle(&self) -> Result<PathBuf> {
        let output = Command::new(self.gradle_exe()?)
            .arg("bundleRelease")
            .current_dir(self.root_dir())
            .output()
            .await
            .context("Failed to run gradle bundleRelease")?;

        if !output.status.success() {
            bail!(
                "Failed to bundleRelease: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        let app_release = self
            .root_dir()
            .join("app")
            .join("build")
            .join("outputs")
            .join("bundle")
            .join("release");

        // Rename it to Name-arch.aab
        let from = app_release.join("app-release.aab");
        let to = app_release.join(format!("{}-{}.aab", self.bundled_app_name(), self.triple));

        std::fs::rename(from, &to).context("Failed to rename aab")?;

        Ok(to)
    }

    /// Install Android plugin artifacts by bundling source folders as Gradle submodules.
    ///
    /// This function handles both prebuilt AARs and source folders:
    /// - If `artifact_path` is a file (ends in .aar), copy it to libs/ and add file dependency
    /// - If `artifact_path` is a directory, copy it as a Gradle submodule and add project dependency
    ///
    /// All sources are bundled first, then a single Gradle build compiles everything in `assemble()`.
    pub(crate) fn install_android_artifacts(
        &self,
        android_artifacts: &[AndroidArtifactMetadata],
    ) -> Result<()> {
        let libs_dir = self.root_dir().join("app").join("libs");
        std::fs::create_dir_all(&libs_dir)?;

        let plugins_dir = self.root_dir().join("plugins");
        let build_gradle = self.root_dir().join("app").join("build.gradle.kts");
        let settings_gradle = self.root_dir().join("settings.gradle");

        for artifact in android_artifacts {
            let artifact_path = PathBuf::from(artifact.artifact_path.as_str());
            let plugin_name = artifact.plugin_name.as_str();

            if artifact_path.is_dir() {
                // It's a source folder - copy it as a Gradle submodule
                tracing::debug!(
                    "Bundling Android plugin '{}' from source: {}",
                    plugin_name,
                    artifact_path.display()
                );

                // Create module directory
                let module_dir = plugins_dir.join(plugin_name);
                self.copy_build_dir_recursive(&artifact_path, &module_dir)?;

                // Strip version specifiers from build.gradle.kts to avoid conflicts with parent project
                self.strip_gradle_plugin_versions(&module_dir)?;

                // Add to settings.gradle
                self.ensure_settings_gradle_include(&settings_gradle, plugin_name)?;

                // Add project dependency to app/build.gradle.kts
                let dep_line = format!("implementation(project(\":plugins:{}\"))", plugin_name);
                self.ensure_gradle_dependency(&build_gradle, &dep_line)?;

                tracing::debug!(
                    "Added Android plugin module :plugins:{} from {}",
                    plugin_name,
                    artifact_path.display()
                );
            } else if artifact_path.extension().is_some_and(|ext| ext == "aar") {
                // It's a prebuilt AAR - copy directly to libs
                if !artifact_path.exists() {
                    anyhow::bail!(
                        "Android plugin artifact not found: {}",
                        artifact_path.display()
                    );
                }

                let filename = artifact_path
                    .file_name()
                    .ok_or_else(|| {
                        anyhow::anyhow!(
                            "Android plugin artifact path has no filename: {}",
                            artifact_path.display()
                        )
                    })?
                    .to_owned();
                let dest_file = libs_dir.join(&filename);
                std::fs::copy(&artifact_path, &dest_file)?;
                tracing::debug!(
                    "Copied Android artifact {} -> {}",
                    artifact_path.display(),
                    dest_file.display()
                );

                let dep_line = format!(
                    "implementation(files(\"libs/{}\"))",
                    filename.to_string_lossy()
                );
                self.ensure_gradle_dependency(&build_gradle, &dep_line)?;
            } else {
                anyhow::bail!(
                    "Android artifact path is neither a directory nor an AAR file: {}",
                    artifact_path.display()
                );
            }

            // Add any extra Gradle dependencies specified by the plugin
            for dependency in artifact
                .gradle_dependencies
                .as_str()
                .lines()
                .map(str::trim)
                .filter(|line| !line.is_empty())
            {
                self.ensure_gradle_dependency(&build_gradle, dependency)?;
            }
        }

        Ok(())
    }

    /// Check if the android tooling is installed
    ///
    /// looks for the android sdk + ndk
    ///
    /// will do its best to fill in the missing bits by exploring the sdk structure
    /// IE will attempt to use the Java installed from android studio if possible.
    pub async fn verify_android_tooling(&self) -> Result<()> {
        let linker = self
            .workspace
            .android_tools()?
            .android_cc(&self.triple, self.min_sdk_version_or_default());

        tracing::debug!("Verifying android linker: {linker:?}");

        if linker.exists() {
            return Ok(());
        }

        bail!(
            "Android linker not found at {linker:?}. Please set the `ANDROID_NDK_HOME` environment variable to the root of your NDK installation."
        );
    }

    pub(crate) fn start_android_sim(&self) -> Result<()> {
        let tools = self.workspace.android_tools()?;
        tokio::spawn(async move {
            let emulator = tools.emulator();
            let avds = Command::new(&emulator)
                .arg("-list-avds")
                .output()
                .await
                .unwrap();
            let avds = String::from_utf8_lossy(&avds.stdout);
            let avd = avds.trim().lines().next().map(|s| s.trim().to_string());
            if let Some(avd) = avd {
                tracing::info!("Booting Android emulator: \"{avd}\"");
                Command::new(&emulator)
                    .arg("-avd")
                    .arg(avd)
                    .args(["-netdelay", "none", "-netspeed", "full"])
                    .stdout(std::process::Stdio::null()) // prevent accumulating huge amounts of mem usage
                    .stderr(std::process::Stdio::null()) // prevent accumulating huge amounts of mem usage
                    .output()
                    .await
                    .unwrap();
            } else {
                tracing::warn!(
                    "No Android emulators found. Please create one using `emulator -avd <name>`"
                );
            }
        });
        Ok(())
    }

    pub(crate) fn gradle_exe(&self) -> Result<PathBuf> {
        // make sure we can execute the gradlew script
        #[cfg(unix)]
        {
            use std::os::unix::prelude::PermissionsExt;
            std::fs::set_permissions(
                self.root_dir().join("gradlew"),
                std::fs::Permissions::from_mode(0o755),
            )
            .context("Failed to make gradlew executable")?;
        }

        let gradle_exec_name = match cfg!(windows) {
            true => "gradlew.bat",
            false => "gradlew",
        };

        Ok(self.root_dir().join(gradle_exec_name))
    }

    pub(crate) fn debug_apk_path(&self) -> PathBuf {
        self.root_dir()
            .join("app")
            .join("build")
            .join("outputs")
            .join("apk")
            .join("debug")
            .join("app-debug.apk")
    }

    pub(crate) fn release_apk_path(&self) -> PathBuf {
        self.root_dir()
            .join("app")
            .join("build")
            .join("outputs")
            .join("apk")
            .join("release")
            .join("app-release.apk")
    }

    pub(crate) fn android_apk_path(&self) -> PathBuf {
        let assembled_release = self.release && self.config.bundle.android.is_some();
        if assembled_release {
            self.release_apk_path()
        } else {
            self.debug_apk_path()
        }
    }

    /// Set the environment variables required for building on Android.
    ///
    /// This involves setting sysroots, CC, CXX, AR, and other environment variables along with
    /// vars that cc-rs uses for its C/C++ compilation.
    ///
    /// We pulled the environment setup from `cargo ndk` and attempt to mimic its behavior to retain
    /// compatibility with existing crates that work with `cargo ndk`.
    ///
    /// <https://github.com/bbqsrc/cargo-ndk/blob/1d1a6dc70a99b7f95bc71ed07bf893ef37966efc/src/cargo.rs#L97-L102>
    ///
    /// cargo-ndk is MIT licensed.
    ///
    /// <https://github.com/bbqsrc/cargo-ndk>
    pub(crate) fn android_env_vars(&self) -> Result<Vec<(Cow<'static, str>, OsString)>> {
        // Derived from getenv_with_target_prefixes in `cc` crate.
        fn cc_env(var_base: &str, triple: &str) -> (String, Option<String>) {
            #[inline]
            fn env_var_with_key(key: String) -> Option<(String, String)> {
                std::env::var(&key).map(|value| (key, value)).ok()
            }

            let triple_u = triple.replace('-', "_");
            let most_specific_key = format!("{}_{}", var_base, triple);

            env_var_with_key(most_specific_key.to_string())
                .or_else(|| env_var_with_key(format!("{}_{}", var_base, triple_u)))
                .or_else(|| env_var_with_key(format!("TARGET_{}", var_base)))
                .or_else(|| env_var_with_key(var_base.to_string()))
                .map(|(key, value)| (key, Some(value)))
                .unwrap_or_else(|| (most_specific_key, None))
        }

        fn cargo_env_target_cfg(triple: &str, key: &str) -> String {
            format!("CARGO_TARGET_{}_{}", &triple.replace('-', "_"), key).to_uppercase()
        }

        fn clang_target(rust_target: &str, api_level: u8) -> String {
            let target = match rust_target {
                "arm-linux-androideabi" => "armv7a-linux-androideabi",
                "armv7-linux-androideabi" => "armv7a-linux-androideabi",
                _ => rust_target,
            };
            format!("--target={target}{api_level}")
        }

        fn sysroot_target(rust_target: &str) -> &str {
            (match rust_target {
                "armv7-linux-androideabi" => "arm-linux-androideabi",
                _ => rust_target,
            }) as _
        }
        fn rt_builtins(rust_target: &str) -> &str {
            (match rust_target {
                "armv7-linux-androideabi" => "arm",
                "aarch64-linux-android" => "aarch64",
                "i686-linux-android" => "i686",
                "x86_64-linux-android" => "x86_64",
                _ => rust_target,
            }) as _
        }

        let mut env_vars: Vec<(Cow<'static, str>, OsString)> = vec![];

        let min_sdk_version = self.min_sdk_version_or_default();

        let tools = self.workspace.android_tools()?;
        let linker = tools.android_cc(&self.triple, min_sdk_version);
        let ar_path = tools.ar_path();
        let target_cc = tools.target_cc();
        let target_cxx = tools.target_cxx();
        let java_home = tools.java_home();
        let ndk_home = tools.ndk.clone();
        let sdk_root = tools.sdk();
        tracing::debug!(
            r#"Using android:
            min_sdk_version: {min_sdk_version}
            linker: {linker:?}
            ar_path: {ar_path:?}
            target_cc: {target_cc:?}
            target_cxx: {target_cxx:?}
            java_home: {java_home:?}
            sdk_root: {sdk_root:?}
            "#
        );

        if let Some(java_home) = &java_home {
            tracing::debug!("Setting JAVA_HOME to {java_home:?}");
            env_vars.push(("JAVA_HOME".into(), java_home.clone().into_os_string()));
            env_vars.push((
                "DX_ANDROID_JAVA_HOME".into(),
                java_home.clone().into_os_string(),
            ));
        }

        env_vars.push((
            "DX_ANDROID_NDK_HOME".into(),
            ndk_home.clone().into_os_string(),
        ));
        env_vars.push((
            "DX_ANDROID_SDK_ROOT".into(),
            sdk_root.clone().into_os_string(),
        ));
        env_vars.push(("ANDROID_NDK_HOME".into(), ndk_home.clone().into_os_string()));
        env_vars.push(("ANDROID_SDK_ROOT".into(), sdk_root.clone().into_os_string()));
        env_vars.push(("ANDROID_HOME".into(), sdk_root.into_os_string()));
        env_vars.push(("NDK_HOME".into(), ndk_home.clone().into_os_string()));

        let triple = self.triple.to_string();

        // Environment variables for the `cc` crate
        let (cc_key, _cc_value) = cc_env("CC", &triple);
        let (cflags_key, cflags_value) = cc_env("CFLAGS", &triple);
        let (cxx_key, _cxx_value) = cc_env("CXX", &triple);
        let (cxxflags_key, cxxflags_value) = cc_env("CXXFLAGS", &triple);
        let (ar_key, _ar_value) = cc_env("AR", &triple);
        let (ranlib_key, _ranlib_value) = cc_env("RANLIB", &triple);

        // Environment variables for cargo
        let cargo_ar_key = cargo_env_target_cfg(&triple, "ar");
        let cargo_rust_flags_key = cargo_env_target_cfg(&triple, "rustflags");
        let bindgen_clang_args_key =
            format!("BINDGEN_EXTRA_CLANG_ARGS_{}", &triple.replace('-', "_"));

        let clang_target = clang_target(&self.triple.to_string(), min_sdk_version as _);
        let target_cc = tools.target_cc();
        let target_cflags = match cflags_value {
            Some(v) => format!("{clang_target} {v}"),
            None => clang_target.to_string(),
        };
        let target_cxx = tools.target_cxx();
        let target_cxxflags = match cxxflags_value {
            Some(v) => format!("{clang_target} {v}"),
            None => clang_target.to_string(),
        };
        let cargo_ndk_sysroot_path_key = "CARGO_NDK_SYSROOT_PATH";
        let cargo_ndk_sysroot_path = tools.sysroot();
        let cargo_ndk_sysroot_target_key = "CARGO_NDK_SYSROOT_TARGET";
        let cargo_ndk_sysroot_target = sysroot_target(&triple);
        let cargo_ndk_sysroot_libs_path_key = "CARGO_NDK_SYSROOT_LIBS_PATH";
        let cargo_ndk_sysroot_libs_path = cargo_ndk_sysroot_path
            .join("usr")
            .join("lib")
            .join(cargo_ndk_sysroot_target);
        let target_ar = tools.ar_path();
        let target_ranlib = tools.ranlib();
        let clang_folder = tools.clang_folder();

        // choose the clang target with the highest version
        // Should we filter for only numbers?
        let clang_rt = std::fs::read_dir(&clang_folder)
            .map(|dir| {
                let clang_builtins_target = dir
                    .filter_map(|a| a.ok())
                    .max_by(|a, b| a.file_name().cmp(&b.file_name()))
                    .map(|s| s.path())
                    .unwrap_or_else(|| clang_folder.join("clang"));

                format!(
                    "-L{} -lstatic=clang_rt.builtins-{}-android",
                    clang_builtins_target.join("lib").join("linux").display(),
                    rt_builtins(&triple)
                )
            })
            .unwrap_or_default();

        let extra_include: String = format!(
            "{}/usr/include/{}",
            &cargo_ndk_sysroot_path.display(),
            &cargo_ndk_sysroot_target
        );

        let bindgen_args = format!(
            "--sysroot={} -I{}",
            &cargo_ndk_sysroot_path.display(),
            extra_include
        );

        // Load up the OpenSSL environment variables, using our defaults if not set.
        // if the user specifies `/vendor`, then they get vendored, unless OPENSSL_NO_VENDOR is passed (implicitly...)
        let openssl_lib_dir = std::env::var("OPENSSL_LIB_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| AndroidTools::openssl_lib_dir(&self.triple));
        let openssl_include_dir = std::env::var("OPENSSL_INCLUDE_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| AndroidTools::openssl_include_dir());
        let openssl_libs =
            std::env::var("OPENSSL_LIBS").unwrap_or_else(|_| "ssl:crypto".to_string());

        for env in [
            (cc_key, target_cc.clone().into_os_string()),
            (cflags_key, target_cflags.into()),
            (cxx_key, target_cxx.into_os_string()),
            (cxxflags_key, target_cxxflags.into()),
            (ar_key, target_ar.clone().into()),
            (ranlib_key, target_ranlib.into_os_string()),
            (cargo_ar_key, target_ar.into_os_string()),
            (
                cargo_ndk_sysroot_path_key.to_string(),
                cargo_ndk_sysroot_path.clone().into_os_string(),
            ),
            (
                cargo_ndk_sysroot_libs_path_key.to_string(),
                cargo_ndk_sysroot_libs_path.into_os_string(),
            ),
            (
                cargo_ndk_sysroot_target_key.to_string(),
                cargo_ndk_sysroot_target.into(),
            ),
            (cargo_rust_flags_key, clang_rt.into()),
            (bindgen_clang_args_key, bindgen_args.into()),
            (
                "ANDROID_NATIVE_API_LEVEL".to_string(),
                min_sdk_version.to_string().into(),
            ),
            (
                format!(
                    "CARGO_TARGET_{}_LINKER",
                    self.triple
                        .to_string()
                        .to_ascii_uppercase()
                        .replace("-", "_")
                ),
                linker.into_os_string(),
            ),
            (
                "ANDROID_NDK_ROOT".to_string(),
                ndk_home.clone().into_os_string(),
            ),
            (
                "OPENSSL_LIB_DIR".to_string(),
                openssl_lib_dir.into_os_string(),
            ),
            (
                "OPENSSL_INCLUDE_DIR".to_string(),
                openssl_include_dir.into_os_string(),
            ),
            ("OPENSSL_LIBS".to_string(), openssl_libs.into()),
            // Set the wry env vars - this is where wry will dump its kotlin files.
            // Their setup is really annoying and requires us to hardcode `dx` to specific versions of tao/wry.
            (
                "WRY_ANDROID_PACKAGE".to_string(),
                "dev.dioxus.main".to_string().into(),
            ),
            (
                "WRY_ANDROID_LIBRARY".to_string(),
                self.android_lib_name().into(),
            ),
            ("WRY_ANDROID_KOTLIN_FILES_OUT_DIR".to_string(), {
                let kotlin_dir = self.wry_android_kotlin_files_out_dir();
                // Ensure the directory exists for WRY's canonicalize check
                if let Err(e) = std::fs::create_dir_all(&kotlin_dir) {
                    tracing::error!("Failed to create kotlin directory {:?}: {}", kotlin_dir, e);
                    return Err(anyhow::anyhow!("Failed to create kotlin directory: {}", e));
                }
                tracing::debug!("Created kotlin directory: {:?}", kotlin_dir);
                kotlin_dir.into_os_string()
            }),
            // Found this through a comment related to bindgen using the wrong clang for cross compiles
            //
            // https://github.com/rust-lang/rust-bindgen/issues/2962#issuecomment-2438297124
            //
            // https://github.com/KyleMayes/clang-sys?tab=readme-ov-file#environment-variables
            ("CLANG_PATH".into(), target_cc.with_extension("exe").into()),
        ] {
            env_vars.push((env.0.into(), env.1));
        }

        if std::env::var("MSYSTEM").is_ok() || std::env::var("CYGWIN").is_ok() {
            for var in env_vars.iter_mut() {
                // Convert windows paths to unix-style paths
                // This is a workaround for the fact that the `cc` crate expects unix-style paths
                // and will fail if it encounters windows-style paths.
                var.1 = var.1.to_string_lossy().replace('\\', "/").into();
            }
        }

        Ok(env_vars)
    }

    /// Strip version specifiers from build.gradle.kts plugins block.
    ///
    /// When a plugin module is included as a subproject, having version specifiers in the
    /// plugins block causes conflicts because the parent project already has the plugins
    /// on the classpath. This function removes version specifications like:
    /// - `version "8.4.2"` or `version "1.9.24"`
    /// - Entire version calls from plugin declarations
    pub(crate) fn strip_gradle_plugin_versions(&self, module_dir: &Path) -> Result<()> {
        use std::fs;

        let build_gradle = module_dir.join("build.gradle.kts");
        if !build_gradle.exists() {
            return Ok(());
        }

        let contents = fs::read_to_string(&build_gradle)?;

        // Remove version specifications from plugin declarations
        // Matches: id("com.android.library") version "8.4.2" -> id("com.android.library")
        // Matches: kotlin("android") version "1.9.24" -> kotlin("android")
        let version_pattern = regex::Regex::new(r#"\s+version\s+"[^"]+""#).expect("Invalid regex");
        let cleaned = version_pattern.replace_all(&contents, "");

        if cleaned != contents {
            fs::write(&build_gradle, cleaned.as_ref())?;
            tracing::debug!(
                "Stripped version specifiers from {}",
                build_gradle.display()
            );
        }

        Ok(())
    }

    /// Add a module include to settings.gradle if not already present.
    pub(crate) fn ensure_settings_gradle_include(
        &self,
        settings_gradle: &Path,
        plugin_name: &str,
    ) -> Result<()> {
        use std::fs;

        let include_line = format!("include ':plugins:{}'", plugin_name);
        let mut contents = fs::read_to_string(settings_gradle)?;

        if contents.contains(&include_line) {
            return Ok(());
        }

        // Add the include at the end
        contents.push_str(&format!("\n{}\n", include_line));
        fs::write(settings_gradle, contents)?;

        Ok(())
    }

    fn wry_android_kotlin_files_out_dir(&self) -> PathBuf {
        let mut kotlin_dir = self
            .root_dir()
            .join("app")
            .join("src")
            .join("main")
            .join("kotlin");

        for segment in "dev.dioxus.main".split('.') {
            kotlin_dir = kotlin_dir.join(segment);
        }

        kotlin_dir
    }

    fn ensure_gradle_dependency(&self, build_gradle: &Path, dependency_line: &str) -> Result<()> {
        use std::fs;

        let mut contents = fs::read_to_string(build_gradle)?;
        if contents.contains(dependency_line) {
            return Ok(());
        }

        if let Some(idx) = contents.find("dependencies {") {
            let insert_pos = idx + "dependencies {".len();
            contents.insert_str(insert_pos, &format!("\n    {dependency_line}"));
        } else {
            contents.push_str(&format!("\ndependencies {{\n    {dependency_line}\n}}\n"));
        }

        fs::write(build_gradle, contents)?;
        Ok(())
    }

    /// Returns the min sdk version set in config. If not set 24 is returned as a default.
    fn min_sdk_version_or_default(&self) -> u32 {
        self.config
            .application
            .android_min_sdk_version
            .unwrap_or(28)
    }

    /// Android native library name (without `lib` prefix and `.so` extension).
    /// Defaults to `"main"` per NativeActivity convention, overridable via `[android] lib_name`.
    pub(crate) fn android_lib_name(&self) -> String {
        self.config
            .android
            .lib_name
            .clone()
            .unwrap_or_else(|| "main".to_string())
    }
}

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

impl AndroidTools {
    pub(crate) fn current() -> Option<Arc<AndroidTools>> {
        // We check for SDK first since users might install Android Studio and then install the SDK
        // After that they might install the NDK, so the SDK drives the source of truth.
        let sdk = Self::var_or_debug("ANDROID_SDK_ROOT")
            .or_else(|| Self::var_or_debug("ANDROID_SDK"))
            .or_else(|| Self::var_or_debug("ANDROID_HOME"));

        // Check the ndk. We look for users's overrides first and then look into the SDK.
        // Sometimes users set only the NDK (especially if they're somewhat advanced) so we need to look for it manually
        //
        // Might look like this, typically under "sdk":
        // "/Users/jonkelley/Library/Android/sdk/ndk/25.2.9519653/toolchains/llvm/prebuilt/darwin-x86_64/bin/aarch64-linux-android24-clang"
        let ndk = Self::var_or_debug("NDK_HOME")
            .or_else(|| Self::var_or_debug("ANDROID_NDK_HOME"))
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
                    let jbr_home = PathBuf::from(
                        "/Applications/Android Studio.app/Contents/jbr/Contents/Home/",
                    );
                    if jbr_home.exists() {
                        return Some(jbr_home);
                    }

                    let jre_home = PathBuf::from(
                        "/Applications/Android Studio.app/Contents/jre/Contents/Home",
                    );
                    if jre_home.exists() {
                        return Some(jre_home);
                    }

                    let jdk_home = PathBuf::from(
                        "/Library/Java/JavaVirtualMachines/openjdk.jdk/Contents/Home/",
                    );
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

    /// The sysroot is usually located in the NDK under:
    /// "~/Library/Android/sdk/ndk/25.2.9519653/toolchains/llvm/prebuilt/darwin-x86_64/sysroot"
    /// or similar, depending on the platform.
    pub(crate) fn sysroot(&self) -> PathBuf {
        self.android_tools_dir().parent().unwrap().join("sysroot")
    }

    /// /Users/jonathankelley/Library/Android/sdk/ndk/25.2/... (25.2 is the ndk here)
    /// /Users/jonathankelley/Library/Android/sdk/
    pub(crate) fn sdk(&self) -> PathBuf {
        self.sdk
            .clone()
            .unwrap_or_else(|| self.ndk.parent().unwrap().parent().unwrap().to_path_buf())
    }

    pub(crate) fn emulator(&self) -> PathBuf {
        self.sdk().join("emulator").join("emulator")
    }

    /// The clang folder is usually located in the NDK under:
    /// `~/Library/Android/sdk/ndk/25.2.9519653/toolchains/llvm/prebuilt/darwin-x86_64/lib/clang/<version>`
    /// or similar, depending on the platform.
    fn clang_folder(&self) -> PathBuf {
        self.android_tools_dir()
            .parent()
            .unwrap()
            .join("lib")
            .join("clang")
    }

    fn ranlib(&self) -> PathBuf {
        self.android_tools_dir().join("llvm-ranlib")
    }

    fn ar_path(&self) -> PathBuf {
        self.android_tools_dir().join("llvm-ar")
    }

    fn target_cc(&self) -> PathBuf {
        self.android_tools_dir().join("clang")
    }

    fn target_cxx(&self) -> PathBuf {
        self.android_tools_dir().join("clang++")
    }

    fn java_home(&self) -> Option<PathBuf> {
        self.java_home.clone()
    }

    pub(crate) fn android_jnilib(triple: &Triple) -> &'static str {
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

    fn var_or_debug(name: &str) -> Option<PathBuf> {
        use std::env::var;

        var(name)
            .inspect_err(|_| tracing::trace!("{name} not set"))
            .ok()
            .map(PathBuf::from)
    }
}
