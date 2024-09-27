use crate::assets::AssetManifest;
use crate::Result;
use crate::{BuildRequest, Platform};
use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};
use std::path::PathBuf;
use std::sync::atomic::AtomicUsize;

const EXE_WRITTEN_NAME: &str = "DioxusApp";
pub const MAC_APP_NAME: &str = "DioxusApp.app";

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
#[derive(Debug)]
pub(crate) struct AppBundle {
    pub(crate) build: BuildRequest,

    /// The directory where the build is located
    ///
    /// app.app
    /// app.appimage
    pub(crate) build_dir: PathBuf,

    pub(crate) cargo_app_exe: PathBuf,
    pub(crate) app_assets: AssetManifest,

    pub(crate) cargo_server_exe: Option<PathBuf>,
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
    ///     assets/
    ///         logo.png
    /// ```
    ///
    /// ## Mac + iOS + TVOS + VisionOS:
    /// We simply use the macos/ios format where binaries are in `Contents/MacOS` and assets are in `Contents/Resources`
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
    /// assets::root() ->
    ///     mac -> ../Resources/
    ///     ios -> ../Resources/
    ///     android -> assets/
    ///     server -> assets/
    ///     liveview -> assets/
    ///     web -> /assets/
    /// root().join(bundled)
    pub(crate) async fn new(
        build: BuildRequest,
        app_assets: AssetManifest,
        app_executable: PathBuf,
        server_executable: Option<PathBuf>,
    ) -> Result<Self> {
        let bundle = Self {
            build_dir: build.krate.build_dir(build.build.platform()),
            cargo_server_exe: server_executable,
            cargo_app_exe: app_executable,
            app_assets,
            server_assets: Default::default(),
            build,
        };

        tracing::debug!("Assembling app bundle");

        bundle.build.status_start_bundle();
        bundle.prepare_build_dir()?;
        bundle.write_main_executable().await?;
        bundle.write_server_executable().await?;
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
                    .run_wasm_bindgen(&self.cargo_app_exe.with_extension("wasm"), &wasm_dir)
                    .await?;

                // Only run wasm-opt if the feature is enabled
                // Wasm-opt has an expensive build script that makes it annoying to keep enabled for iterative dev
                // We put it behind the "wasm-opt" feature flag so that it can be disabled when iterating on the cli
                self.build.run_wasm_opt(&wasm_dir)?;

                // Write the index.html file
                std::fs::write(public_dir.join("index.html"), self.build.prepare_html()?)?;

                // write the server executable
                if let Some(server) = &self.cargo_server_exe {
                    std::fs::copy(server, self.build_dir.join("server"))?;
                }
            }

            Platform::Desktop => {
                // for now, until we have bundled hotreload, just copy the executable to the output location
                let work_dir = self.build_dir.join(MAC_APP_NAME).join("Contents");
                let app_dir = work_dir.join("MacOS");
                let assets_dir = work_dir.join("Resources").join("assets");

                std::fs::create_dir_all(&work_dir)?;
                std::fs::create_dir_all(&app_dir)?;
                std::fs::create_dir_all(&assets_dir)?;

                std::fs::copy(self.cargo_app_exe.clone(), app_dir.join(EXE_WRITTEN_NAME))?;
            }

            // Follows a different format than mac
            Platform::Ios => {
                // for now, until we have bundled hotreload, just copy the executable to the output location
                let work_dir = self.build_dir.join(MAC_APP_NAME);
                let app_dir = work_dir.clone();
                let assets_dir = work_dir.join("assets");

                std::fs::create_dir_all(&work_dir)?;
                std::fs::create_dir_all(&app_dir)?;
                std::fs::create_dir_all(&assets_dir)?;

                std::fs::copy(self.cargo_app_exe.clone(), app_dir.join(EXE_WRITTEN_NAME))?;
            }

            Platform::Android => {
                // https://github.com/rust-mobile/xbuild/blob/master/xbuild/template/lib.rs
                todo!("android not yet supported!")
            }

            Platform::Server => {}
            Platform::Liveview => {}
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
        _ = std::fs::remove_dir_all(&asset_dir);
        _ = std::fs::create_dir_all(&asset_dir);

        // Copy over the bundled assets
        for asset in self.app_assets.assets.keys() {
            let bundled = self.app_assets.assets.get(asset).unwrap();
            let from = &bundled.absolute;
            let to = asset_dir.join(&bundled.bundled);
            tracing::debug!("Copying asset {from:?} to {to:?}");
            std::fs::copy(from, to)?;
        }

        // And then copy over the legacy assets
        for file in self.build.krate.legacy_asset_dir_files() {
            let from = &file;
            let to = asset_dir.join(file.file_name().unwrap());
            tracing::debug!("Copying legacy asset {from:?} to {to:?}");
            std::fs::copy(file, to)?;
        }

        // todo: implement par_iter
        // let asset_count = assets.len();
        // let assets_finished = AtomicUsize::new(0);
        // let optimize = false;
        // let pre_compress = false;

        // // Parallel Copy over the assets and keep track of progress with an atomic counter
        // assets.par_iter().try_for_each(|(asset, legacy)| {
        //     self.run_asset_transfer(
        //         asset,
        //         &assets_finished,
        //         asset_count,
        //         &asset_dir,
        //         optimize,
        //         pre_compress,
        //         *legacy,
        //     )
        // })?;

        Ok(())
    }

    fn run_asset_transfer(
        &self,
        asset: &PathBuf,
        assets_finished: &AtomicUsize,
        asset_count: usize,
        asset_dir: &PathBuf,
        optimize: bool,
        pre_compress: bool,
        legacy: bool,
    ) -> std::result::Result<(), anyhow::Error> {
        self.build.status_copying_asset(
            assets_finished.fetch_add(0, std::sync::atomic::Ordering::SeqCst),
            asset_count,
            asset,
        );

        // let res = self
        //     .app_assets
        //     .copy_asset_to(&asset_dir, asset, optimize, pre_compress, legacy);

        // if let Err(err) = res {
        //     tracing::error!("Failed to copy asset {asset:?}: {err}");
        // }

        // self.build.status_finished_asset(
        //     assets_finished.fetch_add(1, std::sync::atomic::Ordering::SeqCst),
        //     asset_count,
        //     asset,
        // );

        Ok(()) as anyhow::Result<()>
    }

    pub fn main_exe(&self) -> PathBuf {
        match self.build.build.platform() {
            Platform::Web => self.build_dir.join("public").join("index.html"),
            Platform::Desktop => self
                .build_dir
                .join(MAC_APP_NAME)
                .join("Contents")
                .join("MacOS")
                .join(EXE_WRITTEN_NAME),
            Platform::Ios => self
                .build_dir
                .join(MAC_APP_NAME)
                // .join("Contents")
                // .join("MacOS")
                .join(EXE_WRITTEN_NAME),
            Platform::Android => todo!(),
            Platform::Server => self.build_dir.join("server"),
            Platform::Liveview => self.build_dir.join("server"),
        }
    }

    pub fn asset_dir(&self) -> PathBuf {
        let build_dir = &self.build_dir;

        match self.build.build.platform() {
            Platform::Web => build_dir.join("public").join("assets"),
            Platform::Desktop => self
                .build_dir
                .join(MAC_APP_NAME)
                .join("Contents")
                .join("Resources")
                .join("assets"),
            Platform::Ios => build_dir.join(MAC_APP_NAME).join("assets"),
            Platform::Android => build_dir.join("assets"),
            Platform::Server => build_dir.join("assets"),
            Platform::Liveview => build_dir.join("assets"),
        }
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
                    .with_extension(EXE_WRITTEN_NAME);
                crate::fastfs::copy_asset(&self.build_dir, &out_app)?;
                Ok(out_app)
            }

            Platform::Server => {
                std::fs::copy(
                    self.cargo_app_exe.clone(),
                    destination.join(self.build.app_name()),
                )?;

                Ok(destination.join(self.build.app_name()))
            }

            Platform::Liveview => Ok(self.cargo_app_exe.clone()),

            // Create a .ipa, only from macOS
            Platform::Ios => todo!("Implement iOS bundling"),

            // Create a .exe, from linux/mac/windows
            Platform::Android => todo!("Implement Android bundling"),
        }
    }

    async fn write_server_executable(&self) -> Result<()> {
        if let Some(server) = &self.cargo_server_exe {
            let to = self.build_dir.join("server");
            tracing::debug!("Copying server executable from {server:?} to {to:?}");

            // Remove the old server executable if it exists, since we might corrupt it :(
            _ = std::fs::remove_file(&to);
            std::fs::copy(server, to)?;
        }

        Ok(())
    }

    pub fn copy_server(&self, destination: &PathBuf) -> Result<Option<PathBuf>> {
        if let Some(server) = &self.cargo_server_exe {
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

    pub(crate) fn all_app_assets(&self) -> Vec<(PathBuf, bool)> {
        // Merge the legacy asset dir assets with the assets from the manifest
        // Legacy assets need to retain their name in case they're referenced in the manifest
        // todo: we should only copy over assets that appear in `img { src: "assets/logo.png" }` to
        // properly deprecate the legacy asset dir
        let mut assets = self
            .app_assets
            .assets
            .keys()
            .cloned()
            .map(|p| (p, false))
            .chain(
                self.build
                    .krate
                    .legacy_asset_dir_files()
                    .into_iter()
                    .map(|p| (p, true)),
            )
            .collect::<Vec<_>>();

        assets.dedup();

        assets
    }

    pub(crate) fn all_server_assets(&self) -> Vec<PathBuf> {
        let mut assets = self
            .server_assets
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
                let src = include_str!("../../assets/mac.plist");
                let dest = self
                    .build_dir
                    .join(MAC_APP_NAME)
                    .join("Contents")
                    .join("Info.plist");
                std::fs::write(dest, src)?;
            }
            Platform::Ios => {
                let src = include_str!("../../assets/ios.plist");
                let dest = self.build_dir.join(MAC_APP_NAME).join("Info.plist");
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

    pub(crate) fn server(&self) -> Option<PathBuf> {
        if let Some(_server) = &self.cargo_server_exe {
            return Some(self.build_dir.join("server"));
        }

        None
    }

    // returns the .app/.apk/.appimage
    pub(crate) fn app_root(&self) -> PathBuf {
        match self.build.build.platform() {
            Platform::Desktop => self.build_dir.join(MAC_APP_NAME),
            Platform::Ios => self.build_dir.join(MAC_APP_NAME),
            Platform::Web => todo!(),
            Platform::Android => todo!(),
            Platform::Server => todo!(),
            Platform::Liveview => todo!(),
        }
    }
}
