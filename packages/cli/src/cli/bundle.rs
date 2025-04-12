use crate::{AppBuilder, BuildArgs, BuildRequest, Platform};
use anyhow::{anyhow, Context};
use dioxus_cli_config::{server_ip, server_port};
use futures_util::stream::FuturesUnordered;
use futures_util::StreamExt;
use path_absolutize::Absolutize;
use std::collections::HashMap;
use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::Path,
    time::Duration,
};
use tauri_bundler::{BundleBinary, BundleSettings, PackageSettings, SettingsBuilder};
use tokio::process::Command;

use super::*;

/// Bundle an app and its assets.
///
/// This only takes a single build into account. To build multiple targets, use multiple calls to bundle.
///
/// ```
/// dioxus bundle --target <target>
/// dioxus bundle --target <target>
/// ```
///
/// Note that building the server will perform a client build as well:
///
/// ```
/// dioxus bundle --platform server
/// ```
///
/// This will produce a client `public` folder and the associated server executable in the output folder.
#[derive(Clone, Debug, Parser)]
pub struct Bundle {
    /// The package types to bundle
    #[clap(long)]
    pub package_types: Option<Vec<crate::PackageType>>,

    /// The directory in which the final bundle will be placed.
    ///
    /// Relative paths will be placed relative to the current working directory if specified.
    /// Otherwise, the out_dir path specified in Dioxus.toml will be used (relative to the crate root).
    ///
    /// We will flatten the artifacts into this directory - there will be no differentiation between
    /// artifacts produced by different platforms.
    #[clap(long)]
    pub out_dir: Option<PathBuf>,

    /// Build the fullstack variant of this app, using that as the fileserver and backend
    ///
    /// This defaults to `false` but will be overridden to true if the `fullstack` feature is enabled.
    #[clap(long)]
    pub(crate) fullstack: bool,

    /// Run the ssg config of the app and generate the files
    #[clap(long)]
    pub(crate) ssg: bool,

    /// The arguments for the dioxus build
    #[clap(flatten)]
    pub(crate) args: BuildArgs,
}

impl Bundle {
    // todo: make sure to run pre-render static routes! we removed this from the other bundling step
    pub(crate) async fn bundle(mut self) -> Result<StructuredOutput> {
        tracing::info!("Bundling project...");

        // We always use `release` mode for bundling
        // todo - maybe not? what if you want a devmode bundle?
        self.args.release = true;

        let build = BuildRequest::new(&self.args)
            .await
            .context("Failed to load Dioxus workspace")?;

        tracing::info!("Building app...");

        let bundle = AppBuilder::start(&build)?.finish_build().await?;

        // If we're building for iOS, we need to bundle the iOS bundle
        if build.platform == Platform::Ios && self.package_types.is_none() {
            self.package_types = Some(vec![crate::PackageType::IosBundle]);
        }

        let mut bundles = vec![];

        // // Copy the server over if it exists
        // if build.fullstack {
        //     bundles.push(build.server_exe().unwrap());
        // }

        // Create a list of bundles that we might need to copy
        match build.platform {
            // By default, mac/win/linux work with tauri bundle
            Platform::MacOS | Platform::Linux | Platform::Windows => {
                tracing::info!("Running desktop bundler...");
                for bundle in Self::bundle_desktop(&build, &self.package_types)? {
                    bundles.extend(bundle.bundle_paths);
                }
            }

            // Web/ios can just use their root_dir
            Platform::Web => bundles.push(build.root_dir()),
            Platform::Ios => {
                tracing::warn!("iOS bundles are not currently codesigned! You will need to codesign the app before distributing.");
                bundles.push(build.root_dir())
            }
            Platform::Server => bundles.push(build.root_dir()),
            Platform::Liveview => bundles.push(build.root_dir()),

            Platform::Android => {
                let aab = build
                    .android_gradle_bundle()
                    .await
                    .context("Failed to run gradle bundleRelease")?;
                bundles.push(aab);
            }
        };

        // Copy the bundles to the output directory if one was specified
        let crate_outdir = build.crate_out_dir();
        if let Some(outdir) = self.out_dir.clone().or(crate_outdir) {
            let outdir = outdir
                .absolutize()
                .context("Failed to absolutize output directory")?;

            tracing::info!("Copying bundles to output directory: {}", outdir.display());

            std::fs::create_dir_all(&outdir)?;

            for bundle_path in bundles.iter_mut() {
                let destination = outdir.join(bundle_path.file_name().unwrap());

                tracing::debug!(
                    "Copying from {} to {}",
                    bundle_path.display(),
                    destination.display()
                );

                if bundle_path.is_dir() {
                    dircpy::CopyBuilder::new(&bundle_path, &destination)
                        .overwrite(true)
                        .run_par()
                        .context("Failed to copy the app to output directory")?;
                } else {
                    std::fs::copy(&bundle_path, &destination)
                        .context("Failed to copy the app to output directory")?;
                }

                *bundle_path = destination;
            }
        }

        for bundle_path in bundles.iter() {
            tracing::info!(
                "Bundled app at: {}",
                bundle_path.absolutize().unwrap().display()
            );
        }

        // async fn pre_render_ssg_routes(&self) -> Result<()> {
        //     // Run SSG and cache static routes
        //     if !self.ssg {
        //         return Ok(());
        //     }
        //     self.status_prerendering_routes();
        //     pre_render_static_routes(
        //         &self
        //             .server_exe()
        //             .context("Failed to find server executable")?,
        //     )
        //     .await?;
        //     Ok(())
        // }

        Ok(StructuredOutput::BundleOutput { bundles })
    }

    fn bundle_desktop(
        build: &BuildRequest,
        package_types: &Option<Vec<crate::PackageType>>,
    ) -> Result<Vec<tauri_bundler::Bundle>, Error> {
        let krate = &build;
        let exe = build.main_exe();

        _ = std::fs::remove_dir_all(krate.bundle_dir(build.platform));

        let package = krate.package();
        let mut name: PathBuf = krate.executable_name().into();
        if cfg!(windows) {
            name.set_extension("exe");
        }
        std::fs::create_dir_all(krate.bundle_dir(build.platform))
            .context("Failed to create bundle directory")?;
        std::fs::copy(&exe, krate.bundle_dir(build.platform).join(&name))
            .with_context(|| "Failed to copy the output executable into the bundle directory")?;

        let binaries = vec![
            // We use the name of the exe but it has to be in the same directory
            BundleBinary::new(krate.executable_name().to_string(), true)
                .set_src_path(Some(exe.display().to_string())),
        ];

        let mut bundle_settings: BundleSettings = krate.config.bundle.clone().into();

        // Check if required fields are provided instead of failing silently.
        if bundle_settings.identifier.is_none() {
            return Err(anyhow!("\n\nBundle identifier was not provided in `Dioxus.toml`. Add it as:\n\n[bundle]\nidentifier = \"com.mycompany\"\n\n").into());
        }
        if bundle_settings.publisher.is_none() {
            return Err(anyhow!("\n\nBundle publisher was not provided in `Dioxus.toml`. Add it as:\n\n[bundle]\npublisher = \"MyCompany\"\n\n").into());
        }

        if cfg!(windows) {
            let windows_icon_override = krate.config.bundle.windows.as_ref().map(|w| &w.icon_path);
            if windows_icon_override.is_none() {
                let icon_path = bundle_settings
                    .icon
                    .as_ref()
                    .and_then(|icons| icons.first());

                if let Some(icon_path) = icon_path {
                    bundle_settings.icon = Some(vec![icon_path.into()]);
                };
            }
        }

        if bundle_settings.resources_map.is_none() {
            bundle_settings.resources_map = Some(HashMap::new());
        }

        let asset_dir = build.asset_dir();
        if asset_dir.exists() {
            for entry in WalkDir::new(&asset_dir) {
                let entry = entry.unwrap();
                let path = entry.path();

                if path.is_file() {
                    let old = path
                        .canonicalize()
                        .with_context(|| format!("Failed to canonicalize {entry:?}"))?;
                    let new =
                        PathBuf::from("assets").join(path.strip_prefix(&asset_dir).unwrap_or(path));

                    tracing::debug!("Bundled asset: {old:?} -> {new:?}");
                    bundle_settings
                        .resources_map
                        .as_mut()
                        .expect("to be set")
                        .insert(old.display().to_string(), new.display().to_string());
                }
            }
        }

        for resource_path in bundle_settings.resources.take().into_iter().flatten() {
            bundle_settings
                .resources_map
                .as_mut()
                .expect("to be set")
                .insert(resource_path, "".to_string());
        }

        let mut settings = SettingsBuilder::new()
            .project_out_directory(krate.bundle_dir(build.platform))
            .package_settings(PackageSettings {
                product_name: krate.bundled_app_name(),
                version: package.version.to_string(),
                description: package.description.clone().unwrap_or_default(),
                homepage: Some(package.homepage.clone().unwrap_or_default()),
                authors: Some(package.authors.clone()),
                default_run: Some(name.display().to_string()),
            })
            .log_level(log::Level::Debug)
            .binaries(binaries)
            .bundle_settings(bundle_settings);

        if let Some(packages) = &package_types {
            settings = settings.package_types(packages.iter().map(|p| (*p).into()).collect());
        }

        settings = settings.target(build.triple.to_string());

        let settings = settings
            .build()
            .context("failed to bundle tauri bundle settings")?;
        tracing::debug!("Bundling project with settings: {:#?}", settings);
        if cfg!(target_os = "macos") {
            std::env::set_var("CI", "true");
        }

        let bundles = tauri_bundler::bundle::bundle_project(&settings).inspect_err(|err| {
            tracing::error!("Failed to bundle project: {:#?}", err);
            if cfg!(target_os = "macos") {
                tracing::error!("Make sure you have automation enabled in your terminal (https://github.com/tauri-apps/tauri/issues/3055#issuecomment-1624389208) and full disk access enabled for your terminal (https://github.com/tauri-apps/tauri/issues/3055#issuecomment-1624389208)");
            }
        })?;

        Ok(bundles)
    }

    async fn pre_render_static_routes(server_exe: &Path) -> anyhow::Result<()> {
        // Use the address passed in through environment variables or default to localhost:9999. We need
        // to default to a value that is different than the CLI default address to avoid conflicts
        let ip = server_ip().unwrap_or_else(|| IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)));
        let port = server_port().unwrap_or(9999);
        let fullstack_address = SocketAddr::new(ip, port);
        let address = fullstack_address.ip().to_string();
        let port = fullstack_address.port().to_string();
        // Borrow port and address so we can easily moe them into multiple tasks below
        let address = &address;
        let port = &port;

        tracing::info!("Running SSG at http://{address}:{port}");

        // Run the server executable
        let _child = Command::new(server_exe)
            .env(dioxus_cli_config::SERVER_PORT_ENV, port)
            .env(dioxus_cli_config::SERVER_IP_ENV, address)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true)
            .spawn()?;

        let reqwest_client = reqwest::Client::new();
        // Borrow reqwest_client so we only move the reference into the futures
        let reqwest_client = &reqwest_client;

        // Get the routes from the `/static_routes` endpoint
        let mut routes = None;

        // The server may take a few seconds to start up. Try fetching the route up to 5 times with a one second delay
        const RETRY_ATTEMPTS: usize = 5;
        for i in 0..=RETRY_ATTEMPTS {
            let request = reqwest_client
                .post(format!("http://{address}:{port}/api/static_routes"))
                .send()
                .await;
            match request {
                Ok(request) => {
                    routes = Some(request
                    .json::<Vec<String>>()
                    .await
                    .inspect(|text| tracing::debug!("Got static routes: {text:?}"))
                    .context("Failed to parse static routes from the server. Make sure your server function returns Vec<String> with the (default) json encoding")?);
                    break;
                }
                Err(err) => {
                    // If the request fails, try  up to 5 times with a one second delay
                    // If it fails 5 times, return the error
                    if i == RETRY_ATTEMPTS {
                        return Err(err).context("Failed to get static routes from server. Make sure you have a server function at the `/api/static_routes` endpoint that returns Vec<String> of static routes.");
                    }
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                }
            }
        }

        let routes = routes.expect(
            "static routes should exist or an error should have been returned on the last attempt",
        );

        // Create a pool of futures that cache each route
        let mut resolved_routes = routes
            .into_iter()
            .map(|route| async move {
                tracing::info!("Rendering {route} for SSG");
                // For each route, ping the server to force it to cache the response for ssg
                let request = reqwest_client
                    .get(format!("http://{address}:{port}{route}"))
                    .header("Accept", "text/html")
                    .send()
                    .await?;
                // If it takes longer than 30 seconds to resolve the route, log a warning
                let warning_task = tokio::spawn({
                    let route = route.clone();
                    async move {
                        tokio::time::sleep(Duration::from_secs(30)).await;
                        tracing::warn!("Route {route} has been rendering for 30 seconds");
                    }
                });
                // Wait for the streaming response to completely finish before continuing. We don't use the html it returns directly
                // because it may contain artifacts of intermediate streaming steps while the page is loading. The SSG app should write
                // the final clean HTML to the disk automatically after the request completes.
                let _html = request.text().await?;

                // Cancel the warning task if it hasn't already run
                warning_task.abort();

                Ok::<_, reqwest::Error>(route)
            })
            .collect::<FuturesUnordered<_>>();

        while let Some(route) = resolved_routes.next().await {
            match route {
                Ok(route) => tracing::debug!("ssg success: {route:?}"),
                Err(err) => tracing::error!("ssg error: {err:?}"),
            }
        }

        tracing::info!("SSG complete");

        drop(_child);

        Ok(())
    }
}
