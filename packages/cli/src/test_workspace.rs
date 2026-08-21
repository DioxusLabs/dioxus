//! Shared fixtures for tests that need a real cargo workspace on disk.

use krates::{Cmd, Krates};
use std::fs;

/// Write the workspace from the bug report to disk and load its real cargo metadata: an app
/// served for wasm32, a library it uses, and a native-only sibling that depends on the same
/// library but is only reachable from the app on non-wasm targets.
pub(crate) fn native_sibling_workspace() -> (tempfile::TempDir, Krates) {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();

    for (name, manifest, source) in [
        (
            "shared-lib",
            "[package]\nname = \"shared-lib\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
            "pub fn greeting() -> &'static str { \"hi\" }\n",
        ),
        (
            "native_ffi",
            "[package]\nname = \"native_ffi\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\nshared-lib = { path = \"../shared-lib\" }\n",
            "pub fn native() -> &'static str { shared_lib::greeting() }\n",
        ),
        (
            "app",
            "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\nshared-lib = { path = \"../shared-lib\" }\n\n[target.'cfg(not(target_arch = \"wasm32\"))'.dependencies]\nnative_ffi = { path = \"../native_ffi\" }\n",
            "pub fn app() {}\n",
        ),
    ] {
        fs::create_dir_all(root.join(name).join("src")).expect("crate dir");
        fs::write(root.join(name).join("Cargo.toml"), manifest).expect("manifest");
        fs::write(root.join(name).join("src").join("lib.rs"), source).expect("source");
    }

    fs::write(
        root.join("Cargo.toml"),
        "[workspace]\nresolver = \"2\"\nmembers = [\"app\", \"shared-lib\", \"native_ffi\"]\n",
    )
    .expect("workspace manifest");

    let mut cmd = Cmd::new();
    cmd.current_dir(root);
    cmd.other_options(["--offline".to_string()]);
    let mut builder = krates::Builder::new();
    builder.workspace(true);
    let krates = builder.build(cmd, |_| {}).expect("cargo metadata");

    (dir, krates)
}

/// A workspace where two distinct packages normalise to the same rustc crate name. Cargo allows
/// this: `dup-name` and `dup_name` are different package names that both compile as `dup_name`.
pub(crate) fn colliding_member_names_workspace() -> (tempfile::TempDir, Krates) {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();

    for name in ["dup-name", "dup_name"] {
        fs::create_dir_all(root.join(name).join("src")).expect("crate dir");
        fs::write(
            root.join(name).join("Cargo.toml"),
            format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n"),
        )
        .expect("manifest");
        fs::write(
            root.join(name).join("src").join("lib.rs"),
            "pub fn f() {}\n",
        )
        .expect("source");
    }

    fs::write(
        root.join("Cargo.toml"),
        "[workspace]\nresolver = \"2\"\nmembers = [\"dup-name\", \"dup_name\"]\n",
    )
    .expect("workspace manifest");

    let mut cmd = Cmd::new();
    cmd.current_dir(root);
    cmd.other_options(["--offline".to_string()]);
    let mut builder = krates::Builder::new();
    builder.workspace(true);
    let krates = builder.build(cmd, |_| {}).expect("cargo metadata");

    (dir, krates)
}
