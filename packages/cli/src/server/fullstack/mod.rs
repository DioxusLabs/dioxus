use dioxus_cli_config::CrateConfig;

use crate::{
    cfg::{ConfigOptsBuild, ConfigOptsServe},
    BuildResult, Result,
};

use super::{
    desktop::{self, DesktopPlatform},
    Platform,
};

static CLIENT_RUST_FLAGS: &str = "-C debuginfo=none -C strip=debuginfo";
// The `opt-level=2` increases build times, but can noticeably decrease time
// between saving changes and being able to interact with an app. The "overall"
// time difference (between having and not having the optimization) can be
// almost imperceptible (~1 s) but also can be very noticeable (~6 s) — depends
// on setup (hardware, OS, browser, idle load).
static SERVER_RUST_FLAGS: &str = "-C opt-level=2";
static DEBUG_RUST_FLAG: &str = "-C debug-assertions";

fn rust_flags(build: &ConfigOptsBuild, base_flags: &str) -> String {
    let mut rust_flags = base_flags.to_string();
    if !build.release {
        rust_flags += " ";
        rust_flags += DEBUG_RUST_FLAG;
    };
    rust_flags
}

pub fn client_rust_flags(build: &ConfigOptsBuild) -> String {
    rust_flags(build, CLIENT_RUST_FLAGS)
}

pub fn server_rust_flags(build: &ConfigOptsBuild) -> String {
    rust_flags(build, SERVER_RUST_FLAGS)
}

pub async fn startup(config: CrateConfig, serve: &ConfigOptsServe) -> Result<()> {
    desktop::startup_with_platform::<FullstackPlatform>(config, serve).await
}

fn start_web_build_thread(
    config: &CrateConfig,
    serve: &ConfigOptsServe,
) -> std::thread::JoinHandle<Result<()>> {
    let serve = serve.clone();
    let target_directory = config.client_target_dir();
    std::fs::create_dir_all(&target_directory).unwrap();
    std::thread::spawn(move || futures::executor::block_on(build_web(serve, &target_directory)))
}

fn make_desktop_config(config: &CrateConfig, serve: &ConfigOptsServe) -> CrateConfig {
    let mut desktop_config = config.clone();
    if !serve.force_sequential {
        desktop_config.target_dir = config.server_target_dir();
    }
    let desktop_feature = serve.server_feature.clone();
    let features = &mut desktop_config.features;
    match features {
        Some(features) => {
            features.push(desktop_feature);
        }
        None => desktop_config.features = Some(vec![desktop_feature]),
    };
    desktop_config
}

fn add_serve_options_to_env(serve: &ConfigOptsServe, env: &mut Vec<(String, String)>) {
    env.push((
        dioxus_cli_config::__private::SERVE_ENV.to_string(),
        serde_json::to_string(&serve.server_arguments).unwrap(),
    ));
}

struct FullstackPlatform {
    serve: ConfigOptsServe,
    desktop: desktop::DesktopPlatform,
    server_rust_flags: String,
}

impl Platform for FullstackPlatform {
    fn start(
        config: &CrateConfig,
        serve: &ConfigOptsServe,
        env: Vec<(String, String)>,
    ) -> Result<Self>
    where
        Self: Sized,
    {
        let thread_handle = start_web_build_thread(config, serve);

        let desktop_config = make_desktop_config(config, serve);
        let server_rust_flags = server_rust_flags(&serve.clone().into());
        let mut desktop_env = env.clone();
        add_serve_options_to_env(serve, &mut desktop_env);
        let build_result = crate::builder::build_desktop(
            &desktop_config,
            true,
            serve.skip_assets,
            Some(server_rust_flags.clone()),
        )?;
        thread_handle
            .join()
            .map_err(|_| anyhow::anyhow!("Failed to join thread"))??;

        // Only start the server after the web build is finished
        let desktop =
            DesktopPlatform::start_with_options(build_result, &desktop_config, serve, desktop_env)?;

        if serve.open {
            crate::server::web::open_browser(
                config,
                serve
                    .server_arguments
                    .addr
                    .unwrap_or(std::net::IpAddr::V4(std::net::Ipv4Addr::new(0, 0, 0, 0))),
                serve.server_arguments.port,
                false,
            );
        }

        Ok(Self {
            desktop,
            serve: serve.clone(),
            server_rust_flags,
        })
    }

    fn rebuild(
        &mut self,
        crate_config: &CrateConfig,
        serve: &ConfigOptsServe,
        env: Vec<(String, String)>,
    ) -> Result<BuildResult> {
        let thread_handle = start_web_build_thread(crate_config, &self.serve);
        let desktop_config = make_desktop_config(crate_config, &self.serve);
        let mut desktop_env = env.clone();
        add_serve_options_to_env(serve, &mut desktop_env);
        let result = self.desktop.rebuild_with_options(
            &desktop_config,
            Some(self.server_rust_flags.clone()),
            desktop_env,
        );
        thread_handle
            .join()
            .map_err(|_| anyhow::anyhow!("Failed to join thread"))??;
        result
    }
}

async fn build_web(serve: ConfigOptsServe, target_directory: &std::path::Path) -> Result<()> {
    let mut web_config: ConfigOptsBuild = serve.into();
    let web_feature = web_config.client_feature.clone();
    let features = &mut web_config.features;
    match features {
        Some(features) => {
            features.push(web_feature);
        }
        None => web_config.features = Some(vec![web_feature]),
    };
    web_config.platform = Some(dioxus_cli_config::Platform::Web);

    crate::cli::build::Build {
        build: web_config.clone(),
    }
    .build(
        None,
        (!web_config.force_sequential).then_some(target_directory),
        Some(client_rust_flags(&web_config)),
    )
    .await
}
