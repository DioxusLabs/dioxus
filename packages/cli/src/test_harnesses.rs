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
use std::{fmt::Write, path::PathBuf, pin::Pin, prelude::rust_2024::Future};
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
async fn run_harness() {
    test_harnesses().await;
}

#[allow(dead_code)]
pub async fn test_harnesses_used() {
    test_harnesses().await;
}

async fn test_harnesses() {
    let env_filter = EnvFilter::new("error,dx=debug,dioxus_cli=debug,manganis_cli_support=debug,wasm_split_cli=debug,subsecond_cli_support=debug",);
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer().with_filter(env_filter))
        .init();

    run_harnesses(vec![
        TestHarnessBuilder::new("harness-simple-web")
            .deps(r#"dioxus = { workspace = true, features = ["web"] }"#)
            .asrt(r#"dx build"#, |targets| async move {
                let targets = targets.unwrap();
                assert_eq!(targets.client.bundle, BundleFormat::Web);
                assert_eq!(
                    targets.client.triple,
                    "wasm32-unknown-unknown".parse().unwrap()
                );
                assert!(targets.server.is_none());
            }),
        TestHarnessBuilder::new("harness-simple-desktop")
            .deps(r#"dioxus = { workspace = true, features = ["desktop"] }"#)
            .asrt(r#"dx build"#, |targets| async move {
                let targets = targets.unwrap();
                assert_eq!(targets.client.bundle, BundleFormat::host());
                assert_eq!(targets.client.triple, Triple::host());
                assert!(targets.server.is_none());
            }),
        TestHarnessBuilder::new("harness-simple-web")
            .deps(r#"dioxus = { workspace = true, features = ["mobile"] }"#)
            .asrt(
                "dx build",
                |targets| async move { assert!(targets.is_err()) },
            ),
        TestHarnessBuilder::new("harness-simple-fullstack")
            .deps(r#"dioxus = { workspace = true, features = ["fullstack"] }"#)
            .fetr(r#"web=["dioxus/web"]"#)
            .fetr(r#"server=["dioxus/server"]"#)
            .asrt(r#"dx build"#, |targets| async move {
                let targets = targets.unwrap();
                assert_eq!(targets.client.bundle, BundleFormat::Web);
                let server = targets.server.unwrap();
                assert_eq!(server.bundle, BundleFormat::Server);
                assert_eq!(server.triple, Triple::host());
            }),
        TestHarnessBuilder::new("harness-fullstack-multi-target")
            .deps(r#"dioxus = { workspace = true, features = ["fullstack"] }"#)
            .fetr(r#"default=["web", "desktop", "mobile", "server"]"#)
            .fetr(r#"web=["dioxus/web"]"#)
            .fetr(r#"desktop=["dioxus/desktop"]"#)
            .fetr(r#"server=["dioxus/server"]"#)
            .asrt(
                r#"dx build"#,
                |targets| async move { assert!(targets.is_err()) },
            )
            .asrt(r#"dx build --web"#, |targets| async move {
                let targets = targets.unwrap();
                assert_eq!(targets.client.bundle, BundleFormat::Web);
            })
            .asrt(r#"dx build --desktop"#, |targets| async move {
                let targets = targets.unwrap();
                assert_eq!(targets.client.bundle, BundleFormat::host());
            })
            .asrt(r#"dx build --ios"#, |targets| async move {
                let targets = targets.unwrap();
                assert_eq!(targets.client.bundle, BundleFormat::Ios);
                assert_eq!(
                    targets.client.triple,
                    "aarch64-apple-ios-sim".parse().unwrap()
                );
            })
            .asrt(r#"dx build --ios --device"#, |targets| async move {
                let targets = targets.unwrap();
                assert_eq!(targets.client.bundle, BundleFormat::Ios);
                assert_eq!(targets.client.triple, "aarch64-apple-ios".parse().unwrap());
            }),
    ])
    .await;
}

#[derive(Default)]
struct TestHarnessBuilder {
    name: String,
    dependencies: String,
    features: String,

    server_dependencies: Option<String>,
    server_features: Option<String>,

    futures: Vec<TestHarnessTestCase>,
}

struct TestHarnessTestCase {
    args: String,
    callback: Box<dyn FnOnce(Result<BuildTargets>) -> Pin<Box<dyn Future<Output = ()>>>>,
}

impl TestHarnessBuilder {
    fn new(name: &str) -> Self {
        Self {
            name: name.into(),
            dependencies: Default::default(),
            features: Default::default(),
            futures: Default::default(),
            server_dependencies: Default::default(),
            server_features: Default::default(),
        }
    }
    fn deps(mut self, dependencies: impl Into<String>) -> Self {
        writeln!(&mut self.dependencies, "{}", dependencies.into()).unwrap();
        self
    }
    fn fetr(mut self, features: impl Into<String>) -> Self {
        writeln!(&mut self.features, "{}", features.into()).unwrap();
        self
    }

    fn asrt<F>(
        mut self,
        args: impl Into<String>,
        future: impl FnOnce(Result<BuildTargets>) -> F + 'static,
    ) -> Self
    where
        F: Future<Output = ()> + 'static,
    {
        self.futures.push(TestHarnessTestCase {
            args: args.into(),
            callback: Box::new(move |args| Box::pin(future(args))),
        });
        self
    }

    fn build(&self) {
        let name = self.name.clone();
        let dependencies = self.dependencies.clone();
        let features = self.features.clone();

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

    // Now that the harnesses are written to the filesystem, we can call cargo_metadata
    // It will be cached from here
    let workspace = Workspace::current().await.unwrap();
    let mut futures = FuturesUnordered::new();

    for harness in harnesses {
        _ = harness.build();
        for case in harness.futures {
            let escaped = shell_words::split(&case.args).unwrap();
            let args = Cli::try_parse_from(escaped).unwrap();
            let Commands::Build(mut build_args) = args.action else {
                panic!("Expected build command");
            };

            build_args.shared.build_arguments.package = Some(harness.name.clone());

            futures.push(async move {
                let targets = build_args.into_targets().await;
                (case.callback)(targets).await;
            });
        }
    }

    while let Some(res) = futures.next().await {}
}
