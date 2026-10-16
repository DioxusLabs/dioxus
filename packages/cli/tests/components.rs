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

fn write(path: &Path, contents: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, contents).unwrap();
}

/// Commit a registry with a single `button` component on `branch`
fn commit_registry(registry: &Path, branch: &str, button: &str) {
    git(registry, &["checkout", "-B", branch]);
    write(
        &registry.join("component.json"),
        r#"{ "name": "registry", "members": ["button"] }"#,
    );
    write(
        &registry.join("button/component.json"),
        r#"{ "name": "button", "description": "A button", "globalAssets": ["../theme.css"] }"#,
    );
    write(&registry.join("button/mod.rs"), button);
    write(&registry.join("theme.css"), "/* theme */\n");
    git(registry, &["add", "."]);
    git(registry, &["commit", "-m", branch]);
}

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
        commit_registry(&registry, "main", "// main\n");
        commit_registry(&registry, "0.8", "// 0.8\n");
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

    fn dx(&self, args: &[&str]) -> Output {
        let output = Command::new(env!("CARGO_BIN_EXE_dx"))
            .args(args)
            .current_dir(&self.project)
            .env("DX_HOME", &self.dx_home)
            .output()
            .unwrap();
        eprintln!(
            "dx {args:?}\n{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        output
    }

    fn registry_args<'a>(&'a self, rev: &'a str) -> [&'a str; 4] {
        ["--git", self.registry.to_str().unwrap(), "--rev", rev]
    }

    fn read(&self, path: &str) -> String {
        std::fs::read_to_string(self.project.join(path)).unwrap()
    }
}

#[test]
fn add_copies_the_component_from_the_requested_branch() {
    let fixture = Fixture::new();
    let mut args = vec!["components", "add", "button"];
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
fn update_fetches_new_commits_on_the_branch() {
    let fixture = Fixture::new();
    let mut add = vec!["components", "add", "button", "--force"];
    add.extend(fixture.registry_args("0.8"));
    assert!(fixture.dx(&add).status.success());

    commit_registry(&fixture.registry, "0.8", "// 0.8 updated\n");
    let mut update = vec!["components", "update"];
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
    let mut args = vec!["components", "add", "button"];
    args.extend(fixture.registry_args("no-such-branch"));
    assert!(!fixture.dx(&args).status.success());

    let cache = fixture.dx_home.join("components");
    let entries = std::fs::read_dir(&cache).map_or(0, |dir| dir.count());
    assert_eq!(entries, 0, "{} is not empty", cache.display());
}
