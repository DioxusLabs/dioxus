use headless_chrome::Browser;
use rand::{random, seq::IndexedRandom};
use std::path::{Path, PathBuf};

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

fn test_web(
    installed: bool,
    platform: &str,
    features: &[String],
    crate_dir: &Path,
    port: u16,
) -> anyhow::Result<Vec<String>> {
    _ = run_dx(installed)
        .arg("build")
        .arg("--platform")
        .arg(platform)
        .arg("--features")
        .arg(features.join(","))
        .current_dir(&crate_dir)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .output()
        .expect("Failed to start dioxus server");

    let mut command = run_dx(installed)
        .arg("serve")
        .arg("--platform")
        .arg(platform)
        .arg("--features")
        .arg(features.join(","))
        .current_dir(&crate_dir)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to start dioxus server");

    // Wait until the server is alive
    let url = format!("http://127.0.0.1:{port}");
    loop {
        let response = reqwest::blocking::get(&url).map_or(false, |r| r.status().is_success());
        if response {
            break;
        }
    }
    let features = features_enabled_web(&url).expect("Failed to get features");

    command.kill().expect("Failed to wait for dioxus server");

    Ok(features)
}

fn test_desktop(
    installed: bool,
    platform: &str,
    features: &[String],
    crate_dir: &Path,
) -> anyhow::Result<Vec<String>> {
    let mut command = run_dx(installed)
        .arg("run")
        .arg("--platform")
        .arg(platform)
        .arg("--features")
        .arg(features.join(","))
        .current_dir(&crate_dir)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to start dioxus server");

    command.wait().expect("Failed to wait for dioxus server");
    let text_value = std::fs::read_to_string(crate_dir.join("features.txt"))?;
    let features = text_value.lines().map(|line| line.to_string()).collect();

    Ok(features)
}

fn get_features_enabled_for_platform(
    installed: bool,
    platform: &str,
    features: &[String],
    crate_dir: &Path,
    port: u16,
) -> anyhow::Result<Vec<String>> {
    if platform == "web" {
        test_web(installed, platform, features, crate_dir, port)
    } else {
        test_desktop(installed, platform, features, crate_dir)
    }
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
        test_project(&crate_dir, &features, 8080);
    }
    _ = std::fs::remove_dir_all(&root);
}

fn test_project(crate_dir: &Path, features: &[String], port: u16) {
    // Enable features randomly
    let enabled_features: Vec<_> = features
        .iter()
        .filter(|_| random::<bool>())
        .cloned()
        .collect();
    // Choose a random platform
    let platform = PLATFORMS.choose(&mut rand::rng()).unwrap();
    println!("Testing platform {platform} with features {enabled_features:?}");
    let old_enabled_features =
        get_features_enabled_for_platform(true, platform, &enabled_features, &crate_dir, port)
            .unwrap();
    let new_enabled_features =
        get_features_enabled_for_platform(false, platform, &enabled_features, &crate_dir, port)
            .unwrap();

    assert_eq!(
        old_enabled_features, new_enabled_features,
        "Features do not match for platform {platform} and features {enabled_features:?}"
    );
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
