use futures_channel::mpsc::UnboundedSender;
use toml_edit::Item;

use crate::builder::Build;
use crate::dioxus_crate::DioxusCrate;

use crate::builder::BuildRequest;
use std::io::Write;

use super::{BuildReason, TargetPlatform, UpdateBuildProgress};

static CLIENT_PROFILE: &str = "dioxus-client";
static SERVER_PROFILE: &str = "dioxus-server";

impl BuildRequest {
    pub(crate) fn new_single(
        reason: BuildReason,
        krate: DioxusCrate,
        arguments: Build,
        progress: UnboundedSender<UpdateBuildProgress>,
        platform: TargetPlatform,
    ) -> Self {
        Self {
            progress,
            reason,
            krate,
            target_platform: platform,
            build_arguments: arguments,
            target_dir: Default::default(),
            rust_flags: Default::default(),
            executable: Default::default(),
            assets: Default::default(),
            child: None,
        }
    }

    pub(crate) fn new_fullstack(
        config: DioxusCrate,
        build_arguments: Build,
        serve: BuildReason,
        progress: UnboundedSender<UpdateBuildProgress>,
    ) -> Result<Vec<Self>, crate::Error> {
        initialize_profiles(&config)?;

        Ok(vec![
            Self::new_client(serve, &config, build_arguments.clone(), progress.clone()),
            Self::new_server(serve, &config, build_arguments, progress),
        ])
    }

    fn new_with_target_directory_rust_flags_and_features(
        serve: BuildReason,
        config: &DioxusCrate,
        build: &Build,
        feature: Option<String>,
        target_platform: TargetPlatform,
        progress: UnboundedSender<UpdateBuildProgress>,
    ) -> Self {
        let config = config.clone();
        let mut build = build.clone();

        // Add the server feature to the features we pass to the build
        if let Some(feature) = feature {
            build.target_args.features.push(feature);
        }

        // Add the server flags to the build arguments
        Self {
            reason: serve,
            build_arguments: build.clone(),
            krate: config,
            rust_flags: Default::default(),
            target_dir: None,
            target_platform,
            executable: None,
            assets: Default::default(),
            progress,
            child: None,
        }
    }

    fn new_server(
        serve: BuildReason,
        config: &DioxusCrate,
        mut build: Build,
        progress: UnboundedSender<UpdateBuildProgress>,
    ) -> Self {
        if build.profile.is_none() {
            build.profile = Some(CLIENT_PROFILE.to_string());
        }
        let client_feature = build.auto_detect_server_feature(config);
        Self::new_with_target_directory_rust_flags_and_features(
            serve,
            config,
            &build,
            build.target_args.server_feature.clone().or(client_feature),
            TargetPlatform::Server,
            progress,
        )
    }

    fn new_client(
        serve: BuildReason,
        config: &DioxusCrate,
        mut build: Build,
        progress: UnboundedSender<UpdateBuildProgress>,
    ) -> Self {
        if build.profile.is_none() {
            build.profile = Some(SERVER_PROFILE.to_string());
        }
        let (client_feature, client_platform) = build.auto_detect_client_platform(config);
        Self::new_with_target_directory_rust_flags_and_features(
            serve,
            config,
            &build,
            build.target_args.client_feature.clone().or(client_feature),
            client_platform,
            progress,
        )
    }
}

// The `opt-level=2` increases build times, but can noticeably decrease time
// between saving changes and being able to interact with an app. The "overall"
// time difference (between having and not having the optimization) can be
// almost imperceptible (~1 s) but also can be very noticeable (~6 s) — depends
// on setup (hardware, OS, browser, idle load).
// Find or create the client and server profiles in the .cargo/config.toml file
fn initialize_profiles(config: &DioxusCrate) -> crate::Result<()> {
    let config_path = config.workspace_dir().join(".cargo/config.toml");
    let mut config = match std::fs::read_to_string(&config_path) {
        Ok(config) => config.parse::<toml_edit::DocumentMut>().map_err(|e| {
            crate::Error::Other(anyhow::anyhow!("Failed to parse .cargo/config.toml: {}", e))
        })?,
        Err(_) => Default::default(),
    };

    if let Item::Table(table) = config
        .as_table_mut()
        .entry("profile")
        .or_insert(Item::Table(Default::default()))
    {
        if let toml_edit::Entry::Vacant(entry) = table.entry(CLIENT_PROFILE) {
            let mut client = toml_edit::Table::new();
            client.insert("inherits", Item::Value("dev".into()));
            client.insert("opt-level", Item::Value(2.into()));
            entry.insert(Item::Table(client));
        }

        if let toml_edit::Entry::Vacant(entry) = table.entry(SERVER_PROFILE) {
            let mut server = toml_edit::Table::new();
            server.insert("inherits", Item::Value("dev".into()));
            server.insert("opt-level", Item::Value(2.into()));
            entry.insert(Item::Table(server));
        }
    }

    // Write the config back to the file
    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let file = std::fs::File::create(config_path)?;
    let mut buf_writer = std::io::BufWriter::new(file);
    write!(buf_writer, "{}", config)?;

    Ok(())
}
