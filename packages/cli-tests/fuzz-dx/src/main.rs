use headless_chrome::Browser;
use pretty_assertions::assert_eq;
use rand::{random, seq::IndexedRandom};
use std::{
    io::Write,
    path::{Path, PathBuf},
};

mod generate_mock_dioxus;
mod random_project;

static PLATFORMS: &[&str] = &["web", "desktop", "liveview"];

fn dx_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("cli")
}

fn run_dx_local() -> std::process::Command {
    let mut command = std::process::Command::new("cargo");
    command
        .arg("run")
        .arg("-q")
        .arg("--manifest-path")
        .arg(dx_path().join("Cargo.toml"))
        .arg("--");
    command
}

fn run_dx_installed() -> std::process::Command {
    std::process::Command::new("dx")
}

fn run_dx(installed: bool) -> std::process::Command {
    if installed {
        run_dx_installed()
    } else {
        run_dx_local()
    }
}

fn add_args(
    command: &mut std::process::Command,
    platform: Option<&str>,
    features: &[String],
    fullstack: bool,
    crate_dir: &Path,
) {
    if let Some(platform) = platform {
        command.arg("--platform").arg(platform);
    }
    if !features.is_empty() {
        command.arg("--features").arg(features.join(","));
    }
    if fullstack {
        command.arg("--fullstack");
    }
    command
        .current_dir(crate_dir)
        .stdout(std::process::Stdio::null())
        .stdin(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
}

fn test_port(
    installed: bool,
    platform: Option<&str>,
    features: &[String],
    fullstack: bool,
    crate_dir: &Path,
    port: u16,
) -> anyhow::Result<Vec<String>> {
    let mut command = run_dx(installed);
    command.arg("build");
    add_args(&mut command, platform, features, fullstack, crate_dir);
    command.output()?;
    let mut command = run_dx(installed);
    command.arg("serve");
    add_args(&mut command, platform, features, fullstack, crate_dir);
    println!("command: {:?}", command);
    let mut output = command.spawn()?;

    // Wait until the server is alive
    let url = format!("http://127.0.0.1:{port}");
    let mut hit = false;
    for _ in 0..5 {
        let response = reqwest::blocking::get(&url)
            .and_then(|resp| resp.text())
            .map_or(false, |f| {
                // Some versions of dx return a 404 response with a 200 code
                !f.contains("dioxus is not currently serving a web app")
            });
        std::thread::sleep(std::time::Duration::from_millis(100));
        if response {
            hit = true;
            break;
        }
    }
    let features = if hit {
        features_enabled_web(&url)?
    } else {
        let text_value = std::fs::read_to_string(crate_dir.join("features.txt"))?;
        text_value.lines().map(|line| line.to_string()).collect()
    };

    output.kill()?;

    Ok(features)
}

fn get_features_enabled_for_platform(
    installed: bool,
    platform: Option<&str>,
    features: &[String],
    fullstack: bool,
    crate_dir: &Path,
    port: u16,
) -> anyhow::Result<Vec<String>> {
    test_port(installed, platform, features, fullstack, crate_dir, port)
}

fn main() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = root.parent().unwrap().join("temp");
    std::fs::create_dir_all(&root).unwrap();
    let dioxus_features = generate_mock_dioxus::generate_mock_dioxus(&root);
    for _ in 0..100 {
        let crate_dir = root.join("random-dioxus");
        let features = random_project::create_random_project(&crate_dir, &dioxus_features);
        // Print the toml
        println!(
            "Testing with toml:\n{}",
            std::fs::read_to_string(crate_dir.join("Cargo.toml")).unwrap()
        );
        test_project(&crate_dir, &features, 8080, 8081);
    }
    _ = std::fs::remove_dir_all(&root);
}

fn test_project(crate_dir: &Path, features: &[String], port1: u16, port2: u16) {
    // Enable features randomly
    let enabled_features: Vec<_> = features
        .iter()
        .filter(|_| random::<bool>())
        .cloned()
        .collect();
    // Choose a random platform
    let platform = rand::random_bool(0.8).then(|| *PLATFORMS.choose(&mut rand::rng()).unwrap());
    // Randomly set fullstack
    let fullstack = rand::random_bool(0.2);
    println!(
        "Testing platform {platform:?} with features {enabled_features:?} and fullstack {fullstack}"
    );
    let old_enabled_features = get_features_enabled_for_platform(
        true,
        platform,
        &enabled_features,
        fullstack,
        &crate_dir,
        port1,
    );
    let new_enabled_features = get_features_enabled_for_platform(
        false,
        platform,
        &enabled_features,
        fullstack,
        &crate_dir,
        port2,
    );

    match (old_enabled_features, new_enabled_features) {
        (Ok(old_features), Ok(new_features)) => {
            assert_eq!(
                old_features, new_features,
                "Features do not match for platform {platform:?} and features {enabled_features:?}"
            );
            println!("✅ Passed! {old_features:?} == {new_features:?}\n");
        }
        (Err(_), Err(_)) => {
            println!("❓ Both versions of dx failed\n");
        }
        (Ok(_), Err(new_error)) => {
            panic!("New features failed to load: {new_error}");
        }
        (Err(old_error), Ok(_)) => {
            panic!("Old features failed to load: {old_error}");
        }
    }
}

fn features_enabled_web(url: &str) -> anyhow::Result<Vec<String>> {
    let browser = Browser::default()?;

    let tab = browser.new_tab()?;

    // Navigate to localhost
    tab.navigate_to(&url)?;

    let text_value = tab.wait_for_element("#features")?.get_inner_text()?;

    let features = text_value.lines().map(|line| line.to_string()).collect();

    Ok(features)
}
