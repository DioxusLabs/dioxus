use std::{env, path::PathBuf};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum BuildError {
    #[error("Failed to find android.jar")]
    AndroidJarNotFound,
    #[error("Failed to find d8.jar")]
    D8JarNotFound,
    #[error("Java compilation failed")]
    JavaCompilationFailed,
    #[error("DEX compilation failed")]
    DexCompilationFailed,
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Compile Java source files to DEX bytecode
///
/// This function handles the complete Java→DEX compilation pipeline:
/// 1. Compile .java files to .class files using javac
/// 2. Compile .class files to .dex using d8
///
/// # Arguments
///
/// * `java_files` - List of Java source files to compile
/// * `package_name` - The package name for the generated classes
///
/// # Returns
///
/// Returns `Ok(())` if compilation succeeds, or a `BuildError` if it fails
///
/// # Example
///
/// ```rust,no_run
/// use dioxus_mobile_core::build::compile_java_to_dex;
/// use std::path::PathBuf;
///
/// let java_files = vec![PathBuf::from("src/LocationCallback.java")];
/// compile_java_to_dex(&java_files, "dioxus.mobile.geolocation")?;
/// ```
#[cfg(target_os = "android")]
pub fn compile_java_to_dex(java_files: &[PathBuf], package_name: &str) -> Result<(), BuildError> {
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR not set"));

    // Mark Java files as dependencies
    for java_file in java_files {
        println!("cargo:rerun-if-changed={}", java_file.display());
    }

    let android_jar_path =
        android_build::android_jar(None).map_err(|_| BuildError::AndroidJarNotFound)?;

    // Compile .java -> .class
    let compilation_success = android_build::JavaBuild::new()
        .class_path(android_jar_path.clone())
        .classes_out_dir(out_dir.clone())
        .files(java_files)
        .compile()
        .map_err(|_| BuildError::JavaCompilationFailed)?
        .success();

    if !compilation_success {
        return Err(BuildError::JavaCompilationFailed);
    }

    // Find the compiled class file
    let package_path = package_name.replace('.', "/");
    let class_file = out_dir.join(&package_path).join("LocationCallback.class");

    let d8_jar_path = android_build::android_d8_jar(None).map_err(|_| BuildError::D8JarNotFound)?;

    // Compile .class -> .dex
    let dex_success = android_build::JavaRun::new()
        .class_path(d8_jar_path)
        .main_class("com.android.tools.r8.D8")
        .arg("--classpath")
        .arg(android_jar_path)
        .arg("--output")
        .arg(&out_dir)
        .arg(&class_file)
        .run()
        .map_err(|_| BuildError::DexCompilationFailed)?
        .success();

    if !dex_success {
        return Err(BuildError::DexCompilationFailed);
    }

    Ok(())
}

#[cfg(not(target_os = "android"))]
pub fn compile_java_to_dex(_java_files: &[PathBuf], _package_name: &str) -> Result<(), BuildError> {
    // No-op for non-Android targets
    Ok(())
}

/// Link iOS frameworks
///
/// This function adds the necessary linker flags for iOS frameworks.
/// It should be called from build.rs for iOS targets.
///
/// # Arguments
///
/// * `frameworks` - List of framework names to link
///
/// # Example
///
/// ```rust,no_run
/// use dioxus_mobile_core::build::link_ios_frameworks;
///
/// link_ios_frameworks(&["CoreLocation", "Foundation"]);
/// ```
#[cfg(target_os = "ios")]
pub fn link_ios_frameworks(frameworks: &[&str]) {
    for framework in frameworks {
        println!("cargo:rustc-link-lib=framework={}", framework);
    }
}

#[cfg(not(target_os = "ios"))]
pub fn link_ios_frameworks(_frameworks: &[&str]) {
    // No-op for non-iOS targets
}

/// Auto-detect target OS and run appropriate build steps
///
/// This function automatically detects the target OS and runs the
/// appropriate build steps. It's a convenience function for build.rs.
///
/// # Arguments
///
/// * `java_files` - Java files to compile (only used for Android)
/// * `package_name` - Package name for Java compilation
/// * `ios_frameworks` - iOS frameworks to link (only used for iOS)
pub fn auto_build(
    java_files: &[PathBuf],
    package_name: &str,
    ios_frameworks: &[&str],
) -> Result<(), BuildError> {
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();

    match target_os.as_str() {
        "android" => {
            compile_java_to_dex(java_files, package_name)?;
        }
        "ios" => {
            link_ios_frameworks(ios_frameworks);
        }
        _ => {
            // No platform-specific build needed for other targets
            println!(
                "cargo:warning=Skipping platform shims for target_os={}",
                target_os
            );
        }
    }

    Ok(())
}
