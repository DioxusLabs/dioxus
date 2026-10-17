//! End to end tests for `dx components`, against a local git registry.

use std::{
    path::{Path, PathBuf},
    process::{Command, Output},
};

fn git(dir: &Path, args: &[&str]) {
    let output = Command::new("git")
        .args(["-c", "user.name=test", "-c", "user.email=test@example.com"])
        .args(["-c", "init.defaultBranch=main"])
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn strings(args: &[&str]) -> Vec<String> {
    args.iter().map(|arg| arg.to_string()).collect()
}

fn write(path: &Path, contents: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, contents).unwrap();
}

/// Commit a registry on `branch` with a `button` component that depends on the registry's
/// `primitives` crate
fn commit_registry(registry: &Path, branch: &str, button: &str, primitives: &str) {
    git(registry, &["checkout", "-B", branch]);
    write(
        &registry.join("component.json"),
        r#"{ "name": "registry", "members": ["button"] }"#,
    );
    let button_manifest = serde_json::json!({
        "name": "button",
        "description": "A button",
        "globalAssets": ["../theme.css"],
        "cargoDependencies": [{ "name": "primitives", "git": registry_url(registry) }],
    });
    write(
        &registry.join("button/component.json"),
        &button_manifest.to_string(),
    );
    write(&registry.join("button/mod.rs"), button);
    write(&registry.join("theme.css"), "/* theme */\n");
    write(
        &registry.join("primitives/Cargo.toml"),
        "[package]\nname = \"primitives\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    );
    write(&registry.join("primitives/src/lib.rs"), primitives);
    git(registry, &["add", "."]);
    git(registry, &["commit", "-m", branch]);
}

fn registry_url(registry: &Path) -> String {
    format!("file://{}", registry.display())
}

const PRIMITIVES: &str = "pub fn primitive() {}\n";
const MAIN_PRIMITIVES: &str = "compile_error!(\"primitives from main\");\n";

struct Fixture {
    _dir: tempfile::TempDir,
    registry: PathBuf,
    project: PathBuf,
    dx_home: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let registry = dir.path().join("registry");
        std::fs::create_dir(&registry).unwrap();
        git(&registry, &["init"]);
        commit_registry(&registry, "main", "// main\n", MAIN_PRIMITIVES);
        commit_registry(&registry, "0.8", "// 0.8\n", PRIMITIVES);
        git(&registry, &["checkout", "main"]);

        let project = dir.path().join("project");
        write(
            &project.join("Cargo.toml"),
            "[package]\nname = \"project\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[workspace]\n",
        );
        write(&project.join("src/main.rs"), "fn main() {}\n");

        let dx_home = dir.path().join("dx-home");
        Self {
            _dir: dir,
            registry,
            project,
            dx_home,
        }
    }

    fn dx(&self, args: &[String]) -> Output {
        let output = Command::new(env!("CARGO_BIN_EXE_dx"))
            .args(args)
            .current_dir(&self.project)
            .env("DX_HOME", &self.dx_home)
            .env("CARGO_TARGET_DIR", self.project.join("target"))
            .output()
            .unwrap();
        eprintln!(
            "dx {args:?}\n{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        output
    }

    fn registry_args(&self, rev: &str) -> Vec<String> {
        let url = registry_url(&self.registry);
        ["--git", &url, "--rev", rev].map(String::from).to_vec()
    }

    fn read(&self, path: &str) -> String {
        std::fs::read_to_string(self.project.join(path)).unwrap()
    }
}

#[test]
fn add_copies_the_component_from_the_requested_branch() {
    let fixture = Fixture::new();
    let mut args = strings(&["components", "add", "button"]);
    args.extend(fixture.registry_args("0.8"));
    assert!(fixture.dx(&args).status.success());

    assert_eq!(fixture.read("src/components/button/mod.rs"), "// 0.8\n");
    assert!(
        fixture
            .read("src/components/mod.rs")
            .contains("pub mod button;\n")
    );
    assert_eq!(fixture.read("assets/theme.css"), "/* theme */\n");
}

#[test]
fn registry_dependencies_come_from_the_same_commit_as_the_component() {
    let fixture = Fixture::new();
    write(
        &fixture.project.join("src/main.rs"),
        "mod components;\nfn main() {\n    primitives::primitive();\n}\n",
    );
    let mut args = strings(&["components", "add", "button"]);
    args.extend(fixture.registry_args("0.8"));
    assert!(fixture.dx(&args).status.success());

    let check = Command::new(env!("CARGO"))
        .args(["check", "--quiet"])
        .current_dir(&fixture.project)
        .env("CARGO_TARGET_DIR", fixture.project.join("target"))
        .status()
        .unwrap();
    assert!(check.success(), "cargo check failed");
}

#[test]
fn update_fetches_new_commits_on_the_branch() {
    let fixture = Fixture::new();
    let mut add = strings(&["components", "add", "button", "--force"]);
    add.extend(fixture.registry_args("0.8"));
    assert!(fixture.dx(&add).status.success());

    commit_registry(&fixture.registry, "0.8", "// 0.8 updated\n", PRIMITIVES);
    let mut update = strings(&["components", "update"]);
    update.extend(fixture.registry_args("0.8"));
    assert!(fixture.dx(&update).status.success());

    assert!(fixture.dx(&add).status.success());
    assert_eq!(
        fixture.read("src/components/button/mod.rs"),
        "// 0.8 updated\n"
    );
}

#[test]
fn a_missing_rev_leaves_no_cache_behind() {
    let fixture = Fixture::new();
    let mut args = strings(&["components", "add", "button"]);
    args.extend(fixture.registry_args("no-such-branch"));
    assert!(!fixture.dx(&args).status.success());

    let cache = fixture.dx_home.join("components");
    let entries = std::fs::read_dir(&cache).map_or(0, |dir| dir.count());
    assert_eq!(entries, 0, "{} is not empty", cache.display());
}
