use super::prerender::pre_render_static_routes;
use super::templates::InfoPlistData;
use crate::wasm_bindgen::WasmBindgen;
use crate::{BuildRequest, Platform};
use crate::{Result, TraceSrc};
use anyhow::Context;
use dioxus_cli_opt::{process_file_to, AssetManifest};
use manganis::{AssetOptions, JsAssetOptions};
use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};
use std::collections::HashSet;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::atomic::Ordering;
use std::{sync::atomic::AtomicUsize, time::Duration};
use tokio::process::Command;

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
        let mut bundle = Self { app, server, build };

        tracing::debug!("Assembling app bundle");

        bundle.build.status_start_bundle();
        /*
            assume the build dir is already created by BuildRequest
            todo(jon): maybe refactor this a bit to force AppBundle to be created before it can be filled in
        */
        bundle
            .write_main_executable()
            .await
            .context("Failed to write main executable")?;
        bundle.write_server_executable().await?;
        bundle
            .write_assets()
            .await
            .context("Failed to write assets")?;
        bundle.write_metadata().await?;
        bundle.optimize().await?;
        bundle.pre_render_ssg_routes().await?;
        bundle
            .assemble()
            .await
            .context("Failed to assemble app bundle")?;

        tracing::debug!("Bundle created at {}", bundle.build.root_dir().display());

        Ok(bundle)
    }

    /// Take the output of rustc and make it into the main exe of the bundle
    ///
    /// For wasm, we'll want to run `wasm-bindgen` to make it a wasm binary along with some other optimizations
    /// Other platforms we might do some stripping or other optimizations
    /// Move the executable to the workdir
    async fn write_main_executable(&mut self) -> Result<()> {
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
                self.run_wasm_bindgen(&self.app.exe.with_extension("wasm"))
                    .await?;

                // Only run wasm-opt if the feature is enabled
                // Wasm-opt has an expensive build script that makes it annoying to keep enabled for iterative dev
                // We put it behind the "wasm-opt" feature flag so that it can be disabled when iterating on the cli
                self.run_wasm_opt(&self.build.exe_dir())?;

                // Write the index.html file with the pre-configured contents we got from pre-rendering
                std::fs::write(
                    self.build.root_dir().join("index.html"),
                    self.prepare_html()?,
                )?;
            }

            // this will require some extra oomf to get the multi architecture builds...
            // for now, we just copy the exe into the current arch (which, sorry, is hardcoded for my m1)
            // we'll want to do multi-arch builds in the future, so there won't be *one* exe dir to worry about
            // eventually `exe_dir` and `main_exe` will need to take in an arch and return the right exe path
            //
            // todo(jon): maybe just symlink this rather than copy it?
            Platform::Android => {
                self.copy_android_exe(&self.app.exe, &self.main_exe())
                    .await?;
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

        let asset_dir = self.build.asset_dir();

        // First, clear the asset dir of any files that don't exist in the new manifest
        _ = tokio::fs::create_dir_all(&asset_dir).await;
        // Create a set of all the paths that new files will be bundled to
        let mut keep_bundled_output_paths: HashSet<_> = self
            .app
            .assets
            .assets
            .values()
            .map(|a| asset_dir.join(a.bundled_path()))
            .collect();
        // The CLI creates a .version file in the asset dir to keep track of what version of the optimizer
        // the asset was processed. If that version doesn't match the CLI version, we need to re-optimize
        // all assets.
        let version_file = self.build.asset_optimizer_version_file();
        let clear_cache = std::fs::read_to_string(&version_file)
            .ok()
            .filter(|s| s == crate::VERSION.as_str())
            .is_none();
        if clear_cache {
            keep_bundled_output_paths.clear();
        }

        // one possible implementation of walking a directory only visiting files
        fn remove_old_assets<'a>(
            path: &'a Path,
            keep_bundled_output_paths: &'a HashSet<PathBuf>,
        ) -> Pin<Box<dyn Future<Output = std::io::Result<()>> + Send + 'a>> {
            Box::pin(async move {
                // If this asset is in the manifest, we don't need to remove it
                let canon_path = dunce::canonicalize(path)?;
                if keep_bundled_output_paths.contains(canon_path.as_path()) {
                    return Ok(());
                }
                // Otherwise, if it is a directory, we need to walk it and remove child files
                if path.is_dir() {
                    for entry in std::fs::read_dir(path)?.flatten() {
                        let path = entry.path();
                        remove_old_assets(&path, keep_bundled_output_paths).await?;
                    }
                    if path.read_dir()?.next().is_none() {
                        // If the directory is empty, remove it
                        tokio::fs::remove_dir(path).await?;
                    }
                } else {
                    // If it is a file, remove it
                    tokio::fs::remove_file(path).await?;
                }
                Ok(())
            })
        }

        tracing::debug!("Removing old assets");
        remove_old_assets(&asset_dir, &keep_bundled_output_paths).await?;

        // todo(jon): we also want to eventually include options for each asset's optimization and compression, which we currently aren't
        let mut assets_to_transfer = vec![];

        // Queue the bundled assets
        for (asset, bundled) in &self.app.assets.assets {
            let from = asset.clone();
            let to = asset_dir.join(bundled.bundled_path());
            tracing::debug!("Copying asset {from:?} to {to:?}");
            assets_to_transfer.push((from, to, *bundled.options()));
        }

        // And then queue the legacy assets
        // ideally, one day, we can just check the rsx!{} calls for references to assets
        for from in self.build.krate.legacy_asset_dir_files() {
            let to = asset_dir.join(from.file_name().unwrap());
            tracing::debug!("Copying legacy asset {from:?} to {to:?}");
            assets_to_transfer.push((from, to, AssetOptions::Unknown));
        }

        let asset_count = assets_to_transfer.len();
        let started_processing = AtomicUsize::new(0);
        let copied = AtomicUsize::new(0);

        // Parallel Copy over the assets and keep track of progress with an atomic counter
        let progress = self.build.progress.clone();
        // Optimizing assets is expensive and blocking, so we do it in a tokio spawn blocking task
        tokio::task::spawn_blocking(move || {
            assets_to_transfer
                .par_iter()
                .try_for_each(|(from, to, options)| {
                    let processing = started_processing.fetch_add(1, Ordering::SeqCst);
                    tracing::trace!("Starting asset copy {processing}/{asset_count} from {from:?}");

                    let res = process_file_to(options, from, to);
                    if let Err(err) = res.as_ref() {
                        tracing::error!("Failed to copy asset {from:?}: {err}");
                    }

                    let finished = copied.fetch_add(1, Ordering::SeqCst);
                    BuildRequest::status_copied_asset(
                        &progress,
                        finished,
                        asset_count,
                        from.to_path_buf(),
                    );

                    res.map(|_| ())
                })
        })
        .await
        .map_err(|e| anyhow::anyhow!("A task failed while trying to copy assets: {e}"))??;

        // Remove the wasm bindgen output directory if it exists
        _ = std::fs::remove_dir_all(self.build.wasm_bindgen_out_dir());

        // Write the version file so we know what version of the optimizer we used
        std::fs::write(
            self.build.asset_optimizer_version_file(),
            crate::VERSION.as_str(),
        )?;

        Ok(())
    }

    /// The item that we'll try to run directly if we need to.
    ///
    /// todo(jon): we should name the app properly instead of making up the exe name. It's kinda okay for dev mode, but def not okay for prod
    pub fn main_exe(&self) -> PathBuf {
        self.build.exe_dir().join(self.build.platform_exe_name())
    }

    /// We always put the server in the `web` folder!
    /// Only the `web` target will generate a `public` folder though
    async fn write_server_executable(&self) -> Result<()> {
        if let Some(server) = &self.server {
            let to = self
                .server_exe()
                .expect("server should be set if we're building a server");

            std::fs::create_dir_all(self.server_exe().unwrap().parent().unwrap())?;

            tracing::debug!("Copying server executable to: {to:?} {server:#?}");

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
                let dest = self.build.root_dir().join("Contents").join("Info.plist");
                let plist = self.macos_plist_contents()?;
                std::fs::write(dest, plist)?;
            }

            Platform::Ios => {
                let dest = self.build.root_dir().join("Info.plist");
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

                let asset_dir = self.build.asset_dir();
                tokio::task::spawn_blocking(move || {
                    crate::fastfs::pre_compress_folder(&asset_dir, pre_compress)
                })
                .await
                .unwrap()?;
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
            let mut path = self
                .build
                .krate
                .build_dir(Platform::Server, self.build.build.release);

            if cfg!(windows) {
                path.push("server.exe");
            } else {
                path.push("server");
            }

            return Some(path);
        }

        None
    }

    pub(crate) async fn run_wasm_bindgen(&mut self, input_path: &Path) -> anyhow::Result<()> {
        tracing::debug!(dx_src = ?TraceSrc::Bundle, "Running wasm-bindgen");

        let input_path = input_path.to_path_buf();
        // Make sure the bindgen output directory exists
        let bindgen_outdir = self.build.wasm_bindgen_out_dir();
        std::fs::create_dir_all(&bindgen_outdir)?;

        let name = self.build.krate.executable_name().to_string();
        let keep_debug =
            // if we're in debug mode, or we're generating debug symbols, keep debug info
            (self.build.krate.config.web.wasm_opt.debug || self.build.build.debug_symbols)
            // but only if we're not in release mode
            && !self.build.build.release;

        let start = std::time::Instant::now();

        let bindgen_version = self
            .build
            .krate
            .wasm_bindgen_version()
            .expect("this should have been checked by tool verification");

        WasmBindgen::new(&bindgen_version)
            .input_path(&input_path)
            .target("web")
            .debug(keep_debug)
            .demangle(keep_debug)
            .keep_debug(keep_debug)
            .remove_name_section(!keep_debug)
            .remove_producers_section(!keep_debug)
            .out_name(&name)
            .out_dir(&bindgen_outdir)
            .run()
            .await
            .context("Failed to generate wasm-bindgen bindings")?;

        // After running wasm-bindgen, add the js and wasm asset to the manifest
        let js_output_path = self.build.wasm_bindgen_js_output_file();
        let wasm_output_path = self.build.wasm_bindgen_wasm_output_file();
        let new_assets = [
            (
                js_output_path,
                AssetOptions::Js(JsAssetOptions::new().with_minify(true).with_preload(true)),
            ),
            (wasm_output_path, AssetOptions::Unknown),
        ];
        for (asset_path, options) in new_assets {
            let hash = manganis_core::hash::AssetHash::hash_file_contents(&asset_path)?;
            let output_path_str = asset_path.to_str().ok_or(anyhow::anyhow!(
                "Failed to convert wasm bindgen output path to string"
            ))?;
            let bundled_asset = manganis::macro_helpers::create_bundled_asset(
                output_path_str,
                hash.bytes(),
                options,
            );
            self.app.assets.assets.insert(asset_path, bundled_asset);
        }

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

    async fn pre_render_ssg_routes(&self) -> Result<()> {
        // Run SSG and cache static routes
        if !self.build.build.ssg {
            return Ok(());
        }
        self.build.status_prerendering_routes();
        pre_render_static_routes(
            &self
                .server_exe()
                .context("Failed to find server executable")?,
        )
        .await?;
        Ok(())
    }

    fn macos_plist_contents(&self) -> Result<String> {
        handlebars::Handlebars::new()
            .render_template(
                include_str!("../../assets/macos/mac.plist.hbs"),
                &InfoPlistData {
                    display_name: self.build.krate.bundled_app_name(),
                    bundle_name: self.build.krate.bundled_app_name(),
                    executable_name: self.build.platform_exe_name(),
                    bundle_identifier: self.build.krate.bundle_identifier(),
                },
            )
            .map_err(|e| e.into())
    }

    fn ios_plist_contents(&self) -> Result<String> {
        handlebars::Handlebars::new()
            .render_template(
                include_str!("../../assets/ios/ios.plist.hbs"),
                &InfoPlistData {
                    display_name: self.build.krate.bundled_app_name(),
                    bundle_name: self.build.krate.bundled_app_name(),
                    executable_name: self.build.platform_exe_name(),
                    bundle_identifier: self.build.krate.bundle_identifier(),
                },
            )
            .map_err(|e| e.into())
    }

    /// Run any final tools to produce apks or other artifacts we might need.
    async fn assemble(&self) -> Result<()> {
        if let Platform::Android = self.build.build.platform() {
            self.build.status_running_gradle();

            let output = Command::new(self.gradle_exe()?)
                .arg("assembleDebug")
                .current_dir(self.build.root_dir())
                .stderr(std::process::Stdio::piped())
                .stdout(std::process::Stdio::piped())
                .output()
                .await?;

            if !output.status.success() {
                return Err(anyhow::anyhow!("Failed to assemble apk: {output:?}").into());
            }
        }

        Ok(())
    }

    /// Run bundleRelease and return the path to the `.aab` file
    ///
    /// https://stackoverflow.com/questions/57072558/whats-the-difference-between-gradlewassemblerelease-gradlewinstallrelease-and
    pub(crate) async fn android_gradle_bundle(&self) -> Result<PathBuf> {
        let output = Command::new(self.gradle_exe()?)
            .arg("bundleRelease")
            .current_dir(self.build.root_dir())
            .output()
            .await
            .context("Failed to run gradle bundleRelease")?;

        if !output.status.success() {
            return Err(anyhow::anyhow!("Failed to bundleRelease: {output:?}").into());
        }

        let app_release = self
            .build
            .root_dir()
            .join("app")
            .join("build")
            .join("outputs")
            .join("bundle")
            .join("release");

        // Rename it to Name-arch.aab
        let from = app_release.join("app-release.aab");
        let to = app_release.join(format!(
            "{}-{}.aab",
            self.build.krate.bundled_app_name(),
            self.build.build.target_args.arch()
        ));

        std::fs::rename(from, &to).context("Failed to rename aab")?;

        Ok(to)
    }

    fn gradle_exe(&self) -> Result<PathBuf> {
        // make sure we can execute the gradlew script
        #[cfg(unix)]
        {
            use std::os::unix::prelude::PermissionsExt;
            std::fs::set_permissions(
                self.build.root_dir().join("gradlew"),
                std::fs::Permissions::from_mode(0o755),
            )?;
        }

        let gradle_exec_name = match cfg!(windows) {
            true => "gradlew.bat",
            false => "gradlew",
        };

        Ok(self.build.root_dir().join(gradle_exec_name))
    }

    pub(crate) fn apk_path(&self) -> PathBuf {
        self.build
            .root_dir()
            .join("app")
            .join("build")
            .join("outputs")
            .join("apk")
            .join("debug")
            .join("app-debug.apk")
    }

    /// Copy the Android executable to the target directory, and rename the hardcoded com_hardcoded_dioxuslabs entries
    /// to the user's app name.
    async fn copy_android_exe(&self, source: &Path, destination: &Path) -> Result<()> {
        // we might want to eventually use the objcopy logic to handle this
        //
        // https://github.com/rust-mobile/xbuild/blob/master/xbuild/template/lib.rs
        // https://github.com/rust-mobile/xbuild/blob/master/apk/src/lib.rs#L19
        std::fs::copy(source, destination)?;
        Ok(())
    }
}
