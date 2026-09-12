use crate::{BuildTargets, BundleFormat, Cli, Commands, Workspace};
use anyhow::Result;
use clap::Parser;
use futures_util::{StreamExt, stream::FuturesUnordered};
use std::{
    collections::HashSet,
    fmt::Write,
    path::{Path, PathBuf},
    pin::Pin,
    prelude::rust_2024::Future,
};
use target_lexicon::Triple;
use tracing_subscriber::{EnvFilter, Layer, prelude::*, util::SubscriberInitExt};

#[tokio::test]
async fn run_harness() {
    test_harnesses().await;
}

#[allow(dead_code)]
async fn test_harnesses() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer().with_filter(EnvFilter::new("error,dx=debug,dioxus_cli=debug,manganis_cli_support=debug,wasm_split_cli=debug,subsecond_cli_support=debug",)))
        .init();

    TestHarnessBuilder::run(vec![
        TestHarnessBuilder::new("harness-simple-web")
            .deps(r#"dioxus = { workspace = true, features = ["web"] }"#)
            .asrt(r#"dx build"#, |targets| async move {
                let t = targets.unwrap();
                assert_eq!(t.client.bundle, BundleFormat::Web);
                assert_eq!(t.client.triple, "wasm32-unknown-unknown".parse().unwrap());
                assert!(t.server.is_none());
            }),
        TestHarnessBuilder::new("harness-simple-desktop")
            .deps(r#"dioxus = { workspace = true, features = ["desktop"] }"#)
            .asrt(r#"dx build"#, |targets| async move {
                let t = targets.unwrap();
                assert_eq!(t.client.bundle, BundleFormat::host());
                assert_eq!(t.client.triple, Triple::host());
                assert!(t.server.is_none());
            }),
        TestHarnessBuilder::new("harness-simple-mobile")
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
                let t = targets.unwrap();
                assert_eq!(t.client.bundle, BundleFormat::Web);
                let server = t.server.unwrap();
                assert_eq!(server.bundle, BundleFormat::Server);
                assert_eq!(server.triple, Triple::host());
            }),
        TestHarnessBuilder::new("harness-simple-fullstack-with-default")
            .deps(r#"dioxus = { workspace = true, features = ["fullstack"] }"#)
            .fetr(r#"default=["web", "server"]"#)
            .fetr(r#"web=["dioxus/web"]"#)
            .fetr(r#"server=["dioxus/server"]"#)
            .asrt(r#"dx build"#, |targets| async move {
                let t = targets.unwrap();
                assert_eq!(t.client.bundle, BundleFormat::Web);
                let server = t.server.unwrap();
                assert_eq!(server.bundle, BundleFormat::Server);
                assert_eq!(server.triple, Triple::host());
            }),
        TestHarnessBuilder::new("harness-simple-fullstack-native-with-default")
            .deps(r#"dioxus = { workspace = true, features = ["fullstack"] }"#)
            .fetr(r#"default=["native", "server"]"#)
            .fetr(r#"native=["dioxus/native"]"#)
            .fetr(r#"server=["dioxus/server"]"#)
            .asrt(r#"dx build"#, |targets| async move {
                let t = targets.unwrap();
                assert_eq!(t.client.bundle, BundleFormat::host());
                assert_eq!(t.client.features.len(), 1);
                assert_eq!(t.client.features[0], "native");
                let server = t.server.unwrap();
                assert_eq!(server.bundle, BundleFormat::Server);
                assert_eq!(server.triple, Triple::host());
            }),
        TestHarnessBuilder::new("harness-fullstack-multi-target")
            .deps(r#"dioxus = { workspace = true, features = ["fullstack"] }"#)
            .fetr(r#"default=["web", "desktop", "mobile", "server"]"#)
            .fetr(r#"web=["dioxus/web"]"#)
            .fetr(r#"desktop=["dioxus/desktop"]"#)
            .fetr(r#"mobile=["dioxus/mobile"]"#)
            .fetr(r#"server=["dioxus/server"]"#)
            .asrt(r#"dx build"#, |t| async move { assert!(t.is_err()) })
            .asrt(r#"dx build --web"#, |targets| async move {
                let t = targets.unwrap();
                assert_eq!(t.client.bundle, BundleFormat::Web);
            })
            .asrt(r#"dx build --desktop"#, |targets| async move {
                let t = targets.unwrap();
                assert_eq!(t.client.bundle, BundleFormat::host());
            })
            .asrt(r#"dx build --ios"#, |targets| async move {
                if cfg!(not(target_os = "macos")) {
                    // iOS builds require Xcode toolchain, only available on macOS
                    return;
                }
                let t = targets.unwrap();
                assert_eq!(t.client.bundle, BundleFormat::Ios);
                assert_eq!(t.client.triple, TestHarnessBuilder::host_ios_triple_sim());
            })
            .asrt(r#"dx build --ios --device"#, |targets| async move {
                if cfg!(not(target_os = "macos")) {
                    return;
                }
                let targets = targets.unwrap();
                assert_eq!(targets.client.bundle, BundleFormat::Ios);
                assert_eq!(targets.client.triple, "aarch64-apple-ios".parse().unwrap());
            })
            .asrt(r#"dx build --android --device"#, |targets| async move {
                if crate::AndroidTools::current().is_none() {
                    // Skip when Android NDK/SDK is not installed
                    return;
                }
                let t = targets.unwrap();
                assert_eq!(t.client.bundle, BundleFormat::Android);
                assert_eq!(t.client.triple, "aarch64-linux-android".parse().unwrap());
            }),
        TestHarnessBuilder::new("harness-fullstack-multi-target-no-default")
            .deps(r#"dioxus = { workspace = true, features = ["fullstack"] }"#)
            .fetr(r#"web=["dioxus/web"]"#)
            .fetr(r#"desktop=["dioxus/desktop"]"#)
            .fetr(r#"mobile=["dioxus/mobile"]"#)
            .fetr(r#"server=["dioxus/server"]"#)
            .asrt(r#"dx build"#, |targets| async move {
                assert!(targets.is_err())
            })
            .asrt(r#"dx build --desktop"#, |targets| async move {
                let t = targets.unwrap();
                assert_eq!(t.client.bundle, BundleFormat::host());
                let server = t.server.unwrap();
                assert_eq!(server.bundle, BundleFormat::Server);
                assert_eq!(server.triple, Triple::host());
            })
            .asrt(r#"dx build --ios"#, |targets| async move {
                if cfg!(not(target_os = "macos")) {
                    // iOS builds require Xcode toolchain, only available on macOS
                    return;
                }
                let t = targets.unwrap();
                assert_eq!(t.client.bundle, BundleFormat::Ios);
                assert_eq!(t.client.triple, TestHarnessBuilder::host_ios_triple_sim());
                let server = t.server.unwrap();
                assert_eq!(server.bundle, BundleFormat::Server);
                assert_eq!(server.triple, Triple::host());
            }),
        TestHarnessBuilder::new("harness-fullstack-desktop")
            .deps(r#"dioxus = { workspace = true, features = ["fullstack"] }"#)
            .fetr(r#"desktop=["dioxus/desktop"]"#)
            .fetr(r#"server=["dioxus/server"]"#)
            .asrt(r#"dx build"#, |targets| async move {
                let t = targets.unwrap();
                assert_eq!(t.client.bundle, BundleFormat::host());
                let server = t.server.unwrap();
                assert_eq!(server.bundle, BundleFormat::Server);
                assert_eq!(server.triple, Triple::host());
            }),
        TestHarnessBuilder::new("harness-fullstack-desktop-with-features")
            .deps(r#"dioxus = { workspace = true, features = ["fullstack"] }"#)
            .deps(r#"anyhow = { workspace = true, optional = true }"#)
            .fetr(r#"desktop=["dioxus/desktop", "has_anyhow"]"#)
            .fetr(r#"has_anyhow=["dep:anyhow"]"#)
            .fetr(r#"server=["dioxus/server"]"#)
            .asrt(r#"dx build"#, |targets| async move {
                let t = targets.unwrap();
                assert_eq!(t.client.bundle, BundleFormat::host());
                let server = t.server.unwrap();
                assert_eq!(server.bundle, BundleFormat::Server);
                assert_eq!(server.triple, Triple::host());
            }),
        TestHarnessBuilder::new("harness-fullstack-desktop-with-default")
            .deps(r#"dioxus = { workspace = true, features = ["fullstack"] }"#)
            .deps(r#"anyhow = { workspace = true, optional = true }"#)
            .fetr(r#"default=["desktop"]"#)
            .fetr(r#"desktop=["dioxus/desktop", "has_anyhow"]"#)
            .fetr(r#"has_anyhow=["dep:anyhow"]"#)
            .fetr(r#"server=["dioxus/server"]"#)
            .asrt(r#"dx build"#, |targets| async move {
                let t = targets.unwrap();
                assert_eq!(t.client.bundle, BundleFormat::host());
                let server = t.server.unwrap();
                assert_eq!(server.bundle, BundleFormat::Server);
                assert_eq!(server.triple, Triple::host());
            }),
        TestHarnessBuilder::new("harness-no-dioxus")
            .deps(r#"anyhow = { workspace = true, optional = true }"#)
            .fetr(r#"web=["dep:anyhow"]"#)
            .fetr(r#"server=[]"#)
            .asrt(r#"dx build"#, |targets| async move {
                let t = targets.unwrap();
                assert_eq!(t.client.bundle, BundleFormat::host());
                assert!(t.server.is_none());
            }),
        TestHarnessBuilder::new("harness-simple-dedicated-server"),
        TestHarnessBuilder::new("harness-simple-dedicated-client")
            .deps(r#"dioxus = { workspace = true, features = ["web"] }"#)
            .asrt(r#"dx build"#, |targets| async move {
                let t = targets.unwrap();
                assert_eq!(t.client.bundle, BundleFormat::Web);
                assert!(t.server.is_none());
            })
            .asrt(r#"dx build @client --package harness-simple-dedicated-client @server --package harness-simple-dedicated-server"#, |targets| async move {
                    let t = targets.unwrap();
                    assert_eq!(t.client.bundle, BundleFormat::Web);
                    let s = t.server.unwrap();
                    assert_eq!(s.bundle, BundleFormat::Server);
                    assert_eq!(s.triple, Triple::host());
                },
            )
            .asrt(r#"dx build @client --package harness-simple-dedicated-client @server --package harness-simple-dedicated-server --target wasm32-unknown-unknown"#, |targets| async move {
                let t = targets.unwrap();
                assert_eq!(t.client.bundle, BundleFormat::Web);
                let s = t.server.unwrap();
                assert_eq!(s.bundle, BundleFormat::Server);
                assert_eq!(s.triple, "wasm32-unknown-unknown".parse().unwrap());
            }),
        TestHarnessBuilder::new("harness-renderer-swap")
            .deps(r#"dioxus = { workspace = true, features = ["fullstack"] }"#)
            .fetr(r#"default=["desktop", "server"]"#)
            .fetr(r#"desktop=["dioxus/desktop"]"#)
            .fetr(r#"native=["dioxus/native"]"#)
            .fetr(r#"server=["dioxus/server"]"#)
            .asrt(
                r#"dx build --desktop --renderer native"#,
                |targets| async move {
                    let t = targets.unwrap();
                    assert_eq!(t.client.bundle, BundleFormat::host());
                    let server = t.server.unwrap();
                    assert_eq!(server.bundle, BundleFormat::Server);
                    assert_eq!(server.triple, Triple::host());
                },
            ),
        TestHarnessBuilder::new("harness-default-to-non-default")
            .deps(r#"dioxus = { workspace = true, features = [] }"#)
            .fetr(r#"default=["web"]"#)
            .fetr(r#"web=["dioxus/web"]"#)
            .asrt(
                r#"dx build --ios"#,
                |targets| async move {
                    if cfg!(not(target_os = "macos")) {
                        // iOS builds require Xcode toolchain, only available on macOS
                        return;
                    }
                    let t = targets.unwrap();
                    assert!(t.server.is_none());
                    assert_eq!(t.client.bundle, BundleFormat::Ios);
                    assert_eq!(t.client.triple, TestHarnessBuilder::host_ios_triple_sim());
                },
            ),
        TestHarnessBuilder::new("harness-fullstack-with-optional-tokio")
            .deps(r#"dioxus = { workspace = true, features = ["fullstack"] }"#)
            .deps(r#"serde = { workspace = true }"#)
            .deps(r#"tokio = { workspace = true, features = ["full"], optional = true }"#)
            .fetr(r#"default = []"#)
            .fetr(r#"server = ["dioxus/server", "dep:tokio"]"#)
            .fetr(r#"web = ["dioxus/web"]"#)
            // .asrt(r#"dx build"#, |targets| async move {
            //     assert!(targets.is_err())
            // })
            .asrt(r#"dx build --web"#, |targets| async move {
                let t = targets.unwrap();
                assert_eq!(t.client.bundle, BundleFormat::Web);
                assert_eq!(t.client.triple, "wasm32-unknown-unknown".parse().unwrap());
                let server = t.server.unwrap();
                assert_eq!(server.bundle, BundleFormat::Server);
                assert_eq!(server.triple, Triple::host());
            }),
        TestHarnessBuilder::new("harness-web-with-no-default-features")
            .deps(r#"dioxus = { workspace = true }"#)
            .fetr(r#"default=["other"]"#)
            .fetr(r#"other=[]"#)
            .asrt(r#"dx build --no-default-features --web"#, |targets| async move {
                let t = targets.unwrap();
                assert_eq!(t.client.bundle, BundleFormat::Web);
                assert_eq!(t.client.features, vec!["dioxus/web"]);
                assert!(t.server.is_none());
            }),
        TestHarnessBuilder::new("harness-web-with-default-features")
            .deps(r#"dioxus = { workspace = true }"#)
            .fetr(r#"default=["other"]"#)
            .fetr(r#"other=[]"#)
            .asrt(r#"dx build --web"#, |targets| async move {
                let t = targets.unwrap();
                assert_eq!(t.client.bundle, BundleFormat::Web);
                assert_eq!(t.client.features.iter().map(|s| s.as_str()).collect::<HashSet<_>>(), ["dioxus/web", "other"].into_iter().collect::<HashSet<_>>());
                assert!(t.server.is_none());
            }),
        TestHarnessBuilder::new("harness-simple-desktop-tests")
            .deps(r#"dioxus = { workspace = true, features = ["desktop"] }"#)
            .asrt(r#"dx check"#, |targets| async move {
                let t = targets.unwrap();
                assert_eq!(t.client.bundle, BundleFormat::host());
                assert_eq!(t.client.triple, Triple::host());
                assert!(t.server.is_none());

                // The rustc-wrapper scope dir must differ across build kinds so check
                // captures never overwrite the fat/base captures used for hotpatch replay.
                let mut req = t.client.clone();
                let build = req
                    .rustc_wrapper_scope_dir_name(&crate::BuildMode::Base)
                    .unwrap();
                req.kind = crate::BuildKind::Check;
                let check = req
                    .rustc_wrapper_scope_dir_name(&crate::BuildMode::Base)
                    .unwrap();
                assert_ne!(build, check);
            }),
        TestHarnessBuilder::new("harness-web-tests")
            .deps(r#"dioxus = { workspace = true, features = ["web"] }"#)
            .manifest_tail(
                r#"
[dev-dependencies]
dioxus-test-harness = { workspace = true }
dioxus-core = { workspace = true }
dioxus-signals = { workspace = true }

[[test]]
name = "web"
harness = false
"#,
            )
            .file(
                "tests/web.rs",
                r#"dioxus_test_harness::main!();

#[dioxus_test_harness::test]
fn adds() {
    assert_eq!(std::hint::black_box(1) + 1, 2);
}

#[dioxus_test_harness::test]
async fn async_ok() {}

#[dioxus_test_harness::test(ignore)]
fn skipped() {}

#[dioxus_test_harness::test(should_panic)]
fn panics() {
    panic!("expected panic");
}

#[dioxus_test_harness::test(tags = ["ui"])]
fn renders() {
    let mut dom = dioxus::prelude::VirtualDom::new(|| dioxus::prelude::rsx! { "hello" });
    dom.rebuild_in_place();
}

#[dioxus_test_harness::test]
fn fails() {
    assert_eq!(std::hint::black_box(1) + 1, 3);
}
"#,
            )
            .asrt(r#"dx test --web"#, |targets| async move {
                let t = targets.unwrap();
                assert_eq!(t.client.bundle, BundleFormat::Web);
                assert_eq!(t.client.triple, "wasm32-unknown-unknown".parse().unwrap());
            }),
    ])
    .await;
}

#[derive(Default)]
struct TestHarnessBuilder {
    name: String,
    dependencies: String,
    features: String,
    manifest_tail: String,
    files: Vec<(String, String)>,
    futures: Vec<TestHarnessTestCase>,
}

struct TestHarnessTestCase {
    args: String,
    #[allow(clippy::type_complexity)]
    callback: Box<dyn FnOnce(Result<BuildTargets>) -> Pin<Box<dyn Future<Output = ()>>>>,
}

impl TestHarnessBuilder {
    fn new(name: &str) -> Self {
        Self {
            name: name.into(),
            ..Default::default()
        }
    }

    /// Add a dependency to the test harness.
    fn deps(mut self, dependencies: impl Into<String>) -> Self {
        writeln!(&mut self.dependencies, "{}", dependencies.into()).unwrap();
        self
    }

    /// Add a feature to the test harness.
    fn fetr(mut self, features: impl Into<String>) -> Self {
        writeln!(&mut self.features, "{}", features.into()).unwrap();
        self
    }

    /// Append extra manifest sections (dev-dependencies, `[[test]]` targets, ...).
    fn manifest_tail(mut self, manifest_tail: impl Into<String>) -> Self {
        self.manifest_tail = manifest_tail.into();
        self
    }

    /// Write an extra file into the harness (eg `tests/web.rs`).
    fn file(mut self, path: impl Into<String>, contents: impl Into<String>) -> Self {
        self.files.push((path.into(), contents.into()));
        self
    }

    /// Assert the expected behavior of the test harness.
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

    /// Write the test harness to the filesystem.
    fn build(&self, harness_dir: &Path) {
        let name = self.name.clone();
        let dependencies = self.dependencies.clone();
        let features = self.features.clone();
        let manifest_tail = self.manifest_tail.clone();

        let test_dir = harness_dir.join(&name);

        _ = std::fs::remove_dir_all(&test_dir);
        std::fs::create_dir_all(&test_dir).unwrap();
        std::fs::create_dir_all(test_dir.join("src")).unwrap();

        // `dependencies` and `features` already end with `\n` (writeln! appends one), so closing
        // the raw string immediately after `{features}` keeps the generated file at a single
        // trailing newline. The previous form put 4 spaces and `"#` on their own line, which
        // landed inside the string and produced "    " (no newline) at EOF — pure diff noise.
        let cargo_toml = format!(
            r#"[package]
name = "{name}"
version = "0.0.1"
edition = "2024"
license = "MIT OR Apache-2.0"
publish = false

[dependencies]
{dependencies}
[features]
{features}{manifest_tail}"#,
            name = name,
            dependencies = dependencies,
            features = features,
            manifest_tail = manifest_tail
        );

        std::fs::write(test_dir.join("Cargo.toml"), cargo_toml).unwrap();

        let contents = if features.contains("dioxus") {
            r#"use dioxus::prelude::*;
fn main() {
    dioxus::launch(|| rsx! { "hello world!" })
}
"#
        } else {
            r#"fn main() {
    println!("Hello, world!");
}
"#
        };

        std::fs::write(test_dir.join("src/main.rs"), contents).unwrap();

        for (path, contents) in &self.files {
            let path = test_dir.join(path);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, contents).unwrap();
        }
    }

    async fn run(harnesses: Vec<Self>) {
        _ = crate::VERBOSITY.set(crate::Verbosity {
            verbose: true,
            trace: true,
            json_output: false,
            log_to_file: None,
            locked: false,
            offline: false,
            frozen: false,
        });

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

        // Erase old entries in the harness directory, but keep files (ie README.md) around
        for entry in std::fs::read_dir(&harness_dir).unwrap() {
            let entry = entry.unwrap();
            if entry.file_type().unwrap().is_dir() {
                std::fs::remove_dir_all(entry.path()).unwrap();
            }
        }

        // Now that the harnesses are written to the filesystem, we can call cargo_metadata
        // It will be cached from here
        let mut futures = FuturesUnordered::new();
        let mut seen_names = HashSet::new();

        for harness in harnesses {
            if !seen_names.insert(harness.name.clone()) {
                panic!("Duplicate test harness name found: {}", harness.name);
            }

            harness.build(&harness_dir);

            for case in harness.futures {
                let mut escaped = shell_words::split(&case.args).unwrap();
                if !(escaped.contains(&"--package".to_string())
                    || escaped.contains(&"@server".to_string())
                    || escaped.contains(&"@client".to_string()))
                {
                    escaped.push("--package".to_string());
                    escaped.push(harness.name.clone());
                }
                let args = Cli::try_parse_from(escaped).unwrap();
                let build_args = match args.action {
                    Commands::Build(build_args) => build_args,
                    Commands::Check(check) => check.build_args,
                    _ => panic!("Expected build/check command"),
                };

                futures.push(async move {
                    let targets = build_args.into_targets().await;
                    (case.callback)(targets).await;
                });
            }
        }

        // Give a moment for fs to catch up
        std::thread::sleep(std::time::Duration::from_secs(1));

        let _workspace = Workspace::current().await.unwrap();

        while let Some(_res) = futures.next().await {}
    }

    fn host_ios_triple_sim() -> Triple {
        if cfg!(target_arch = "aarch64") {
            "aarch64-apple-ios-sim".parse().unwrap()
        } else {
            "x86_64-apple-ios".parse().unwrap()
        }
    }
}
