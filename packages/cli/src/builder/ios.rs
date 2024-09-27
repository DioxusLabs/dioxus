use serde_json::Value;
use std::error::Error;
use std::path::Path;
use tokio::process::Command;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // 1. Install the app on the device
    let app_path = "target/aarch64-apple-ios-sim/debug/bundle/ios/DioxusApp.app";

    Ok(())
}
