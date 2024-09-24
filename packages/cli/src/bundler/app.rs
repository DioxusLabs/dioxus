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
/// When we write the AppBundle to a folder, it'll contain each bundle for each platform under the app's name:
/// ```
/// bundle/
///   dog-app/
///       build.json                 // The build stats, config flags, and other info that was used to build the app
///       Mobile_x64.app
///       Mobile_arm64.app
///       Mobile_rosetta.app
///       server.exe
///       private/                   // The assets that were collected for the server, if they exist
///           some-secret-asset.txt
///       public/
///           index.html
///           assets/
///               logo.png
///               style.css
///      server.appimage             // contains everything - you can just ./ it to run everything
/// ```
///
/// When deploying, the build.json file will provide all the metadata that dx-deploy will use to
/// push the app to stores, set up infra, manage versions, etc.
///
/// The format of each build will follow the name plus some metadata such that when distributing you
/// can easily trim off the metadata.
#[derive(Debug)]
pub(crate) struct AppBundle {
    pub(crate) build: BuildRequest,
    pub(crate) workdir: PathBuf,

    pub(crate) app_executable: PathBuf,
    pub(crate) app_assets: AssetManifest,

    pub(crate) server_executable: Option<PathBuf>,
    pub(crate) server_assets: AssetManifest,
}

impl AppBundle {
    pub(crate) async fn new(
        build: BuildRequest,
        app_assets: AssetManifest,
        app_exe: PathBuf,
        server_exe: Option<PathBuf>,
    ) -> Result<Self> {
        let bundle = Self {
            workdir: build.krate.workdir(build.build.platform()),
            server_executable: server_exe,
            build,
            app_executable: app_exe,
            app_assets,
            server_assets: Default::default(),
        };

        bundle.build.status_start_bundle();
        bundle.prepare_workdir()?;
        bundle.write_main_executable().await?;
        bundle.write_assets().await?;
        bundle.write_metadata().await?;
        bundle.optimize().await?;

        Ok(bundle)
    }

    // Create the workdir and then clean its contents, in case it already exists
    fn prepare_workdir(&self) -> Result<()> {
        // _ = std::fs::remove_dir_all(&self.workdir);
        _ = std::fs::create_dir_all(&self.workdir);
        Ok(())
    }

    /// Take the output of rustc and make it into the main exe of the bundle
    ///
    /// For wasm, we'll want to run `wasm-bindgen` to make it a wasm binary along with some other optimizations
    /// Other platforms we might do some stripping or other optimizations
    async fn write_main_executable(&self) -> Result<()> {
        match self.build.platform() {
            // Run wasm-bindgen on the wasm binary and set its output to be in the bundle folder
            // Also run wasm-opt on the wasm binary, and sets the index.html since that's also the "executable".
            //
            // The wasm stuff will be in a folder called "wasm" in the workdir.
            //
            // Final output format:
            // ```
            // dist/
            //     web
            //         index.html
            //         wasm/
            //            app.wasm
            //            glue.js
            //            snippets/
            //                ...
            //         assets/
            //            logo.png
            // ```
            Platform::Web => {
                self.build.status_wasm_bindgen();

                // Run wasm-bindgen and drop its output into the assets folder under "dioxus"
                self.build
                    .run_wasm_bindgen(
                        &self.app_executable.with_extension("wasm"),
                        &self.bindgen_dir(),
                    )
                    .await?;

                // Only run wasm-opt if the feature is enabled
                // Wasm-opt has an expensive build script that makes it annoying to keep enabled for iterative dev
                // We put it behind the "wasm-opt" feature flag so that it can be disabled when iterating on the cli
                self.build.run_wasm_opt(&self.bindgen_dir())?;

                // Write the index.html file
                std::fs::write(self.workdir.join("index.html"), self.build.prepare_html()?)?;
            }

            // Move the executable to the workdir
            Platform::Desktop => {
                std::fs::copy(self.app_executable.clone(), self.workdir.join("app"))?;
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
        if self.build.platform() == Platform::Server {
            return Ok(());
        }

        let asset_dir = self.asset_dir();
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
        // std::fs::create_dir_all(&destination.join(self.build.app_name()))?;

        match self.build.platform() {
            // Nothing special to do - just copy the workdir to the output location
            Platform::Web => {
                std::fs::create_dir_all(&destination.join("web"))?;
                crate::fastfs::copy_asset(&self.workdir, &destination.join("web"))?;
                Ok(destination.join("web"))
            }

            // Create a final .app/.exe/etc depending on the host platform, not dependent on the host
            Platform::Desktop => {
                // for now, until we have bundled hotreload, just copy the executable to the output location
                // let output_location = destination.join(self.build.app_name());
                Ok(self.app_executable.clone())
                // Ok(output_location)
            }

            Platform::Server => {
                std::fs::copy(
                    self.app_executable.clone(),
                    destination.join(self.build.app_name()),
                )?;

                Ok(destination.join(self.build.app_name()))
            }
            Platform::Liveview => Ok(self.app_executable.clone()),

            // Create a .ipa, only from macOS
            Platform::Ios => todo!(),

            // Create a .exe, from linux/mac/windows
            Platform::Android => todo!(),
        }
    }

    fn bindgen_dir(&self) -> PathBuf {
        self.workdir.join("wasm")
    }

    pub(crate) fn all_source_assets(&self) -> Vec<PathBuf> {
        // Merge the legacy asset dir assets with the assets from the manifest
        // Legacy assets need to retain their name in case they're referenced in the manifest
        // todo: we should only copy over assets that appear in `img { src: "assets/logo.png" }` to
        // properly deprecate the legacy asset dir
        self.app_assets
            .assets
            .keys()
            .cloned()
            .chain(self.build.krate.legacy_asset_dir_files())
            .collect::<Vec<_>>()
    }

    async fn write_metadata(&self) -> Result<()> {
        Ok(())
    }

    pub(crate) fn asset_dir(&self) -> PathBuf {
        let dir: PathBuf = match self.build.platform() {
            Platform::Web => self.workdir.join("assets"),
            Platform::Desktop => self.workdir.join("Resources"),
            Platform::Ios => self.workdir.join("Resources"),
            Platform::Android => self.workdir.join("assets"),
            Platform::Server => self.workdir.join("assets"),
            Platform::Liveview => self.workdir.join("assets"),
        };

        if !dir.exists() {
            std::fs::create_dir_all(&dir).expect("Failed to create asset dir in temp dir");
        }

        dir
    }

    /// Run the optimizers, obfuscators, minimizers, etc
    pub(crate) async fn optimize(&self) -> Result<()> {
        match self.build.platform() {
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
