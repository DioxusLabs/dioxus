// [package]
// name = "dx-cli-harness-simple-web"
// version = "0.0.1"
// edition = "2021"
// license = "MIT OR Apache-2.0"
// publish = false

// [dependencies]
// dioxus = { workspace = true, features = ["web"] }

// fn main() {
//     println!("Hello, world!");
// }

use anyhow::{bail, Result};
use clap::Parser;
use futures_util::{
    stream::{futures_unordered, FuturesUnordered},
    StreamExt,
};
use std::{path::PathBuf, pin::Pin, prelude::rust_2024::Future};
use target_lexicon::Triple;
use tokio::task::{JoinSet, LocalSet};
use tracing_subscriber::{
    prelude::__tracing_subscriber_SubscriberExt, util::SubscriberInitExt, EnvFilter, Layer,
};

use crate::{
    platform_override::CommandWithPlatformOverrides, workspace, BuildArgs, BuildTargets,
    BundleFormat, Cli, Commands, Workspace,
};

#[tokio::test]
async fn test_harnesses() {
    let env_filter = EnvFilter::new("error,dx=debug,dioxus_cli=debug,manganis_cli_support=debug,wasm_split_cli=debug,subsecond_cli_support=debug",);
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer().with_filter(env_filter))
        .init();

    run_harnesses(vec![
        TestHarnessBuilder::new()
            .args("dx build --package harness-simple-web")
            .deps(r#"dioxus = { workspace = true, features = ["web"] }"#)
            .asrt(|targets| async move {
                let targets = targets.unwrap();
                assert_eq!(targets.client.bundle, BundleFormat::Web);
                assert_eq!(
                    targets.client.triple,
                    "wasm32-unknown-unknown".parse().unwrap()
                );
                assert!(targets.server.is_none());
            }),
        TestHarnessBuilder::new()
            .args("dx build --package harness-simple-desktop")
            .deps(r#"dioxus = { workspace = true, features = ["desktop"] }"#)
            .asrt(|targets| async move {
                let targets = targets.unwrap();
                assert_eq!(targets.client.bundle, BundleFormat::host());
                assert_eq!(targets.client.triple, Triple::host());
                assert!(targets.server.is_none());
            }),
    ])
    .await;
}

#[derive(Default)]
struct TestHarnessBuilder {
    args: String,
    dependencies: String,
    features: String,

    server_dependencies: Option<String>,
    server_features: Option<String>,

    future: Option<Box<dyn FnOnce(Result<BuildTargets>) -> Pin<Box<dyn Future<Output = ()>>>>>,
}

impl TestHarnessBuilder {
    fn new() -> Self {
        Self {
            args: Default::default(),
            dependencies: Default::default(),
            features: Default::default(),
            future: Default::default(),
            server_dependencies: Default::default(),
            server_features: Default::default(),
        }
    }
    fn deps(mut self, dependencies: impl Into<String>) -> Self {
        self.dependencies = dependencies.into();
        self
    }
    fn features(mut self, features: impl Into<String>) -> Self {
        self.features = features.into();
        self
    }

    fn args(mut self, args: impl Into<String>) -> Self {
        self.args = args.into();
        self
    }

    fn asrt<F>(mut self, future: impl FnOnce(Result<BuildTargets>) -> F + 'static) -> Self
    where
        F: Future<Output = ()> + 'static,
    {
        self.future = Some(Box::new(move |args| Box::pin(future(args))));
        self
    }

    fn build(mut self) -> CommandWithPlatformOverrides<BuildArgs> {
        let args = self.args;
        let dependencies = self.dependencies;
        let features = self.features;
        let escaped = shell_words::split(&args).unwrap();

        let package_arg = escaped
            .iter()
            .position(|s| s.starts_with("--package"))
            .unwrap();
        let name = escaped[package_arg + 1].to_string();

        let cargo_manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
            .map(PathBuf::from)
            .unwrap();

        let harness_dir = cargo_manifest_dir.parent().unwrap().join("cli-harnesses");

        // make sure we don't start deleting random stuff.
        if !harness_dir.exists() {
            panic!(
                "cli-harnesses directory does not exist, aborting: {:?}",
                harness_dir
            );
        }

        let test_dir = harness_dir.join(&name);

        _ = std::fs::remove_dir_all(&test_dir);
        std::fs::create_dir_all(&test_dir).unwrap();
        std::fs::create_dir_all(test_dir.join("src")).unwrap();

        let cargo_toml = format!(
            r#"[package]
name = "{name}"
version = "0.0.1"
edition = "2021"
license = "MIT OR Apache-2.0"
publish = false

[dependencies]
{dependencies}

[features]
{features}
    "#,
            name = name,
            dependencies = dependencies,
            features = features
        );

        std::fs::write(test_dir.join("Cargo.toml"), cargo_toml).unwrap();
        std::fs::write(
            test_dir.join("src/main.rs"),
            r#"fn main() { println!("Hello, world!"); }"#,
        )
        .unwrap();

        let args = Cli::try_parse_from(escaped).unwrap();
        let Commands::Build(build_args) = args.action else {
            panic!("Expected build command");
        };

        build_args
    }
}

async fn run_harnesses(harnesses: Vec<TestHarnessBuilder>) {
    _ = crate::VERBOSITY.set(crate::Verbosity {
        verbose: true,
        trace: true,
        json_output: false,
        log_to_file: None,
        locked: false,
        offline: false,
        frozen: false,
    });

    //     let harnesses =

    // Now that the harnesses are written to the filesystem, we can call cargo_metadata
    // It will be cached from here
    let workspace = Workspace::current();

    // let mut res = FuturesUnordered::from_iter(harnesses.into_iter().map(|harness| harness.assert));
    // while let Some(res) = res.next().await {}
}
