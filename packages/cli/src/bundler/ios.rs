use serde_json::Value;
use std::error::Error;
use std::path::Path;
use tokio::process::Command;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // 1. Install the app on the device
    let app_path = "target/aarch64-apple-ios-sim/debug/bundle/ios/DioxusApp.app";
    install_app(app_path).await?;

    // 2. Determine which device the app was installed to
    let device_uuid = get_device_uuid().await?;

    // 3. Get the installation URL of the app
    let installation_url = get_installation_url(&device_uuid, app_path).await?;

    // 4. Launch the app into the background, paused
    launch_app_paused(&device_uuid, &installation_url).await?;

    // 5. Pick up the paused app and resume it
    resume_app(&device_uuid).await?;

    Ok(())
}

async fn install_app(app_path: &str) -> Result<(), Box<dyn Error>> {
    let output = Command::new("xcrun")
        .args(&["simctl", "install", "booted", app_path])
        .output()
        .await?;

    if !output.status.success() {
        return Err(format!("Failed to install app: {:?}", output).into());
    }

    Ok(())
}

async fn get_device_uuid() -> Result<String, Box<dyn Error>> {
    let output = Command::new("xcrun")
        .args(&[
            "devicectl",
            "list",
            "devices",
            "--json-output",
            "target/deviceid.json",
        ])
        .output()
        .await?;

    if !output.status.success() {
        return Err(format!("Failed to list devices: {:?}", output).into());
    }

    let json: Value = serde_json::from_str(&std::fs::read_to_string("target/deviceid.json")?)?;
    let device_uuid = json["result"]["devices"][0]["identifier"]
        .as_str()
        .ok_or("Failed to extract device UUID")?
        .to_string();

    Ok(device_uuid)
}

async fn get_installation_url(device_uuid: &str, app_path: &str) -> Result<String, Box<dyn Error>> {
    let output = Command::new("xcrun")
        .args(&[
            "devicectl",
            "device",
            "install",
            "app",
            "--device",
            device_uuid,
            app_path,
            "--json-output",
            "target/xcrun.json",
        ])
        .output()
        .await?;

    if !output.status.success() {
        return Err(format!("Failed to install app: {:?}", output).into());
    }

    let json: Value = serde_json::from_str(&std::fs::read_to_string("target/xcrun.json")?)?;
    let installation_url = json["result"]["installedApplications"][0]["installationURL"]
        .as_str()
        .ok_or("Failed to extract installation URL")?
        .to_string();

    Ok(installation_url)
}

async fn launch_app_paused(
    device_uuid: &str,
    installation_url: &str,
) -> Result<(), Box<dyn Error>> {
    let output = Command::new("xcrun")
        .args(&[
            "devicectl",
            "device",
            "process",
            "launch",
            "--no-activate",
            "--verbose",
            "--device",
            device_uuid,
            installation_url,
            "--json-output",
            "target/launch.json",
        ])
        .output()
        .await?;

    if !output.status.success() {
        return Err(format!("Failed to launch app: {:?}", output).into());
    }

    Ok(())
}

async fn resume_app(device_uuid: &str) -> Result<(), Box<dyn Error>> {
    let json: Value = serde_json::from_str(&std::fs::read_to_string("target/launch.json")?)?;
    let status_pid = json["result"]["process"]["processIdentifier"]
        .as_u64()
        .ok_or("Failed to extract process identifier")?;

    let output = Command::new("xcrun")
        .args(&[
            "devicectl",
            "device",
            "process",
            "resume",
            "--device",
            device_uuid,
            "--pid",
            &status_pid.to_string(),
        ])
        .output()
        .await?;

    if !output.status.success() {
        return Err(format!("Failed to resume app: {:?}", output).into());
    }

    Ok(())
}
