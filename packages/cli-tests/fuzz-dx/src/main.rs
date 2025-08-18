use std::fs::create_dir_all;
use std::path::{Path, PathBuf};

use headless_chrome::Browser;

static PLATFORMS: &[&str] = &["web", "desktop", "mobile", "liveview"];

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
    scrap_dir: &Path,
    port: u16,
) -> anyhow::Result<Vec<String>> {
    let mut command = run_dx(installed)
        .arg("serve")
        .arg("--platform")
        .arg(platform)
        .current_dir(&scrap_dir)
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

fn test_desktop(installed: bool, platform: &str, scrap_dir: &Path) -> anyhow::Result<Vec<String>> {
    let mut command = run_dx(installed)
        .arg("run")
        .arg("--platform")
        .arg(platform)
        .current_dir(&scrap_dir)
        .spawn()
        .expect("Failed to start dioxus server");

    command.kill().expect("Failed to wait for dioxus server");
    let text_value = std::fs::read_to_string(scrap_dir.join("features.txt"))?;
    let features = text_value.lines().map(|line| line.to_string()).collect();

    Ok(features)
}

fn get_features_enabled_for_platform(
    installed: bool,
    platform: &str,
    scrap_dir: &Path,
    port: u16,
) -> anyhow::Result<Vec<String>> {
    if platform == "web" {
        test_web(installed, platform, scrap_dir, port)
    } else {
        test_desktop(installed, platform, scrap_dir)
    }
}

fn main() {
    let dx_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("cli");
    let scrap_dir = dx_path.join("testing");
    create_dir_all(&scrap_dir).unwrap();
    for platform in PLATFORMS {
        let old_enabled_features =
            get_features_enabled_for_platform(true, platform, &scrap_dir, 8080).unwrap();
        let new_enabled_features =
            get_features_enabled_for_platform(false, platform, &scrap_dir, 8080).unwrap();

        assert_eq!(
            old_enabled_features, new_enabled_features,
            "Features do not match for platform {platform}"
        );
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
