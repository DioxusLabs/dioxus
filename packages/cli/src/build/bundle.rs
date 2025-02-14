use super::prerender::pre_render_static_routes;
use super::templates::InfoPlistData;
use crate::{BuildRequest, Platform, WasmOptConfig};
use crate::{Result, TraceSrc};
use anyhow::Context;
use dioxus_cli_opt::{process_file_to, AssetManifest};
use manganis::{AssetOptions, JsAssetOptions};
use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};
use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::atomic::Ordering;
use std::{collections::HashSet, io::Write};
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
                self.bundle_web().await?;
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
        tracing::trace!(
            "Keeping bundled output paths: {:#?}",
            keep_bundled_output_paths
        );
        remove_old_assets(&asset_dir, &keep_bundled_output_paths).await?;

        // todo(jon): we also want to eventually include options for each asset's optimization and compression, which we currently aren't
        let mut assets_to_transfer = vec![];

        // Queue the bundled assets
        for (asset, bundled) in &self.app.assets.assets {
            let from = asset.clone();
            let to = asset_dir.join(bundled.bundled_path());

            // prefer to log using a shorter path relative to the workspace dir by trimming the workspace dir
            let from_ = from
                .strip_prefix(self.build.krate.workspace_dir())
                .unwrap_or(from.as_path());
            let to_ = from
                .strip_prefix(self.build.krate.workspace_dir())
                .unwrap_or(to.as_path());

            tracing::debug!("Copying asset {from_:?} to {to_:?}");
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
        let ws_dir = self.build.krate.workspace_dir();
        // Optimizing assets is expensive and blocking, so we do it in a tokio spawn blocking task
        tokio::task::spawn_blocking(move || {
            assets_to_transfer
                .par_iter()
                .try_for_each(|(from, to, options)| {
                    let processing = started_processing.fetch_add(1, Ordering::SeqCst);
                    let from_ = from.strip_prefix(&ws_dir).unwrap_or(from);
                    tracing::trace!(
                        "Starting asset copy {processing}/{asset_count} from {from_:?}"
                    );

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

        // // Remove the wasm bindgen output directory if it exists
        // _ = std::fs::remove_dir_all(self.build.wasm_bindgen_out_dir());

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

                self.build.status_compressing_assets();
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

    /// Bundle the web app
    /// - Run wasm-bindgen
    /// - Bundle split
    /// - Run wasm-opt
    /// - Register the .wasm and .js files with the asset system
    async fn bundle_web(&mut self) -> Result<()> {
        use crate::{wasm_bindgen::WasmBindgen, wasm_opt};
        use std::fmt::Write;

        // Locate the output of the build files and the bindgen output
        // We'll fill these in a second if they don't already exist
        let bindgen_outdir = self.build.wasm_bindgen_out_dir();
        let prebindgen = self.app.exe.clone();
        let post_bindgen_wasm = self.build.wasm_bindgen_wasm_output_file();
        let should_bundle_split = self.build.build.experimental_wasm_split;
        let rustc_exe = self.app.exe.with_extension("wasm");
        let bindgen_version = self
            .build
            .krate
            .wasm_bindgen_version()
            .expect("this should have been checked by tool verification");

        // Prepare any work dirs
        std::fs::create_dir_all(&bindgen_outdir)?;

        // Prepare our configuration
        //
        // we turn off debug symbols in dev mode but leave them on in release mode (weird!) since
        // wasm-opt and wasm-split need them to do better optimizations.
        //
        // We leave demangling to false since it's faster and these tools seem to prefer the raw symbols.
        // todo(jon): investigate if the chrome extension needs them demangled or demangles them automatically.
        let will_wasm_opt = (self.build.build.release || self.build.build.experimental_wasm_split)
            && crate::wasm_opt::wasm_opt_available();
        let keep_debug = self.build.krate.config.web.wasm_opt.debug
            || self.build.build.debug_symbols
            || self.build.build.experimental_wasm_split
            || !self.build.build.release
            || will_wasm_opt;
        let demangle = false;
        let wasm_opt_options = WasmOptConfig {
            memory_packing: self.build.build.experimental_wasm_split,
            debug: self.build.build.debug_symbols,
            ..self.build.krate.config.web.wasm_opt.clone()
        };

        // Run wasm-bindgen. Some of the options are not "optimal" but will be fixed up by wasm-opt
        //
        // There's performance implications here. Running with --debug is slower than without
        // We're keeping around lld sections and names but wasm-opt will fix them
        // todo(jon): investigate a good balance of wiping debug symbols during dev (or doing a double build?)
        self.build.status_wasm_bindgen_start();
        tracing::debug!(dx_src = ?TraceSrc::Bundle, "Running wasm-bindgen");
        let start = std::time::Instant::now();
        WasmBindgen::new(&bindgen_version)
            .input_path(&rustc_exe)
            .target("web")
            .debug(keep_debug)
            .demangle(demangle)
            .keep_debug(keep_debug)
            .keep_lld_sections(true)
            .out_name(self.build.krate.executable_name())
            .out_dir(&bindgen_outdir)
            .remove_name_section(!will_wasm_opt)
            .remove_producers_section(!will_wasm_opt)
            .run()
            .await
            .context("Failed to generate wasm-bindgen bindings")?;
        tracing::debug!(dx_src = ?TraceSrc::Bundle, "wasm-bindgen complete in {:?}", start.elapsed());

        // Run bundle splitting if the user has requested it
        // It's pretty expensive but because of rayon should be running separate threads, hopefully
        // not blocking this thread. Dunno if that's true
        if should_bundle_split {
            self.build.status_splitting_bundle();

            if !will_wasm_opt {
                return Err(anyhow::anyhow!(
                    "Bundle splitting requires wasm-opt to be installed or the CLI to be built with `--features optimizations`. Please install wasm-opt and try again."
                )
                .into());
            }

            // Load the contents of these binaries since we need both of them
            // We're going to use the default makeLoad glue from wasm-split
            let original = std::fs::read(&prebindgen)?;
            let bindgened = std::fs::read(&post_bindgen_wasm)?;
            let mut glue = wasm_split_cli::MAKE_LOAD_JS.to_string();

            // Run the emitter
            let splitter = wasm_split_cli::Splitter::new(&original, &bindgened);
            let modules = splitter
                .context("Failed to parse wasm for splitter")?
                .emit()
                .context("Failed to emit wasm split modules")?;

            // Write the chunks that contain shared imports
            // These will be in the format of chunk_0_modulename.wasm - this is hardcoded in wasm-split
            tracing::debug!("Writing split chunks to disk");
            for (idx, chunk) in modules.chunks.iter().enumerate() {
                let path = bindgen_outdir.join(format!("chunk_{}_{}.wasm", idx, chunk.module_name));
                wasm_opt::write_wasm(&chunk.bytes, &path, &wasm_opt_options).await?;
                writeln!(
                    glue, "export const __wasm_split_load_chunk_{idx} = makeLoad(\"/assets/{url}\", [], fusedImports);",
                    url = self
                        .app
                        .assets
                        .register_asset(&path, AssetOptions::Unknown)?.bundled_path(),
                )?;
            }

            // Write the modules that contain the entrypoints
            tracing::debug!("Writing split modules to disk");
            for (idx, module) in modules.modules.iter().enumerate() {
                let comp_name = module
                    .component_name
                    .as_ref()
                    .context("generated bindgen module has no name?")?;

                let path = bindgen_outdir.join(format!("module_{}_{}.wasm", idx, comp_name));
                wasm_opt::write_wasm(&module.bytes, &path, &wasm_opt_options).await?;

                let hash_id = module.hash_id.as_ref().unwrap();

                writeln!(
                    glue,
                    "export const __wasm_split_load_{module}_{hash_id}_{comp_name} = makeLoad(\"/assets/{url}\", [{deps}], fusedImports);",
                    module = module.module_name,


                    // Again, register this wasm with the asset system
                    url = self
                        .app
                        .assets
                        .register_asset(&path, AssetOptions::Unknown)?.bundled_path(),

                    // This time, make sure to write the dependencies of this chunk
                    // The names here are again, hardcoded in wasm-split - fix this eventually.
                    deps = module
                        .relies_on_chunks
                        .iter()
                        .map(|idx| format!("__wasm_split_load_chunk_{idx}"))
                        .collect::<Vec<_>>()
                        .join(", ")
                )?;
            }

            // Write the js binding
            // It's not registered as an asset since it will get included in the main.js file
            let js_output_path = bindgen_outdir.join("__wasm_split.js");
            std::fs::write(&js_output_path, &glue)?;

            // Make sure to write some entropy to the main.js file so it gets a new hash
            // If we don't do this, the main.js file will be cached and never pick up the chunk names
            let uuid = uuid::Uuid::new_v5(&uuid::Uuid::NAMESPACE_URL, glue.as_bytes());
            std::fs::OpenOptions::new()
                .append(true)
                .open(self.build.wasm_bindgen_js_output_file())
                .context("Failed to open main.js file")?
                .write_all(format!("/*{uuid}*/").as_bytes())?;

            // Write the main wasm_bindgen file and register it with the asset system
            // This will overwrite the file in place
            // We will wasm-opt it in just a second...
            std::fs::write(&post_bindgen_wasm, modules.main.bytes)?;
        }

        // Make sure to optimize the main wasm file if requested or if bundle splitting
        if should_bundle_split || self.build.build.release {
            self.build.status_optimizing_wasm();
            wasm_opt::optimize(&post_bindgen_wasm, &post_bindgen_wasm, &wasm_opt_options).await?;
        }

        // Make sure to register the main wasm file with the asset system
        self.app
            .assets
            .register_asset(&post_bindgen_wasm, AssetOptions::Unknown)?;

        // Register the main.js with the asset system so it bundles in the snippets and optimizes
        self.app.assets.register_asset(
            &self.build.wasm_bindgen_js_output_file(),
            AssetOptions::Js(JsAssetOptions::new().with_minify(true).with_preload(true)),
        )?;

        // Write the index.html file with the pre-configured contents we got from pre-rendering
        std::fs::write(
            self.build.root_dir().join("index.html"),
            self.prepare_html()?,
        )?;

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
