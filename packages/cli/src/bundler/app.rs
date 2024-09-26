use crate::assets::AssetManifest;
use crate::Result;
use crate::{BuildRequest, Platform};
use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};
use std::path::PathBuf;
use std::sync::atomic::AtomicUsize;

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
/// When we write the AppBundle to a folder, it'll contain each bundle for each platform under the app's name:
/// ```
/// dog-app/
///   build/
///       web/
///         server
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
///            assets/
///                some-secret-asset.txt
///            public/
///                index.html
///                assets/
///                    logo.png
///                    style.css
/// ```
///
/// When deploying, the build.json file will provide all the metadata that dx-deploy will use to
/// push the app to stores, set up infra, manage versions, etc.
///
/// The format of each build will follow the name plus some metadata such that when distributing you
/// can easily trim off the metadata.
///
/// The idea here is that we can run any of the programs in the same way that they're deployed
#[derive(Debug)]
pub(crate) struct AppBundle {
    pub(crate) build: BuildRequest,

    /// The directory where the build is located
    ///
    /// app.app
    /// app.appimage
    pub(crate) build_dir: PathBuf,

    pub(crate) app: PathBuf,
    pub(crate) app_assets: AssetManifest,

    pub(crate) server: Option<PathBuf>,
    pub(crate) server_assets: AssetManifest,
}

impl AppBundle {
    /// ## Web:
    /// Create a folder that is somewhat similar to an app-image (exe + asset)
    /// ```
    /// web/
    ///     server
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
    ///
    /// Linux:
    /// https://docs.appimage.org/reference/appdir.html#ref-appdir
    /// current_exe.join("Assets")
    /// ```
    /// app.appimage/
    ///     main.exe
    ///     main.desktop
    ///     package.json
    ///     usr/
    ///         logo.png
    /// ```
    ///
    /// ## Mac + iOS + TVOS + VisionOS:
    /// We simply use the macos/ios format where binaries are in `Contents/MacOS` and assets are in `Contents/Resources`
    /// ```
    /// blah.app/
    ///     Contents/
    ///         Info.plist
    ///         MacOS/
    ///             Frameworks/
    ///         Resources/
    ///             blah.icns
    ///             blah.png
    ///         CodeResources
    ///         _CodeSignature/
    /// ```
    ///
    /// ## Android:
    /// ```
    /// app.apk/
    ///   lib/
    ///       armeabi-v7a/
    ///           libmyapp.so
    ///       arm64-v8a/
    ///           libmyapp.so
    ///   assets/
    ///       logo.png
    /// ```
    ///
    /// Windows:
    /// https://superuser.com/questions/749447/creating-a-single-file-executable-from-a-directory-in-windows
    /// Windows does not provide an AppImage format, so instead we're going build the same folder
    /// structure as an AppImage, but when distributing, we'll create a .exe that embeds the resources
    /// as an embedded .zip file. When the app runs, it will implicitly unzip its resources into the
    /// Program Files folder. Any subsquent launches of the parent .exe will simply call the AppRun.exe
    /// entrypoint in the associated Program Files folder.
    ///
    /// This is, in essence, the same as an installer, so we might eventually just support something like msi/msix
    /// which functionally do the same thing but with a sleeker UI.
    ///
    /// This means no installers are required and we can bake an updater into the host exe.
    /// current_exe.join("usr")
    /// ```
    /// app.appimage/
    ///     main.exe
    ///     main.desktop
    ///     package.json
    ///     usr/
    ///         logo.png
    /// ```
    pub(crate) async fn new(
        build: BuildRequest,
        app_assets: AssetManifest,
        app_executable: PathBuf,
        server_executable: Option<PathBuf>,
    ) -> Result<Self> {
        let mut bundle = Self {
            build_dir: build.krate.build_dir(build.build.platform()),
            server: server_executable,
            app: app_executable,
            app_assets,
            server_assets: Default::default(),
            build,
        };

        // Add any legacy assets to the bundle manifest
        for legacy in bundle.build.krate.legacy_asset_dir_files() {
            if let Ok(legacy) = legacy.canonicalize() {
                bundle
                    .app_assets
                    .insert_legacy_asset(&bundle.build.krate.legacy_asset_dir(), &legacy);
            }
        }

        bundle.build.status_start_bundle();
        bundle.prepare_build_dir()?;
        bundle.write_main_executable().await?;
        // bundle.write_server_executable().await?;
        bundle.write_assets().await?;
        bundle.write_metadata().await?;
        bundle.optimize().await?;

        Ok(bundle)
    }

    // Create the workdir and then clean its contents, in case it already exists
    fn prepare_build_dir(&self) -> Result<()> {
        _ = std::fs::create_dir_all(&self.build_dir);
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
            // app/
            //     build/
            //         desktop.app        // mac
            //         mobile.ipa         // ios
            //         mobile.apk         // android (unbundled, not actually zipped yet)
            //         server.appimage    // server
            //         app.exe            // windows
            //         server
            //         public/            // web
            //             index.html
            //             assets/
            //                 logo.png
            //
            // dx/
            //     app/
            //         desktop/
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
                let public_dir = self.build_dir.join("public");
                let wasm_dir = public_dir.join("wasm");

                self.build.status_wasm_bindgen();

                // Run wasm-bindgen and drop its output into the assets folder under "dioxus"
                self.build
                    .run_wasm_bindgen(&self.app.with_extension("wasm"), &wasm_dir)
                    .await?;

                // Only run wasm-opt if the feature is enabled
                // Wasm-opt has an expensive build script that makes it annoying to keep enabled for iterative dev
                // We put it behind the "wasm-opt" feature flag so that it can be disabled when iterating on the cli
                self.build.run_wasm_opt(&wasm_dir)?;

                // Write the index.html file
                std::fs::write(public_dir.join("index.html"), self.build.prepare_html()?)?;

                // write the server executable
                if let Some(server) = &self.server {
                    std::fs::copy(server, self.build_dir.join("server"))?;
                }
            }

            Platform::Desktop => {
                // for now, until we have bundled hotreload, just copy the executable to the output location
                let work_dir = self.build_dir.join("App.app").join("Contents");
                let app_dir = work_dir.join("MacOS");
                let assets_dir = work_dir.join("Resources");

                std::fs::create_dir_all(&work_dir)?;
                std::fs::create_dir_all(&app_dir)?;
                std::fs::create_dir_all(&assets_dir)?;

                std::fs::copy(self.app.clone(), app_dir.join("app"))?;
            }

            Platform::Ios => {}
            Platform::Server => {}
            Platform::Liveview => {}
            Platform::Android => todo!("android not yet supported!"),
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

        let build_dir = self.build_dir.clone();

        let asset_dir: PathBuf = match self.build.build.platform() {
            Platform::Web => build_dir.join("public").join("assets"),
            Platform::Desktop => self
                .build_dir
                .join("App.app")
                .join("Contents")
                .join("Resources"),
            Platform::Ios => build_dir.join("App.app").join("Contents").join("Resources"),
            Platform::Android => build_dir.join("assets"),
            Platform::Server => build_dir.join("assets"),
            Platform::Liveview => build_dir.join("assets"),
        };

        std::fs::create_dir_all(&asset_dir)?;

        let assets = self.all_source_assets();
        let asset_count = assets.len();
        let assets_finished = AtomicUsize::new(0);
        let optimize = false;
        let pre_compress = false;

        // Parallel Copy over the assets and keep track of progress with an atomic counter
        assets.par_iter().try_for_each(|asset| {
            self.build.status_copying_asset(
                assets_finished.fetch_add(0, std::sync::atomic::Ordering::SeqCst),
                asset_count,
                asset,
            );

            let res = self
                .app_assets
                .copy_asset_to(&asset_dir, asset, optimize, pre_compress);

            if let Err(err) = res {
                tracing::error!("Failed to copy asset {asset:?}: {err}");
            }

            self.build.status_finished_asset(
                assets_finished.fetch_add(1, std::sync::atomic::Ordering::SeqCst),
                asset_count,
                asset,
            );

            Ok(()) as anyhow::Result<()>
        })?;

        Ok(())
    }

    /// Take the workdir and copy it to the output location, returning the path to final bundle
    ///
    /// Perform any finishing steps here:
    /// - Signing the bundle
    pub(crate) fn finish(&self, destination: PathBuf) -> Result<PathBuf> {
        match self.build.build.platform() {
            // Nothing special to do - just copy the workdir to the output location
            Platform::Web => {
                let work_dir = self.build_dir.join("public");
                let out_dir = destination.join("public");
                crate::fastfs::copy_asset(&work_dir, &out_dir)?;
                Ok(out_dir)
            }

            // Create a final .app/.exe/etc depending on the host platform, not dependent on the host
            Platform::Desktop => {
                let out_app = destination
                    .join(self.build_dir.file_name().unwrap())
                    .with_extension("app");
                crate::fastfs::copy_asset(&self.build_dir, &out_app)?;
                Ok(out_app)
            }

            Platform::Server => {
                std::fs::copy(self.app.clone(), destination.join(self.build.app_name()))?;

                Ok(destination.join(self.build.app_name()))
            }

            Platform::Liveview => Ok(self.app.clone()),

            // Create a .ipa, only from macOS
            Platform::Ios => todo!("Implement iOS bundling"),

            // Create a .exe, from linux/mac/windows
            Platform::Android => todo!("Implement Android bundling"),
        }
    }

    async fn write_server_executable(&self) {
        todo!()
    }

    pub fn copy_server(&self, destination: &PathBuf) -> Result<Option<PathBuf>> {
        if let Some(server) = &self.server {
            let to = destination.join("server");
            _ = std::fs::remove_file(&to);
            std::fs::copy(server, &to)?;
            return Ok(Some(to));
        }

        Ok(None)
    }

    fn bindgen_dir(&self) -> PathBuf {
        self.build_dir.join("public").join("wasm")
    }

    pub(crate) fn all_source_assets(&self) -> Vec<PathBuf> {
        // Merge the legacy asset dir assets with the assets from the manifest
        // Legacy assets need to retain their name in case they're referenced in the manifest
        // todo: we should only copy over assets that appear in `img { src: "assets/logo.png" }` to
        // properly deprecate the legacy asset dir
        let mut assets = self
            .app_assets
            .assets
            .keys()
            .cloned()
            .chain(self.build.krate.legacy_asset_dir_files())
            .collect::<Vec<_>>();

        assets.dedup();

        assets
    }

    async fn write_metadata(&self) -> Result<()> {
        // write the Info.plist file
        match self.build.build.platform() {
            Platform::Desktop => {
                let src = include_str!("../../assets/some.plist");
                let dest = self
                    .build_dir
                    .join("App.app")
                    .join("Contents")
                    .join("Info.plist");
                std::fs::write(dest, src)?;
            }
            _ => {}
        }

        Ok(())
    }

    /// Run the optimizers, obfuscators, minimizers, etc
    pub(crate) async fn optimize(&self) -> Result<()> {
        match self.build.build.platform() {
            Platform::Web => {
                // Compress the asset dir
                // // If pre-compressing is enabled, we can pre_compress the wasm-bindgen output
                // let pre_compress = self
                //     .krate
                //     .should_pre_compress_web_assets(self.build.release);

                // tokio::task::spawn_blocking(move || {
                //     pre_compress_folder(&bindgen_outdir, pre_compress)
                // })
                // .await
                // .unwrap()?;
            }
            Platform::Desktop => {}
            Platform::Ios => {}
            Platform::Android => {}
            Platform::Server => {}
            Platform::Liveview => {}
        }

        Ok(())
    }
}
