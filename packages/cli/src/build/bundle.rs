use crate::{assets::AssetManifest, TraceSrc};
use crate::{build, Result};
use crate::{BuildRequest, Platform};
use anyhow::Context;
use once_cell::sync::{Lazy, OnceCell};
use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};
use std::{
    fs::create_dir_all,
    path::{Path, PathBuf},
};
use std::{sync::atomic::AtomicUsize, time::Duration};
use tokio::process::Command;
use wasm_bindgen_cli_support::Bindgen;

use super::templates::InfoPlistData;

/// The end result of a build.
///
/// Contains the final asset manifest, the executables, and the workdir.
///
/// Every dioxus app can have an optional server executable which will influence the final bundle.
/// This is built in parallel with the app executable during the `build` phase and the progres/status
/// of the build is aggregated.
///
/// The server will *always* be dropped into the `web` folder since it is considered "web" in nature,
/// and will likely need to be combined with the public dir to be useful.
///
/// We do our best to assemble read-to-go bundles here, such that the "bundle" step for each platform
/// can just use the build dir
///
/// When we write the AppBundle to a folder, it'll contain each bundle for each platform under the app's name:
/// ```
/// dog-app/
///   build/
///       web/
///         server.exe
///         assets/
///           some-secret-asset.txt (a server-side asset)
///         public/
///           index.html
///           assets/
///             logo.png
///       desktop/
///          App.app
///          App.appimage
///          App.exe
///          server/
///              server
///              assets/
///                some-secret-asset.txt (a server-side asset)
///       ios/
///          App.app
///          App.ipa
///       android/
///          App.apk
///   bundle/
///       build.json
///       Desktop.app
///       Mobile_x64.ipa
///       Mobile_arm64.ipa
///       Mobile_rosetta.ipa
///       web.appimage
///       web/
///         server.exe
///         assets/
///             some-secret-asset.txt
///         public/
///             index.html
///             assets/
///                 logo.png
///                 style.css
/// ```
///
/// When deploying, the build.json file will provide all the metadata that dx-deploy will use to
/// push the app to stores, set up infra, manage versions, etc.
///
/// The format of each build will follow the name plus some metadata such that when distributing you
/// can easily trim off the metadata.
///
/// The idea here is that we can run any of the programs in the same way that they're deployed.
///
///
/// ## Bundle structure links
/// - apple: https://developer.apple.com/documentation/bundleresources/placing_content_in_a_bundle
/// - appimage: https://docs.appimage.org/packaging-guide/manual.html#ref-manual
///
/// ## Extra links
/// - xbuild: https://github.com/rust-mobile/xbuild/blob/master/xbuild/src/command/build.rs
#[derive(Debug)]
pub(crate) struct AppBundle {
    pub(crate) build: BuildRequest,
    pub(crate) app: BuildArtifacts,
    pub(crate) server: Option<BuildArtifacts>,
}

#[derive(Debug)]
pub struct BuildArtifacts {
    pub(crate) exe: PathBuf,
    pub(crate) assets: AssetManifest,
    pub(crate) time_taken: Duration,
}

impl AppBundle {
    /// ## Web:
    /// Create a folder that is somewhat similar to an app-image (exe + asset)
    /// The server is dropped into the `web` folder, even if there's no `public` folder.
    /// If there's no server (SPA), we still use the `web` folder, but it only contains the
    /// public folder.
    /// ```
    /// web/
    ///     server
    ///     assets/
    ///     public/
    ///         index.html
    ///         wasm/
    ///            app.wasm
    ///            glue.js
    ///            snippets/
    ///                ...
    ///         assets/
    ///            logo.png
    /// ```
    ///
    /// ## Linux:
    /// https://docs.appimage.org/reference/appdir.html#ref-appdir
    /// current_exe.join("Assets")
    /// ```
    /// app.appimage/
    ///     AppRun
    ///     app.desktop
    ///     package.json
    ///     assets/
    ///         logo.png
    /// ```
    ///
    /// ## Macos
    /// We simply use the macos format where binaries are in `Contents/MacOS` and assets are in `Contents/Resources`
    /// We put assets in an assets dir such that it generally matches every other platform and we can
    /// output `/assets/blah` from manganis.
    /// ```
    /// App.app/
    ///     Contents/
    ///         Info.plist
    ///         MacOS/
    ///             Frameworks/
    ///         Resources/
    ///             assets/
    ///                 blah.icns
    ///                 blah.png
    ///         CodeResources
    ///         _CodeSignature/
    /// ```
    ///
    /// ## iOS
    /// Not the same as mac! ios apps are a bit "flattened" in comparison. simpler format, presumably
    /// since most ios apps don't ship frameworks/plugins and such.
    ///
    /// todo(jon): include the signing and entitlements in this format diagram.
    /// ```
    /// App.app/
    ///     main
    ///     assets/
    /// ```
    ///
    /// ## Android:
    ///
    /// Currently we need to generate a `src` type structure, not a pre-packaged apk structure, since
    /// we need to compile kotlin and java. This pushes us into using gradle and following a structure
    /// similar to that of cargo mobile2. Eventually I'd like to slim this down (drop buildSrc) and
    /// drive the kotlin build ourselves. This would let us drop gradle (yay! no plugins!) but requires
    /// us to manage dependencies (like kotlinc) ourselves (yuck!).
    ///
    /// https://github.com/WanghongLin/miscellaneous/blob/master/tools/build-apk-manually.sh
    ///
    /// Unfortunately, it seems that while we can drop the `android` build plugin, we still will need
    /// gradle since kotlin is basically gradle-only.
    ///
    /// Pre-build:
    /// ```
    /// app.apk/
    ///     .gradle
    ///     app/
    ///         src/
    ///             main/
    ///                 assets/
    ///                 jniLibs/
    ///                 java/
    ///                 kotlin/
    ///                 res/
    ///                 AndroidManifest.xml
    ///             build.gradle.kts
    ///             proguard-rules.pro
    ///         buildSrc/
    ///             build.gradle.kts
    ///             src/
    ///                 main/
    ///                     kotlin/
    ///                          BuildTask.kt
    ///     build.gradle.kts
    ///     gradle.properties
    ///     gradlew
    ///     gradlew.bat
    ///     settings.gradle
    /// ```
    ///
    /// Final build:
    /// ```
    /// app.apk/
    ///   AndroidManifest.xml
    ///   classes.dex
    ///   assets/
    ///       logo.png
    ///   lib/
    ///       armeabi-v7a/
    ///           libmyapp.so
    ///       arm64-v8a/
    ///           libmyapp.so
    /// ```
    /// Notice that we *could* feasibly build this ourselves :)
    ///
    /// ## Windows:
    /// https://superuser.com/questions/749447/creating-a-single-file-executable-from-a-directory-in-windows
    /// Windows does not provide an AppImage format, so instead we're going build the same folder
    /// structure as an AppImage, but when distributing, we'll create a .exe that embeds the resources
    /// as an embedded .zip file. When the app runs, it will implicitly unzip its resources into the
    /// Program Files folder. Any subsequent launches of the parent .exe will simply call the AppRun.exe
    /// entrypoint in the associated Program Files folder.
    ///
    /// This is, in essence, the same as an installer, so we might eventually just support something like msi/msix
    /// which functionally do the same thing but with a sleeker UI.
    ///
    /// This means no installers are required and we can bake an updater into the host exe.
    ///
    /// ## Handling asset lookups:
    /// current_exe.join("assets")
    /// ```
    /// app.appimage/
    ///     main.exe
    ///     main.desktop
    ///     package.json
    ///     assets/
    ///         logo.png
    /// ```
    ///
    /// Since we support just a few locations, we could just search for the first that exists
    /// - usr
    /// - ../Resources
    /// - assets
    /// - Assets
    /// - $cwd/assets
    ///
    /// ```
    /// assets::root() ->
    ///     mac -> ../Resources/
    ///     ios -> ../Resources/
    ///     android -> assets/
    ///     server -> assets/
    ///     liveview -> assets/
    ///     web -> /assets/
    /// root().join(bundled)
    /// ```
    pub(crate) async fn new(
        build: BuildRequest,
        app: BuildArtifacts,
        server: Option<BuildArtifacts>,
    ) -> Result<Self> {
        let bundle = Self { app, server, build };

        tracing::debug!("Assembling app bundle");

        bundle.build.status_start_bundle();
        bundle.prepare_build_dir()?;
        bundle.write_main_executable().await?;
        bundle.write_server_executable().await?;
        bundle.write_assets().await?;
        bundle.write_metadata().await?;
        bundle.optimize().await?;
        bundle.assemble().await?;

        tracing::debug!("Bundle created at {}", bundle.root_dir().display());

        Ok(bundle)
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
        static INTIALIZED: OnceCell<Result<()>> = OnceCell::new();

        let success = INTIALIZED.get_or_init(|| {
            _ = std::fs::remove_dir_all(self.root_dir());

            create_dir_all(self.root_dir())?;
            create_dir_all(self.exe_dir())?;
            create_dir_all(self.asset_dir())?;

            tracing::debug!("Initialized Root dir: {:?}", self.root_dir());
            tracing::debug!("Initialized Exe dir: {:?}", self.exe_dir());
            tracing::debug!("Initialized Asset dir: {:?}", self.asset_dir());

            // we could download the templates from somewhere (github?) but after having banged my head against
            // cargo-mobile2 for ages, I give up with that. We're literally just going to hardcode the templates
            // by writing them here.
            if let Platform::Android = self.build.build.platform() {
                self.build_android_app_dir()?;
            }

            Ok(())
        });

        if let Err(e) = success.as_ref() {
            return Err(format!("Failed to initialize build directory: {e}").into());
        }

        Ok(())
    }

    /// Take the output of rustc and make it into the main exe of the bundle
    ///
    /// For wasm, we'll want to run `wasm-bindgen` to make it a wasm binary along with some other optimizations
    /// Other platforms we might do some stripping or other optimizations
    /// Move the executable to the workdir
    async fn write_main_executable(&self) -> Result<()> {
        match self.build.build.platform() {
            // Run wasm-bindgen on the wasm binary and set its output to be in the bundle folder
            // Also run wasm-opt on the wasm binary, and sets the index.html since that's also the "executable".
            //
            // The wasm stuff will be in a folder called "wasm" in the workdir.
            //
            // Final output format:
            // ```
            // dx/
            //     app/
            //         web/
            //             bundle/
            //             build/
            //                 public/
            //                     index.html
            //                     wasm/
            //                        app.wasm
            //                        glue.js
            //                        snippets/
            //                            ...
            //                     assets/
            //                        logo.png
            // ```
            Platform::Web => {
                // Run wasm-bindgen and drop its output into the assets folder under "dioxus"
                self.build.status_wasm_bindgen_start();
                self.run_wasm_bindgen(&self.app.exe.with_extension("wasm"), &self.exe_dir())
                    .await?;

                // Only run wasm-opt if the feature is enabled
                // Wasm-opt has an expensive build script that makes it annoying to keep enabled for iterative dev
                // We put it behind the "wasm-opt" feature flag so that it can be disabled when iterating on the cli
                self.run_wasm_opt(&self.exe_dir())?;

                // Write the index.html file with the pre-configured contents we got from pre-rendering
                std::fs::write(
                    self.root_dir().join("index.html"),
                    self.build.prepare_html()?,
                )?;
            }

            // this will require some extra oomf to get the multi architecture builds...
            // for now, we just copy the exe into the current arch (which, sorry, is hardcoded for my m1)
            // we'll want to do multi-arch builds in the future, so there won't be *one* exe dir to worry about
            // eventually `exe_dir` and `main_exe` will need to take in an arch and return the right exe path
            //
            // todo(jon): maybe just symlink this rather than copy it?
            Platform::Android => {
                // https://github.com/rust-mobile/xbuild/blob/master/xbuild/template/lib.rs
                // https://github.com/rust-mobile/xbuild/blob/master/apk/src/lib.rs#L19
                std::fs::copy(&self.app.exe, self.main_exe())?;
            }

            // These are all super simple, just copy the exe into the folder
            // eventually, perhaps, maybe strip + encrypt the exe?
            Platform::MacOS
            | Platform::Windows
            | Platform::Linux
            | Platform::Ios
            | Platform::Liveview
            | Platform::Server => {
                std::fs::copy(&self.app.exe, self.main_exe())?;
            }
        }

        Ok(())
    }

    /// Copy the assets out of the manifest and into the target location
    ///
    /// Should be the same on all platforms - just copy over the assets from the manifest into the output directory
    async fn write_assets(&self) -> Result<()> {
        // Server doesn't need assets - web will provide them
        if self.build.build.platform() == Platform::Server {
            return Ok(());
        }

        let asset_dir = self.asset_dir();

        // First, clear the asset dir
        // todo(jon): cache the asset dir, removing old files and only copying new ones that changed since the last build
        _ = std::fs::remove_dir_all(&asset_dir);
        _ = create_dir_all(&asset_dir);

        // todo(jon): we also want to eventually include options for each asset's optimization and compression, which we currently aren't
        let mut assets_to_transfer = vec![];

        // Queue the bundled assets
        for asset in self.app.assets.assets.keys() {
            let bundled = self.app.assets.assets.get(asset).unwrap();
            let from = bundled.absolute.clone();
            let to = asset_dir.join(&bundled.bundled);
            tracing::debug!("Copying asset {from:?} to {to:?}");
            assets_to_transfer.push((from, to));
        }

        // And then queue the legacy assets
        // ideally, one day, we can just check the rsx!{} calls for references to assets
        for from in self.build.krate.legacy_asset_dir_files() {
            let to = asset_dir.join(from.file_name().unwrap());
            tracing::debug!("Copying legacy asset {from:?} to {to:?}");
            assets_to_transfer.push((from, to));
        }

        let asset_count = assets_to_transfer.len();
        let assets_finished = AtomicUsize::new(0);

        // Parallel Copy over the assets and keep track of progress with an atomic counter
        // todo: we want to use the fastfs variant that knows how to parallelize folders, too
        assets_to_transfer.par_iter().try_for_each(|(from, to)| {
            tracing::trace!(
                "Starting asset copy {current}/{asset_count} from {from:?}",
                current = assets_finished.fetch_add(0, std::sync::atomic::Ordering::SeqCst),
            );

            // todo(jon): implement optimize + pre_compress on the asset type
            let res = crate::fastfs::copy_asset(from, to);

            if let Err(err) = res.as_ref() {
                tracing::error!("Failed to copy asset {from:?}: {err}");
            }

            self.build.status_copied_asset(
                assets_finished.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1,
                asset_count,
                from.clone(),
            );

            res.map(|_| ())
        })?;

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
        match self.build.build.platform() {
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

    /// The item that we'll try to run directly if we need to.
    ///
    /// todo(jon): we should name the app properly instead of making up the exe name. It's kinda okay for dev mode, but def not okay for prod
    pub fn main_exe(&self) -> PathBuf {
        self.exe_dir().join(self.platform_exe_name())
    }

    fn platform_exe_name(&self) -> String {
        match self.build.build.platform() {
            Platform::MacOS => self.build.krate.executable_name().to_string(),
            Platform::Ios => self.build.krate.executable_name().to_string(),
            Platform::Server => self.build.krate.executable_name().to_string(),
            Platform::Liveview => self.build.krate.executable_name().to_string(),
            Platform::Windows => format!("{}.exe", self.build.krate.executable_name()),

            // from the apk spec, the root exe will actually be a shared library
            Platform::Android => format!("lib{}.so", self.build.krate.executable_name()),
            Platform::Web => unimplemented!("there's no main exe on web"), // this will be wrong, I think, but not important?

            // todo: maybe this should be called AppRun?
            Platform::Linux => self.build.krate.executable_name().to_string(),
        }
    }

    pub fn asset_dir(&self) -> PathBuf {
        match self.build.build.platform() {
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

    /// We always put the server in the `web` folder!
    /// Only the `web` target will generate a `public` folder though
    async fn write_server_executable(&self) -> Result<()> {
        if let Some(server) = &self.server {
            let to = self
                .server_exe()
                .expect("server should be set if we're building a server");

            std::fs::create_dir_all(self.server_exe().unwrap().parent().unwrap())?;

            tracing::debug!("Copying server executable from {server:?} to {to:?}");

            // Remove the old server executable if it exists, since copying might corrupt it :(
            // todo(jon): do this in more places, I think
            _ = std::fs::remove_file(&to);
            std::fs::copy(&server.exe, to)?;
        }

        Ok(())
    }

    /// todo(jon): use handlebars templates instead of these prebaked templates
    async fn write_metadata(&self) -> Result<()> {
        // write the Info.plist file
        match self.build.build.platform() {
            Platform::MacOS => {
                let dest = self.root_dir().join("Contents").join("Info.plist");
                let plist = self.macos_plist_contents()?;
                std::fs::write(dest, plist)?;
            }

            Platform::Ios => {
                let dest = self.root_dir().join("Info.plist");
                let plist = self.ios_plist_contents()?;
                std::fs::write(dest, plist)?;
            }

            // AndroidManifest.xml
            // er.... maybe even all the kotlin/java/gradle stuff?
            Platform::Android => {}

            // Probably some custom format or a plist file (haha)
            // When we do the proper bundle, we'll need to do something with wix templates, I think?
            Platform::Windows => {}

            // eventually we'll create the .appimage file, I guess?
            Platform::Linux => {}

            // These are served as folders, not appimages, so we don't need to do anything special (I think?)
            // Eventually maybe write some secrets/.env files for the server?
            // We could also distribute them as a deb/rpm for linux and msi for windows
            Platform::Web => {}
            Platform::Server => {}
            Platform::Liveview => {}
        }

        Ok(())
    }

    /// Run the optimizers, obfuscators, minimizers, signers, etc
    pub(crate) async fn optimize(&self) -> Result<()> {
        match self.build.build.platform() {
            Platform::Web => {
                // Compress the asset dir
                // If pre-compressing is enabled, we can pre_compress the wasm-bindgen output
                let pre_compress = self
                    .build
                    .krate
                    .should_pre_compress_web_assets(self.build.build.release);

                let bindgen_dir = self.exe_dir();
                tokio::task::spawn_blocking(move || {
                    crate::fastfs::pre_compress_folder(&bindgen_dir, pre_compress)
                })
                .await
                .unwrap()?;

                // Run SSG and cache static routes
                if self.build.build.ssg {
                    self.run_ssg().await?;
                }
            }
            Platform::MacOS => {}
            Platform::Windows => {}
            Platform::Linux => {}
            Platform::Ios => {}
            Platform::Android => {}
            Platform::Server => {}
            Platform::Liveview => {}
        }

        Ok(())
    }

    pub(crate) fn server_exe(&self) -> Option<PathBuf> {
        if let Some(_server) = &self.server {
            return Some(
                self.build
                    .krate
                    .build_dir(Platform::Server, self.build.build.release)
                    .join("server"),
            );
        }

        None
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
            .build
            .krate
            .build_dir(self.build.build.platform(), self.build.build.release);

        match self.build.build.platform() {
            Platform::Web => platform_dir.join("public"),
            Platform::Server => platform_dir.clone(), // ends up *next* to the public folder

            // These might not actually need to be called `.app` but it does let us run these with `open`
            Platform::MacOS => platform_dir.join(format!("{}.app", self.platform_exe_name())),
            Platform::Ios => platform_dir.join(format!("{}.app", self.platform_exe_name())),

            // in theory, these all could end up directly in the root dir
            Platform::Android => platform_dir.join("apk"), // .apk (after bundling)
            Platform::Linux => platform_dir.join("app"),   // .appimage (after bundling)
            Platform::Windows => platform_dir.join("app"), // .exe (after bundling)
            Platform::Liveview => platform_dir.join("app"), // .exe (after bundling)
        }
    }

    pub(crate) async fn run_wasm_bindgen(
        &self,
        input_path: &Path,
        bindgen_outdir: &Path,
    ) -> anyhow::Result<()> {
        tracing::debug!(dx_src = ?TraceSrc::Bundle, "Running wasm-bindgen");

        let input_path = input_path.to_path_buf();
        let bindgen_outdir = bindgen_outdir.to_path_buf();
        let name = self.build.krate.executable_name().to_string();
        let keep_debug = self.build.krate.config.web.wasm_opt.debug;
        let reference_types = self.build.krate.config.web.wasm_opt.reference_types;

        let start = std::time::Instant::now();
        tokio::task::spawn_blocking(move || {
            Bindgen::new()
                .input_path(&input_path)
                .web(true)
                .unwrap()
                .debug(keep_debug)
                .demangle(keep_debug)
                .keep_debug(keep_debug)
                .reference_types(reference_types)
                .remove_name_section(true)
                .remove_producers_section(true)
                .out_name(&name)
                .generate(&bindgen_outdir)
        })
        .await
        .context("Wasm-bindgen crashed while optimizing the wasm binary")?
        .context("Failed to generate wasm-bindgen bindings")?;

        tracing::debug!(dx_src = ?TraceSrc::Bundle, "wasm-bindgen complete in {:?}", start.elapsed());

        Ok(())
    }

    #[allow(unused)]
    pub(crate) fn run_wasm_opt(&self, bindgen_outdir: &std::path::Path) -> Result<()> {
        if !self.build.build.release {
            return Ok(());
        };
        self.build.status_optimizing_wasm();

        #[cfg(feature = "optimizations")]
        {
            use crate::config::WasmOptLevel;

            tracing::info!(dx_src = ?TraceSrc::Build, "Running optimization with wasm-opt...");

            let mut options = match self.build.krate.config.web.wasm_opt.level {
                WasmOptLevel::Z => {
                    wasm_opt::OptimizationOptions::new_optimize_for_size_aggressively()
                }
                WasmOptLevel::S => wasm_opt::OptimizationOptions::new_optimize_for_size(),
                WasmOptLevel::Zero => wasm_opt::OptimizationOptions::new_opt_level_0(),
                WasmOptLevel::One => wasm_opt::OptimizationOptions::new_opt_level_1(),
                WasmOptLevel::Two => wasm_opt::OptimizationOptions::new_opt_level_2(),
                WasmOptLevel::Three => wasm_opt::OptimizationOptions::new_opt_level_3(),
                WasmOptLevel::Four => wasm_opt::OptimizationOptions::new_opt_level_4(),
            };
            let wasm_file =
                bindgen_outdir.join(format!("{}_bg.wasm", self.build.krate.executable_name()));
            let old_size = wasm_file.metadata()?.len();
            options
                // WASM bindgen relies on reference types
                .enable_feature(wasm_opt::Feature::ReferenceTypes)
                .debug_info(self.build.krate.config.web.wasm_opt.debug)
                .run(&wasm_file, &wasm_file)
                .map_err(|err| crate::Error::Other(anyhow::anyhow!(err)))?;

            let new_size = wasm_file.metadata()?.len();
            tracing::debug!(
                dx_src = ?TraceSrc::Build,
                "wasm-opt reduced WASM size from {} to {} ({:2}%)",
                old_size,
                new_size,
                (new_size as f64 - old_size as f64) / old_size as f64 * 100.0
            );
        }

        Ok(())
    }

    async fn run_ssg(&self) -> anyhow::Result<()> {
        use futures_util::stream::FuturesUnordered;
        use futures_util::StreamExt;
        use tokio::process::Command;

        const PORT: u16 = 9999;

        tracing::info!("Running SSG");

        // Run the server executable
        let _child = Command::new(
            self.server_exe()
                .context("Failed to find server executable")?,
        )
        .env(dioxus_cli_config::SERVER_PORT_ENV, PORT.to_string())
        .env(dioxus_cli_config::SERVER_IP_ENV, "127.0.0.1")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true)
        .spawn()?;

        // Wait a second for the server to start
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;

        // Get the routes from the `/static_routes` endpoint
        let mut routes = reqwest::Client::builder()
            .build()?
            .post(format!("http://127.0.0.1:{PORT}/api/static_routes"))
            .send()
            .await
            .context("Failed to get static routes from server")?
            .text()
            .await
            .map(|raw| serde_json::from_str::<String>(&raw).unwrap())
            .inspect(|text| tracing::debug!("Got static routes: {text:?}"))
            .context("Failed to parse static routes from server")?
            .lines()
            .map(|line| line.to_string())
            .map(|line| async move {
                tracing::info!("SSG: {line}");
                reqwest::Client::builder()
                    .build()?
                    .get(format!("http://127.0.0.1:{PORT}{line}"))
                    .header("Accept", "text/html")
                    .send()
                    .await
            })
            .collect::<FuturesUnordered<_>>();

        while let Some(route) = routes.next().await {
            match route {
                Ok(route) => tracing::debug!("ssg success: {route:?}"),
                Err(err) => tracing::error!("ssg error: {err:?}"),
            }
        }

        // Wait a second for the cache to be written by the server
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;

        tracing::info!("SSG complete");

        drop(_child);

        Ok(())
    }

    pub(crate) fn bundle_identifier(&self) -> String {
        format!("com.dioxuslabs.{}", self.build.krate.executable_name())
    }

    fn macos_plist_contents(&self) -> Result<String> {
        handlebars::Handlebars::new()
            .render_template(
                include_str!("../../assets/macos/mac.plist.hbs"),
                &InfoPlistData {
                    display_name: self.platform_exe_name(),
                    bundle_name: self.platform_exe_name(),
                    executable_name: self.platform_exe_name(),
                    bundle_identifier: format!("com.dioxuslabs.{}", self.platform_exe_name()),
                },
            )
            .map_err(|e| e.into())
    }

    fn ios_plist_contents(&self) -> Result<String> {
        handlebars::Handlebars::new()
            .render_template(
                include_str!("../../assets/ios/ios.plist.hbs"),
                &InfoPlistData {
                    display_name: self.platform_exe_name(),
                    bundle_name: self.platform_exe_name(),
                    executable_name: self.platform_exe_name(),
                    bundle_identifier: format!("com.dioxuslabs.{}", self.platform_exe_name()),
                },
            )
            .map_err(|e| e.into())
    }

    fn build_android_app_dir(&self) -> Result<()> {
        use std::fs::{create_dir_all, write};
        let root = self.root_dir();

        // gradle
        let wrapper = root.join("gradle").join("wrapper");
        create_dir_all(&wrapper)?;
        tracing::debug!("Initialized Gradle wrapper: {:?}", wrapper);

        // build-src
        let build_src = root.join("buildSrc");
        let build_src_kotlin = build_src.join("src").join("main").join("kotlin");
        create_dir_all(&build_src)?;
        create_dir_all(&build_src_kotlin)?;
        tracing::debug!("Initialized build-src: {:?}", build_src);
        tracing::debug!("Initialized build-src/kotlin: {:?}", build_src_kotlin);

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

        // Now the buildSrc directory
        write(
            build_src.join("build.gradle.kts"),
            include_bytes!("../../assets/android/gen/buildSrc/build.gradle.kts"),
        )?;
        write(
            build_src_kotlin.join("RustPlugin.kt"),
            include_bytes!("../../assets/android/gen/buildSrc/src/main/kotlin/RustPlugin.kt"),
        )?;
        write(
            build_src_kotlin.join("BuildTask.kt"),
            include_bytes!("../../assets/android/gen/buildSrc/src/main/kotlin/BuildTask.kt"),
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

        Ok(())
    }

    /// Run any final tools to produce apks or other artifacts we might need.
    async fn assemble(&self) -> Result<()> {
        match self.build.build.platform() {
            Platform::Android => {
                let output = Command::new("gradlew")
                    .arg("assembleArm64Debug")
                    .current_dir(self.root_dir())
                    .stderr(std::process::Stdio::piped())
                    .stdout(std::process::Stdio::piped())
                    .output()
                    .await?;

                if !output.status.success() {
                    return Err(anyhow::anyhow!("Failed to assemble apk: {output:?}").into());
                }
            }
            _ => {}
        }

        Ok(())
    }

    pub(crate) fn apk_path(&self) -> PathBuf {
        self.root_dir()
            .join("app")
            .join("build")
            .join("outputs")
            .join("apk")
            .join("arm64")
            .join("debug")
            .join("app-arm64-debug.apk")
    }
}
