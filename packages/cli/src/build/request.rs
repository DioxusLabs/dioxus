use super::{progress::ProgressTx, BuildArtifacts};
use crate::dioxus_crate::DioxusCrate;
use crate::Result;
use crate::{assets::AssetManifest, TraceSrc};
use crate::{link::LinkAction, BuildArgs};
use crate::{AppBundle, Platform};
use anyhow::Context;
use serde::Deserialize;
use std::{path::PathBuf, process::Stdio, time::Instant};
use tokio::{io::AsyncBufReadExt, process::Command};

#[derive(Clone, Debug)]
pub(crate) struct BuildRequest {
    /// The configuration for the crate we are building
    pub(crate) krate: DioxusCrate,

    /// The arguments for the build
    pub(crate) build: BuildArgs,

    /// Status channel to send our progress updates to
    pub(crate) progress: ProgressTx,

    /// The target directory for the build
    pub(crate) custom_target_dir: Option<PathBuf>,
}

impl BuildRequest {
    pub fn new(krate: DioxusCrate, build: BuildArgs, progress: ProgressTx) -> Self {
        Self {
            build,
            krate,
            progress,
            custom_target_dir: None,
        }
    }

    /// Run the build command with a pretty loader, returning the executable output location
    ///
    /// This will also run the fullstack build. Note that fullstack is handled separately within this
    /// code flow rather than outside of it.
    pub(crate) async fn build_all(self) -> Result<AppBundle> {
        tracing::debug!("Running build command...");

        let (app, server) = self.build_concurrent().await?;

        AppBundle::new(self, app, server).await
    }

    /// Run the build command with a pretty loader, returning the executable output location
    async fn build_concurrent(&self) -> Result<(BuildArtifacts, Option<BuildArtifacts>)> {
        let (app, server) =
            futures_util::future::try_join(self.build_app(), self.build_server()).await?;

        Ok((app, server))
    }

    pub(crate) async fn build_app(&self) -> Result<BuildArtifacts> {
        tracing::debug!("Building app...");

        let start = Instant::now();
        self.prepare_build_dir()?;
        let exe = self.build_cargo().await?;
        let assets = self.collect_assets().await?;

        Ok(BuildArtifacts {
            exe,
            assets,
            time_taken: start.elapsed(),
        })
    }

    pub(crate) async fn build_server(&self) -> Result<Option<BuildArtifacts>> {
        tracing::debug!("Building server...");

        if !self.build.fullstack {
            return Ok(None);
        }

        let mut cloned = self.clone();
        cloned.build.platform = Some(Platform::Server);
        Ok(Some(cloned.build_app().await?))
    }

    /// Run `cargo`, returning the location of the final executable
    ///
    /// todo: add some stats here, like timing reports, crate-graph optimizations, etc
    pub(crate) async fn build_cargo(&self) -> Result<PathBuf> {
        tracing::debug!("Executing cargo...");

        // Extract the unit count of the crate graph so build_cargo has more accurate data
        let crate_count = self.get_unit_count_estimate().await;

        // Update the status to show that we're starting the build and how many crates we expect to build
        self.status_starting_build(crate_count);

        let mut cmd = Command::new("cargo");

        cmd.arg("rustc")
            .current_dir(self.krate.crate_dir())
            .arg("--message-format")
            .arg("json-diagnostic-rendered-ansi")
            .args(self.build_arguments())
            .envs(self.env_vars());
        // .args(["--", "-Csave-temps=y"]);

        if let Some(target_dir) = self.custom_target_dir.as_ref() {
            cmd.env("CARGO_TARGET_DIR", target_dir);
        }

        // Android needs a special linker since the linker is actually tied to the android toolchain.
        // For the sake of simplicity, we're going to pass the linker here using ourselves as the linker,
        // but in reality we could simply use the android toolchain's linker as the path.
        //
        // We don't want to overwrite the user's .cargo/config.toml since that gets committed to git
        // and we want everyone's install to be the same.
        if self.build.platform() == Platform::Android {
            let linker = self
                .krate
                .android_linker()
                .context("Could not autodetect android linker")?;

            tracing::trace!("Using android linker: {linker:?}");

            cmd.env(
                LinkAction::ENV_VAR_NAME,
                LinkAction::LinkAndroid {
                    linker,
                    extra_flags: vec![],
                }
                .to_json(),
            );
        }

        tracing::trace!(dx_src = ?TraceSrc::Build, "Rust cargo args: {:?}", cmd);

        let mut child = cmd
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context("Failed to spawn cargo build")?;

        let stdout = tokio::io::BufReader::new(child.stdout.take().unwrap());
        let stderr = tokio::io::BufReader::new(child.stderr.take().unwrap());
        let mut output_location = None;
        let mut stdout = stdout.lines();
        let mut stderr = stderr.lines();
        let mut units_compiled = 0;

        loop {
            use cargo_metadata::Message;

            let line = tokio::select! {
                Ok(Some(line)) = stdout.next_line() => line,
                Ok(Some(line)) = stderr.next_line() => line,
                else => break,
            };

            let mut deserializer = serde_json::Deserializer::from_str(line.trim());
            deserializer.disable_recursion_limit();

            let message =
                Message::deserialize(&mut deserializer).unwrap_or(Message::TextLine(line));

            match message {
                Message::BuildScriptExecuted(_) => units_compiled += 1,
                Message::TextLine(line) => self.status_build_message(line),
                Message::CompilerMessage(msg) => self.status_build_diagnostic(msg),
                Message::CompilerArtifact(artifact) => {
                    units_compiled += 1;
                    match artifact.executable {
                        Some(executable) => output_location = Some(executable.into()),
                        None => self.status_build_progress(
                            units_compiled,
                            crate_count,
                            artifact.target.name,
                        ),
                    }
                }
                Message::BuildFinished(finished) => {
                    if !finished.success {
                        return Err(anyhow::anyhow!(
                            "Cargo build failed, signaled by the compiler"
                        )
                        .into());
                    }
                }
                _ => {}
            }
        }

        if output_location.is_none() {
            tracing::error!("Cargo build failed - no output location");
        }

        let out_location = output_location.context("Build did not return an executable")?;

        tracing::debug!(
            "Build completed successfully - output location: {:?}",
            out_location
        );

        Ok(out_location)
    }

    /// Run the linker intercept and then fill in our AssetManifest from the incremental artifacts
    ///
    /// This will execute `dx` with an env var set to force `dx` to operate as a linker, and then
    /// traverse the .o and .rlib files rustc passes that new `dx` instance, collecting the link
    /// tables marked by manganis and parsing them as a ResourceAsset.
    pub(crate) async fn collect_assets(&self) -> Result<AssetManifest> {
        tracing::debug!("Collecting assets ...");

        if self.build.skip_assets {
            return Ok(AssetManifest::default());
        }

        self.deep_linker_asset_extract().await
    }

    /// Create a list of arguments for cargo builds
    pub(crate) fn build_arguments(&self) -> Vec<String> {
        let mut cargo_args = Vec::new();

        // Set the target, profile and features that vary between the app and server builds
        if self.build.platform() == Platform::Server {
            cargo_args.push("--profile".to_string());
            match self.build.release {
                true => cargo_args.push("release".to_string()),
                false => cargo_args.push(self.build.server_profile.to_string()),
            };
        } else {
            // Add required profile flags. --release overrides any custom profiles.
            let custom_profile = &self.build.profile.as_ref();
            if custom_profile.is_some() || self.build.release {
                cargo_args.push("--profile".to_string());
                match self.build.release {
                    true => cargo_args.push("release".to_string()),
                    false => {
                        cargo_args.push(
                            custom_profile
                                .expect("custom_profile should have been checked by is_some")
                                .to_string(),
                        );
                    }
                };
            }

            // todo: use the right arch based on the current arch
            let custom_target = match self.build.platform() {
                Platform::Web => Some("wasm32-unknown-unknown"),
                Platform::Ios => match self.build.target_args.device {
                    Some(true) => Some("aarch64-apple-ios"),
                    _ => Some("aarch64-apple-ios-sim"),
                },
                Platform::Android => Some("aarch64-linux-android"),
                Platform::Server => None,
                // we're assuming we're building for the native platform for now... if you're cross-compiling
                // the targets here might be different
                Platform::MacOS => None,
                Platform::Windows => None,
                Platform::Linux => None,
                Platform::Liveview => None,
            };

            if let Some(target) = custom_target.or(self.build.target_args.target.as_deref()) {
                cargo_args.push("--target".to_string());
                cargo_args.push(target.to_string());
            }
        }

        // We always run in verbose since the CLI itself is the one doing the presentation
        cargo_args.push("--verbose".to_string());

        if self.build.target_args.no_default_features {
            cargo_args.push("--no-default-features".to_string());
        }

        let features = self.target_features();

        if !features.is_empty() {
            cargo_args.push("--features".to_string());
            cargo_args.push(features.join(" "));
        }

        if let Some(ref package) = self.build.target_args.package {
            cargo_args.push(String::from("-p"));
            cargo_args.push(package.clone());
        }

        cargo_args.append(&mut self.build.cargo_args.clone());

        match self.krate.executable_type() {
            krates::cm::TargetKind::Bin => cargo_args.push("--bin".to_string()),
            krates::cm::TargetKind::Lib => cargo_args.push("--lib".to_string()),
            krates::cm::TargetKind::Example => cargo_args.push("--example".to_string()),
            _ => {}
        };

        cargo_args.push(self.krate.executable_name().to_string());

        tracing::debug!(dx_src = ?TraceSrc::Build, "cargo args: {:?}", cargo_args);

        cargo_args
    }

    #[allow(dead_code)]
    pub(crate) fn android_rust_flags(&self) -> String {
        let mut rust_flags = std::env::var("RUSTFLAGS").unwrap_or_default();

        if self.build.platform() == Platform::Android {
            let cur_exe = std::env::current_exe().unwrap();
            rust_flags.push_str(format!(" -Clinker={}", cur_exe.display()).as_str());
            rust_flags.push_str(" -Clink-arg=-landroid");
            rust_flags.push_str(" -Clink-arg=-llog");
            rust_flags.push_str(" -Clink-arg=-lOpenSLES");
            rust_flags.push_str(" -Clink-arg=-Wl,--export-dynamic");
        }

        rust_flags
    }

    /// Create the list of features we need to pass to cargo to build the app by merging together
    /// either the client or server features depending on if we're building a server or not.
    pub(crate) fn target_features(&self) -> Vec<String> {
        let mut features = self.build.target_args.features.clone();

        if self.build.platform() == Platform::Server {
            features.extend(self.build.target_args.server_features.clone());
        } else {
            features.extend(self.build.target_args.client_features.clone());
        }

        features
    }

    pub(crate) fn all_target_features(&self) -> Vec<String> {
        let mut features = self.target_features();

        if !self.build.target_args.no_default_features {
            features.extend(
                self.krate
                    .package()
                    .features
                    .get("default")
                    .cloned()
                    .unwrap_or_default(),
            );
        }

        features.dedup();

        features
    }

    /// Try to get the unit graph for the crate. This is a nightly only feature which may not be available with the current version of rustc the user has installed.
    pub(crate) async fn get_unit_count(&self) -> crate::Result<usize> {
        #[derive(Debug, Deserialize)]
        struct UnitGraph {
            units: Vec<serde_json::Value>,
        }

        let output = tokio::process::Command::new("cargo")
            .arg("+nightly")
            .arg("build")
            .arg("--unit-graph")
            .arg("-Z")
            .arg("unstable-options")
            .args(self.build_arguments())
            .envs(self.env_vars())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await?;

        if !output.status.success() {
            return Err(anyhow::anyhow!("Failed to get unit count").into());
        }

        let output_text = String::from_utf8(output.stdout).context("Failed to get unit count")?;
        let graph: UnitGraph =
            serde_json::from_str(&output_text).context("Failed to get unit count")?;

        Ok(graph.units.len())
    }

    /// Get an estimate of the number of units in the crate. If nightly rustc is not available, this will return an estimate of the number of units in the crate based on cargo metadata.
    /// TODO: always use https://doc.rust-lang.org/nightly/cargo/reference/unstable.html#unit-graph once it is stable
    pub(crate) async fn get_unit_count_estimate(&self) -> usize {
        // Try to get it from nightly
        self.get_unit_count().await.unwrap_or_else(|_| {
            // Otherwise, use cargo metadata
            (self
                .krate
                .krates
                .krates_filtered(krates::DepKind::Dev)
                .iter()
                .map(|k| k.targets.len())
                .sum::<usize>() as f64
                / 3.5) as usize
        })
    }

    /// We used to require traversing incremental artifacts for assets that were included but not
    /// directly exposed to the final binary. Now, however, we force APIs to carry items created
    /// from asset calls into top-level items such that they *do* get included in the final binary.
    ///
    /// There's a chance that's not actually true, so this function is kept around in case we do
    /// need to revert to "deep extraction".
    async fn deep_linker_asset_extract(&self) -> Result<AssetManifest> {
        // Create a temp file to put the output of the args
        // We need to do this since rustc won't actually print the link args to stdout, so we need to
        // give `dx` a file to dump its env::args into
        let tmp_file = tempfile::NamedTempFile::new()?;

        // Run `cargo rustc` again, but this time with a custom linker (dx) and an env var to force
        // `dx` to act as a linker
        //
        // This will force `dx` to look through the incremental cache and find the assets from the previous build
        Command::new("cargo")
            .arg("rustc")
            .args(self.build_arguments())
            .envs(self.env_vars())
            .arg("--offline") /* don't use the network, should already be resolved */
            .arg("--")
            .arg(format!(
                "-Clinker={}",
                std::env::current_exe()
                    .unwrap()
                    .canonicalize()
                    .unwrap()
                    .display()
            ))
            .env(
                LinkAction::ENV_VAR_NAME,
                LinkAction::BuildAssetManifest {
                    destination: tmp_file.path().to_path_buf().clone(),
                }
                .to_json(),
            )
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await?;

        // The linker wrote the manifest to the temp file, let's load it!
        let manifest = AssetManifest::load_from_file(tmp_file.path())?;

        if let Ok(path) = std::env::var("DEEPLINK").map(|s| s.parse::<PathBuf>().unwrap()) {
            _ = tmp_file.persist(path);
        }

        Ok(manifest)
    }

    fn env_vars(&self) -> Vec<(&str, String)> {
        let mut env_vars = vec![];

        if self.build.platform() == Platform::Android {
            let app = self.root_dir().join("app");
            let app_main = app.join("src").join("main");
            let app_kotlin = app_main.join("kotlin");
            let app_kotlin_out = app_kotlin.join("com").join("example").join("androidfinal");

            env_vars.push((
                "WRY_ANDROID_PACKAGE",
                "com.example.androidfinal".to_string(),
            ));
            env_vars.push(("WRY_ANDROID_LIBRARY", "androidfinal".to_string()));
            env_vars.push((
                "WRY_ANDROID_KOTLIN_FILES_OUT_DIR",
                app_kotlin_out.display().to_string(),
            ));

            env_vars.push(("RUSTFLAGS", self.android_rust_flags()))
        };

        env_vars
    }

    /// We only really currently care about:
    ///
    /// - app dir (.app, .exe, .apk, etc)
    /// - assets dir
    /// - exe dir (.exe, .app, .apk, etc)
    /// - extra scaffolding
    ///
    /// It's not guaranteed that they're different from any other folder
    fn prepare_build_dir(&self) -> Result<()> {
        use once_cell::sync::OnceCell;
        use std::fs::{create_dir_all, remove_dir_all};

        static INITIALIZED: OnceCell<Result<()>> = OnceCell::new();

        let success = INITIALIZED.get_or_init(|| {
            _ = remove_dir_all(self.root_dir());

            create_dir_all(self.root_dir())?;
            create_dir_all(self.exe_dir())?;
            create_dir_all(self.asset_dir())?;

            tracing::debug!("Initialized Root dir: {:?}", self.root_dir());
            tracing::debug!("Initialized Exe dir: {:?}", self.exe_dir());
            tracing::debug!("Initialized Asset dir: {:?}", self.asset_dir());

            // we could download the templates from somewhere (github?) but after having banged my head against
            // cargo-mobile2 for ages, I give up with that. We're literally just going to hardcode the templates
            // by writing them here.
            if let Platform::Android = self.build.platform() {
                self.build_android_app_dir()?;
            }

            Ok(())
        });

        if let Err(e) = success.as_ref() {
            return Err(format!("Failed to initialize build directory: {e}").into());
        }

        Ok(())
    }

    /// The directory in which we'll put the main exe
    ///
    /// Mac, Android, Web are a little weird
    /// - mac wants to be in Contents/MacOS
    /// - android wants to be in jniLibs/arm64-v8a (or others, depending on the platform / architecture)
    /// - web wants to be in wasm (which... we don't really need to, we could just drop the wasm into public and it would work)
    ///
    /// I think all others are just in the root folder
    ///
    /// todo(jon): investigate if we need to put .wasm in `wasm`. It kinda leaks implementation details, which ideally we don't want to do.
    pub fn exe_dir(&self) -> PathBuf {
        match self.build.platform() {
            Platform::MacOS => self.root_dir().join("Contents").join("MacOS"),
            Platform::Web => self.root_dir().join("wasm"),

            // Android has a whole build structure to it
            Platform::Android => self
                .root_dir()
                .join("app")
                .join("src")
                .join("main")
                .join("jniLibs")
                .join("arm64-v8a"),

            // these are all the same, I think?
            Platform::Windows
            | Platform::Linux
            | Platform::Ios
            | Platform::Server
            | Platform::Liveview => self.root_dir(),
        }
    }

    /// returns the path to root build folder. This will be our working directory for the build.
    ///
    /// we only add an extension to the folders where it sorta matters that it's named with the extension.
    /// for example, on mac, the `.app` indicates we can `open` it and it pulls in icons, dylibs, etc.
    ///
    /// for our simulator-based platforms, this is less important since they need to be zipped up anyways
    /// to run in the simulator.
    ///
    /// For windows/linux, it's also not important since we're just running the exe directly out of the folder
    ///
    /// The idea of this folder is that we can run our top-level build command against it and we'll get
    /// a final build output somewhere. Some platforms have basically no build command, and can simply
    /// be ran by executing the exe directly.
    pub(crate) fn root_dir(&self) -> PathBuf {
        let platform_dir = self
            .krate
            .build_dir(self.build.platform(), self.build.release);

        match self.build.platform() {
            Platform::Web => platform_dir.join("public"),
            Platform::Server => platform_dir.clone(), // ends up *next* to the public folder

            // These might not actually need to be called `.app` but it does let us run these with `open`
            Platform::MacOS => platform_dir.join(format!("{}.app", self.platform_exe_name())),
            Platform::Ios => platform_dir.join(format!("{}.app", self.platform_exe_name())),

            // in theory, these all could end up directly in the root dir
            Platform::Android => platform_dir.join("app"), // .apk (after bundling)
            Platform::Linux => platform_dir.join("app"),   // .appimage (after bundling)
            Platform::Windows => platform_dir.join("app"), // .exe (after bundling)
            Platform::Liveview => platform_dir.join("app"), // .exe (after bundling)
        }
    }

    pub fn asset_dir(&self) -> PathBuf {
        match self.build.platform() {
            Platform::MacOS => self
                .root_dir()
                .join("Contents")
                .join("Resources")
                .join("assets"),

            Platform::Android => self
                .root_dir()
                .join("app")
                .join("src")
                .join("main")
                .join("assets"),

            // everyone else is soooo normal, just app/assets :)
            Platform::Web
            | Platform::Ios
            | Platform::Windows
            | Platform::Linux
            | Platform::Server
            | Platform::Liveview => self.root_dir().join("assets"),
        }
    }

    pub fn platform_exe_name(&self) -> String {
        match self.build.platform() {
            Platform::MacOS => self.krate.executable_name().to_string(),
            Platform::Ios => self.krate.executable_name().to_string(),
            Platform::Server => self.krate.executable_name().to_string(),
            Platform::Liveview => self.krate.executable_name().to_string(),
            Platform::Windows => format!("{}.exe", self.krate.executable_name()),

            // from the apk spec, the root exe will actually be a shared library
            Platform::Android => format!("lib{}.so", self.krate.executable_name()),
            Platform::Web => unimplemented!("there's no main exe on web"), // this will be wrong, I think, but not important?

            // todo: maybe this should be called AppRun?
            Platform::Linux => self.krate.executable_name().to_string(),
        }
    }

    fn build_android_app_dir(&self) -> Result<()> {
        use std::fs::{create_dir_all, write};
        let root = self.root_dir();

        // gradle
        let wrapper = root.join("gradle").join("wrapper");
        create_dir_all(&wrapper)?;
        tracing::debug!("Initialized Gradle wrapper: {:?}", wrapper);

        // app
        let app = root.join("app");
        let app_main = app.join("src").join("main");
        let app_kotlin = app_main.join("kotlin");
        let app_jnilibs = app_main.join("jniLibs");
        let app_assets = app_main.join("assets");
        let app_kotlin_out = app_kotlin.join("com").join("example").join("androidfinal");
        create_dir_all(&app)?;
        create_dir_all(&app_main)?;
        create_dir_all(&app_kotlin)?;
        create_dir_all(&app_jnilibs)?;
        create_dir_all(&app_assets)?;
        create_dir_all(&app_kotlin_out)?;
        tracing::debug!("Initialized app: {:?}", app);
        tracing::debug!("Initialized app/src: {:?}", app_main);
        tracing::debug!("Initialized app/src/kotlin: {:?}", app_kotlin);
        tracing::debug!("Initialized app/src/jniLibs: {:?}", app_jnilibs);
        tracing::debug!("Initialized app/src/assets: {:?}", app_assets);
        tracing::debug!("Initialized app/src/kotlin/main: {:?}", app_kotlin_out);

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
            include_bytes!("../../assets/android/gen/app/build.gradle.kts"),
        )?;
        write(
            app.join("proguard-rules.pro"),
            include_bytes!("../../assets/android/gen/app/proguard-rules.pro"),
        )?;
        write(
            app.join("src").join("main").join("AndroidManifest.xml"),
            include_bytes!("../../assets/android/gen/app/src/main/AndroidManifest.xml"),
        )?;

        // Write the main activity manually since tao dropped support for it
        write(
            app_main
                .join("kotlin")
                .join("com")
                .join("example")
                .join("androidfinal")
                .join("MainActivity.kt"),
            include_bytes!("../../assets/android/MainActivity.kt"),
        )?;

        // Write the res folder
        let res = app_main.join("res");
        create_dir_all(&res)?;
        create_dir_all(res.join("values"))?;
        write(
            res.join("values").join("strings.xml"),
            include_bytes!("../../assets/android/gen/app/src/main/res/values/strings.xml"),
        )?;
        write(
            res.join("values").join("colors.xml"),
            include_bytes!("../../assets/android/gen/app/src/main/res/values/colors.xml"),
        )?;
        write(
            res.join("values").join("styles.xml"),
            include_bytes!("../../assets/android/gen/app/src/main/res/values/styles.xml"),
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
}
