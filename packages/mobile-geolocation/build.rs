use std::{env, path::PathBuf};

const JAVA_FILE_RELATIVE_PATH: &str = "src/sys/android/LocationCallback.java";

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

/// Build the Android Java source into DEX bytecode
fn build_android() {
    println!("cargo:rerun-if-changed={JAVA_FILE_RELATIVE_PATH}");

    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR not set"));
    let java_file =
        PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap()).join(JAVA_FILE_RELATIVE_PATH);

    let android_jar_path = android_build::android_jar(None).expect("Failed to find android.jar");

    // Compile .java -> .class
    assert!(
        android_build::JavaBuild::new()
            .class_path(android_jar_path.clone())
            .classes_out_dir(out_dir.clone())
            .file(java_file)
            .compile()
            .expect("Failed to get javac exit status")
            .success(),
        "javac invocation failed"
    );

    let class_file = out_dir
        .join("dioxus")
        .join("mobile")
        .join("geolocation")
        .join("LocationCallback.class");

    let d8_jar_path = android_build::android_d8_jar(None).expect("Failed to find d8.jar");

    // Compile .class -> .dex
    assert!(
        android_build::JavaRun::new()
            .class_path(d8_jar_path)
            .main_class("com.android.tools.r8.D8")
            .arg("--classpath")
            .arg(android_jar_path)
            .arg("--output")
            .arg(&out_dir)
            .arg(&class_file)
            .run()
            .expect("Failed to get d8.jar exit status")
            .success(),
        "d8.jar invocation failed"
    );
}

/// Build for iOS - objc2 handles everything, no compilation needed
fn build_ios() {
    println!("cargo:rustc-link-lib=framework=CoreLocation");
    println!("cargo:rustc-link-lib=framework=Foundation");
}
