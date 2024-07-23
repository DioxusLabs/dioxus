use crate::builder::Build;
use crate::dioxus_crate::DioxusCrate;
use dioxus_cli_config::Platform;

use crate::builder::BuildRequest;
use std::path::PathBuf;

static CLIENT_RUST_FLAGS: &[&str] = &["-Cdebuginfo=none", "-Cstrip=debuginfo"];
// The `opt-level=2` increases build times, but can noticeably decrease time
// between saving changes and being able to interact with an app. The "overall"
// time difference (between having and not having the optimization) can be
// almost imperceptible (~1 s) but also can be very noticeable (~6 s) — depends
// on setup (hardware, OS, browser, idle load).
static SERVER_RUST_FLAGS: &[&str] = &["-O"];
static DEBUG_RUST_FLAG: &str = "-Cdebug-assertions";

fn add_debug_rust_flags(build: &Build, flags: &mut Vec<String>) {
    if !build.release {
        flags.push(DEBUG_RUST_FLAG.to_string());
    }
}

fn fullstack_rust_flags(build: &Build, base_flags: &[&str]) -> Vec<String> {
    // If we are forcing debug mode, don't add any debug flags
    if build.force_debug {
        return Default::default();
    }

    let mut rust_flags = base_flags.iter().map(ToString::to_string).collect();
    add_debug_rust_flags(build, &mut rust_flags);
    rust_flags
}

// Fullstack builds run the server and client builds parallel by default
// To make them run in parallel, we need to set up different target directories for the server and client within /.dioxus
fn get_target_directory(build: &Build, target: PathBuf) -> Option<PathBuf> {
    (!build.force_sequential).then_some(target)
}

impl BuildRequest {
    pub(crate) fn new_fullstack(
        config: DioxusCrate,
        build_arguments: Build,
        serve: bool,
    ) -> Vec<Self> {
        vec![
            Self::new_client(serve, &config, &build_arguments),
            Self::new_server(serve, &config, &build_arguments),
        ]
    }

    fn new_with_target_directory_rust_flags_and_features(
        serve: bool,
        config: &DioxusCrate,
        build: &Build,
        target_directory: PathBuf,
        rust_flags: &[&str],
        feature: String,
        web: bool,
    ) -> Self {
        let config = config.clone();
        let mut build = build.clone();
        build.platform = Some(if web {
            Platform::Web
        } else {
            Platform::Desktop
        });
        // Set the target directory we are building the server in
        let target_dir = get_target_directory(&build, target_directory);
        // Add the server feature to the features we pass to the build
        build.target_args.features.push(feature);

        // Add the server flags to the build arguments
        let rust_flags = fullstack_rust_flags(&build, rust_flags);

        Self {
            web,
            serve,
            build_arguments: build.clone(),
            dioxus_crate: config,
            rust_flags,
            target_dir,
        }
    }

    fn new_server(serve: bool, config: &DioxusCrate, build: &Build) -> Self {
        Self::new_with_target_directory_rust_flags_and_features(
            serve,
            config,
            build,
            config.server_target_dir(),
            SERVER_RUST_FLAGS,
            build.target_args.server_feature.clone(),
            false,
        )
    }

    fn new_client(serve: bool, config: &DioxusCrate, build: &Build) -> Self {
        Self::new_with_target_directory_rust_flags_and_features(
            serve,
            config,
            build,
            config.client_target_dir(),
            CLIENT_RUST_FLAGS,
            build.target_args.client_feature.clone(),
            true,
        )
    }
}
