use dioxus_mobile_core::build::auto_build;
use std::path::PathBuf;

fn main() {
    let java_files = vec![PathBuf::from("src/sys/android/LocationCallback.java")];

    if let Err(e) = auto_build(
        &java_files,
        "dioxus.mobile.geolocation",
        &["CoreLocation", "Foundation"],
    ) {
        eprintln!("Build error: {}", e);
        std::process::exit(1);
    }
}
