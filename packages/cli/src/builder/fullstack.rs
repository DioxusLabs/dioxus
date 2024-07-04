use crate::builder::Build;
use crate::builder::BuildResult;
use dioxus_cli_config::CrateConfig;

use crate::{build, Result};

use super::Platform;
use crate::builder::BuildRequest;
use crate::serve::Serve;
use cargo_metadata::diagnostic::Diagnostic;
use dioxus_cli_config::ServeArguments;
use manganis_cli_support::AssetManifest;
use std::{path::PathBuf, time::Duration};
use tokio::process::Child;

static CLIENT_RUST_FLAGS: &str = "-C debuginfo=none -C strip=debuginfo";
// The `opt-level=2` increases build times, but can noticeably decrease time
// between saving changes and being able to interact with an app. The "overall"
// time difference (between having and not having the optimization) can be
// almost imperceptible (~1 s) but also can be very noticeable (~6 s) — depends
// on setup (hardware, OS, browser, idle load).
static SERVER_RUST_FLAGS: &str = "-C opt-level=2";
static DEBUG_RUST_FLAG: &str = "-C debug-assertions";

fn add_debug_rust_flags(build: &Build, flags: &mut String) {
    if !build.release {
        *flags += " ";
        *flags += DEBUG_RUST_FLAG;
    }
}

fn fullstack_rust_flags(build: &Build, base_flags: &str) -> String {
    // If we are forcing debug mode, don't add any debug flags
    if build.force_debug {
        return Default::default();
    }

    let mut rust_flags = base_flags.to_string();
    add_debug_rust_flags(build, &mut rust_flags);
    rust_flags
}

// Fullstack builds run the server and client builds parallel by default
// To make them run in parallel, we need to set up different target directories for the server and client within /.dioxus
fn set_target_directory(build: &Build, config: &mut CrateConfig, target: PathBuf) {
    if !build.force_sequential {
        config.target_dir = target;
    }
}

impl BuildRequest {
    pub(crate) fn new_fullstack(
        config: CrateConfig,
        build_arguments: Build,
        serve: bool,
    ) -> Vec<Self> {
        vec![
            Self::new_server(serve, &config, &build_arguments),
            Self::new_client(serve, &config, &build_arguments),
        ]
    }

    fn new_with_target_directory_rust_flags_and_features(
        serve: bool,
        config: &CrateConfig,
        build: &Build,
        target_directory: PathBuf,
        rust_flags: &str,
        feature: String,
        web: bool,
    ) -> Self {
        let mut config = config.clone();
        // Set the target directory we are building the server in
        set_target_directory(build, &mut config, target_directory);
        // Add the server feature to the features we pass to the build
        config.features.push(feature);

        // Add the server flags to the build arguments
        let rust_flags = fullstack_rust_flags(build, rust_flags);

        Self {
            web,
            serve,
            build_arguments: build.clone(),
            config,
            rust_flags: Some(rust_flags),
        }
    }

    fn new_server(serve: bool, config: &CrateConfig, build: &Build) -> Self {
        Self::new_with_target_directory_rust_flags_and_features(
            serve,
            config,
            build,
            config.server_target_dir(),
            SERVER_RUST_FLAGS,
            build.server_feature.clone(),
            false,
        )
    }

    fn new_client(serve: bool, config: &CrateConfig, build: &Build) -> Self {
        Self::new_with_target_directory_rust_flags_and_features(
            serve,
            config,
            build,
            config.client_target_dir(),
            CLIENT_RUST_FLAGS,
            build.client_feature.clone(),
            true,
        )
    }

    // When building the fullstack server, we need to forward the serve arguments (like port) to the fullstack server through env vars
    fn add_serve_options_to_env(serve: &Serve, env: &mut Vec<(String, String)>) {
        env.push((
            dioxus_cli_config::__private::SERVE_ENV.to_string(),
            serde_json::to_string(&serve.server_arguments).unwrap(),
        ));
    }
}
