use std::{
    fs,
    io::Write,
    net::{TcpListener, TcpStream},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::LazyLock,
    time::Duration,
};

use escargot::CargoBuild;

static DX: LazyLock<escargot::CargoRun> = LazyLock::new(|| {
    CargoBuild::new()
        .bin("dx")
        // `dx` currently overflows the stack in debug builds on Windows; always build release.
        .release()
        .run()
        .expect("Couldn't build dx for tests")
});

struct ChildGuard(Child);

impl Drop for ChildGuard {
    fn drop(&mut self) {
        _ = self.0.kill();
        _ = self.0.wait();
    }
}

fn run_git(current_dir: &Path, args: &[&str]) {
    let output = Command::new("git")
        .args(args)
        .current_dir(current_dir)
        .output()
        .expect("git command failed to run");

    assert!(
        output.status.success(),
        "git {:?} failed\nstdout:\n{}\nstderr:\n{}",
        args,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn wait_for_port(addr: (&str, u16)) {
    for _ in 0..50 {
        if TcpStream::connect(addr).is_ok() {
            return;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    panic!("git daemon did not start listening on {:?}", addr);
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

    fs::write(
        template_dir.join("README.md"),
        r#"# Template

This is a test template.
"#,
    )
    .expect("write template README.md");

    run_git(&template_dir, &["init", "-b", "main"]);
    run_git(&template_dir, &["add", "."]);
    run_git(
        &template_dir,
        &[
            "-c",
            "user.email=test@example.com",
            "-c",
            "user.name=test",
            "commit",
            "-m",
            "init template",
        ],
    );

    template_dir
}

fn write_rewritten_gitconfig(home_dir: &Path) {
    fs::create_dir_all(home_dir).expect("create fake HOME dir");
    let mut file = fs::File::create(home_dir.join(".gitconfig")).expect("create .gitconfig");
    file.write_all(
        br#"[url "invalid://invalid-host/"]
    insteadOf = git://
"#,
    )
    .expect("write .gitconfig");
}

#[test]
fn dx_new_succeeds_even_with_url_insteadof_rewrite() {
    let temp = tempfile::tempdir().expect("create temp dir");

    let template_dir = write_template_repo(temp.path());

    let repos_dir = temp.path().join("repos");
    fs::create_dir_all(&repos_dir).expect("create repos dir");
    let bare_repo = repos_dir.join("template.git");
    run_git(
        temp.path(),
        &[
            "clone",
            "--bare",
            template_dir.to_string_lossy().as_ref(),
            bare_repo.to_string_lossy().as_ref(),
        ],
    );

    let port = TcpListener::bind(("127.0.0.1", 0))
        .expect("bind local port")
        .local_addr()
        .expect("local_addr")
        .port();

    let daemon = Command::new("git")
        .args([
            "daemon",
            "--export-all",
            &format!("--base-path={}", repos_dir.display()),
            "--reuseaddr",
            "--listen=127.0.0.1",
            &format!("--port={port}"),
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn git daemon");
    let _daemon = ChildGuard(daemon);
    wait_for_port(("127.0.0.1", port));

    let home_dir = temp.path().join("home");
    write_rewritten_gitconfig(&home_dir);

    let out_dir = temp.path().join("out");
    let project_dir = out_dir.join("myapp");
    let template_url = format!("git://127.0.0.1:{port}/template.git");

    let output = DX
        .command()
        .args([
            "new",
            project_dir.to_string_lossy().as_ref(),
            "--yes",
            "--template",
            &template_url,
            "--branch",
            "main",
            "--vcs",
            "none",
        ])
        .env("HOME", &home_dir)
        .env("USERPROFILE", &home_dir)
        .env_remove("GIT_CONFIG_GLOBAL")
        .env_remove("GIT_CONFIG_NOSYSTEM")
        .current_dir(temp.path())
        .output()
        .expect("run dx new");

    assert!(
        output.status.success(),
        "dx new failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );

    let cargo_toml = fs::read_to_string(project_dir.join("Cargo.toml")).expect("read Cargo.toml");
    assert!(cargo_toml.contains("name = \"myapp\""));
    assert!(project_dir.join("src/main.rs").exists());
    assert!(project_dir.join("README.md").exists());
}
