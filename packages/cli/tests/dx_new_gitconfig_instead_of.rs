use escargot::{CargoBuild, CargoRun};
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::LazyLock,
};

static DX_BIN: LazyLock<CargoRun> = LazyLock::new(|| {
    CargoBuild::new()
        .bin("dx")
        .release()
        .run()
        .expect("failed to build dx for tests")
});

fn git_available() -> bool {
    Command::new("git")
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok()
}

fn run_git(current_dir: &Path, args: &[&str]) -> Output {
    Command::new("git")
        .args(args)
        .current_dir(current_dir)
        .output()
        .expect("git command failed to run")
}

fn assert_git_success(args: &[&str], output: &Output) {
    assert!(
        output.status.success(),
        "git {:?} failed\nstdout:\n{}\nstderr:\n{}",
        args,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn file_url(path: &Path) -> String {
    format!("file:///{}", path.display().to_string().replace('\\', "/"))
}

fn write_template_repo(root: &Path) -> PathBuf {
    let template_dir = root.join("template");
    fs::create_dir_all(template_dir.join("src")).expect("create template src");

    fs::write(
        template_dir.join("Cargo.toml"),
        r#"[package]
name = "{{project-name}}"
version = "0.1.0"
edition = "2021"

[dependencies]
"#,
    )
    .expect("write template Cargo.toml");

    fs::write(
        template_dir.join("src/main.rs"),
        r#"fn main() {
    println!("hello");
}
"#,
    )
    .expect("write template src/main.rs");

    fs::write(template_dir.join("README.md"), "# test template\n").expect("write template README");

    let init_args = ["init", "-b", "main"];
    let output = run_git(&template_dir, &init_args);
    assert_git_success(&init_args, &output);

    let add_args = ["add", "."];
    let output = run_git(&template_dir, &add_args);
    assert_git_success(&add_args, &output);

    let commit_args = [
        "-c",
        "user.email=test@example.com",
        "-c",
        "user.name=test",
        "commit",
        "-m",
        "init template",
    ];
    let output = run_git(&template_dir, &commit_args);
    assert_git_success(&commit_args, &output);

    template_dir
}

#[test]
fn dx_new_succeeds_even_when_gitconfig_instead_of_breaks_libgit2_clone() {
    if !git_available() {
        return;
    }

    let temp = tempfile::tempdir().expect("create temp dir");
    let template_dir = write_template_repo(temp.path());
    let template_url = file_url(&template_dir);

    let home_dir = temp.path().join("home");
    fs::create_dir_all(&home_dir).expect("create fake HOME dir");
    fs::write(
        home_dir.join(".gitconfig"),
        r#"[url "ssh://git@127.0.0.1:1/"]
    insteadOf = file:///
"#,
    )
    .expect("write .gitconfig");

    let dest_dir = temp.path().join("out").join("myapp");

    let mut cmd = DX_BIN.command();
    cmd.arg("new")
        .arg(&dest_dir)
        .arg("--template")
        .arg(template_url)
        .arg("--branch")
        .arg("main")
        .arg("--yes")
        .arg("--vcs")
        .arg("none")
        .env("HOME", &home_dir)
        .env("USERPROFILE", &home_dir)
        .env("XDG_CONFIG_HOME", &home_dir);

    let output = cmd.output().expect("dx failed to run");
    assert!(
        output.status.success(),
        "dx new failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    assert!(dest_dir.join("Cargo.toml").exists());
    assert!(dest_dir.join("src/main.rs").exists());
    assert!(dest_dir.join("README.md").exists());
}
