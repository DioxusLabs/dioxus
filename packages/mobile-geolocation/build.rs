use std::env;
use std::path::PathBuf;

fn main() {
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();

    match target_os.as_str() {
        "android" => build_android(),
        "ios" => build_ios(),
        _ => {
            // No platform-specific build needed for other targets
            println!(
                "cargo:warning=Skipping platform shims for target_os={}",
                target_os
            );
        }
    }
}

/// Build the Android Java shim
fn build_android() {
    println!("cargo:warning=Android Java sources will be compiled by Gradle");
}

/// Build the iOS Swift shim using xcodebuild or swift build
fn build_ios() {
    println!("cargo:rerun-if-changed=ios-shim/Sources");
    println!("cargo:rerun-if-changed=ios-shim/Package.swift");

    let out_dir = env::var("OUT_DIR").expect("OUT_DIR not set");
    let target_triple = env::var("TARGET").unwrap_or_default();

    println!(
        "cargo:warning=Building iOS Swift shim for target: {}",
        target_triple
    );

    // Determine SDK based on target triple
    let is_simulator = target_triple.contains("sim");
    let sdk = if is_simulator {
        "iphonesimulator"
    } else {
        "iphoneos"
    };

    println!("cargo:warning=Detected SDK: {}", sdk);

    // Build with swift build for the appropriate platform
    let mut cmd = std::process::Command::new("swift");
    cmd.current_dir("ios-shim")
        .args(&["build", "-c", "release"]);

    // Set the destination platform
    let destination = if is_simulator {
        "generic/platform=iOS Simulator"
    } else {
        "generic/platform=iOS"
    };

    cmd.args(&["--build-path", ".build"])
        .env("DESTINATION", destination);

    let status = cmd.status();

    match status {
        Ok(s) if s.success() => {
            println!("cargo:warning=Swift build succeeded");

            // Find the built library
            let lib_path = PathBuf::from("ios-shim")
                .join(".build")
                .join("release")
                .join("libGeolocationShim.a");

            if lib_path.exists() {
                // Copy to OUT_DIR
                let out_lib = PathBuf::from(&out_dir).join("libGeolocationShim.a");
                std::fs::copy(&lib_path, &out_lib).expect("Failed to copy Swift library");
                println!(
                    "cargo:warning=Copied Swift library to: {}",
                    out_lib.display()
                );

                // Tell Cargo where to find the library
                println!("cargo:rustc-link-search=native={}", out_dir);
            } else {
                println!(
                    "cargo:warning=Swift library not found at: {}",
                    lib_path.display()
                );
            }
        }
        Ok(s) => {
            println!("cargo:warning=Swift build failed with status: {}", s);
            println!(
                "cargo:warning=Continuing without Swift shim (iOS functionality will not work)"
            );
        }
        Err(e) => {
            println!("cargo:warning=Failed to execute swift build: {}", e);
            println!("cargo:warning=Make sure Swift toolchain is installed");
            println!(
                "cargo:warning=Continuing without Swift shim (iOS functionality will not work)"
            );
        }
    }

    // Only link frameworks/libraries if the Swift shim was built successfully
    // This prevents linker errors when the Swift build fails
    if PathBuf::from(&out_dir)
        .join("libGeolocationShim.a")
        .exists()
    {
        println!("cargo:rustc-link-lib=framework=CoreLocation");
        println!("cargo:rustc-link-lib=framework=Foundation");
        println!("cargo:rustc-link-lib=static=GeolocationShim");
    } else {
        println!("cargo:warning=Skipping iOS framework linking (Swift shim not built)");
    }
}
