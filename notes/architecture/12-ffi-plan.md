Native FFI Macro Implementation Plan
Summary
Implement #[manganis::ffi] macro that:
Declares native library folders (Gradle/SwiftPM) to bundle
Auto-generates direct JNI/ObjC FFI bindings from extern block declarations
CLI compiles the native code during build (no build.rs needed)
Target Syntax

#[manganis::ffi("/src/android")]  // Gradle library folder
extern "Kotlin" {
    pub type GeolocationPlugin;
    pub fn get_position(this: &GeolocationPlugin, high_accuracy: bool) -> Option<String>;
}

#[manganis::ffi("/src/ios")]  // SwiftPM package folder
extern "Swift" {
    pub type GeolocationPlugin;
    pub fn get_position(this: &GeolocationPlugin, high_accuracy: bool) -> Option<String>;
}
Phase 1: Fix Compile Errors
packages/cli/src/build/permissions.rs:12 - Change use permissions::Platform to use manganis_core::Platform
packages/cli/src/build/permissions.rs:16 - Change permissions::PermissionManifest to manganis_core::PermissionManifest
packages/cli/src/build/request.rs:1440-1442 - Change permissions::Platform::* to manganis_core::Platform::*
packages/manganis/manganis-macro/src/platform_bridge/ios_plugin.rs:115 - Change android::macro_helpers::copy_bytes to darwin::macro_helpers::copy_bytes
Phase 2: Macro Implementation
Step 1: Create FFI Macro Parser
Create packages/manganis/manganis-macro/src/platform_bridge/ffi_bridge.rs:

pub struct FfiBridgeParser {
    source_path: String,        // Folder path ("/src/android")
    abi: ForeignAbi,            // Swift or Kotlin
    types: Vec<ForeignTypeDecl>,
    functions: Vec<ForeignFunctionDecl>,
}

pub enum ForeignAbi { Swift, Kotlin }
Step 2: Register Macro
In packages/manganis/manganis-macro/src/lib.rs:

#[proc_macro_attribute]
pub fn ffi(attr: TokenStream, item: TokenStream) -> TokenStream {
    // Parse attribute for source path
    // Parse extern block for ABI and declarations
    // Generate platform-specific code + linker metadata
}
Step 3: Emit Linker Metadata
Use existing manganis primitives (like android_plugin!/ios_plugin! do):
For Kotlin: Emit AndroidArtifactMetadata with source folder path
For Swift: Emit SwiftPackageMetadata with package path and product name
Note: Modify metadata types to store SOURCE folder path (for CLI to compile) rather than artifact path (for prebuilt AAR).
Step 4: Generate FFI Bindings
Android (JNI):

pub struct GeolocationPlugin {
    inner: jni::objects::GlobalRef,
}

pub fn get_position(this: &GeolocationPlugin, high_accuracy: bool) -> Result<Option<String>, FfiError> {
    dioxus_platform_bridge::android::with_activity(|env, _| {
        env.call_method(this.inner.as_obj(), "getPosition", "(Z)Ljava/lang/String;",
            &[JValue::Bool(if high_accuracy { 1 } else { 0 })])
        // ... convert result
    })
}
iOS (msg_send!):

pub struct GeolocationPlugin {
    inner: *mut objc2::runtime::Object,
}

pub fn get_position(this: &GeolocationPlugin, high_accuracy: bool) -> Result<Option<String>, FfiError> {
    let mtm = MainThreadMarker::new().ok_or(FfiError::NotOnMainThread)?;
    unsafe {
        let result: *mut Object = msg_send![this.inner, getPositionWithHighAccuracy: Bool::new(high_accuracy)];
        // ... convert NSString
    }
}
Phase 3: CLI Integration
Step 1: Extract Metadata After Rust Compilation
In packages/cli/src/build/assets.rs, the existing extract_symbols_from_file() already extracts:
AndroidArtifactMetadata → android_artifacts
SwiftPackageMetadata → swift_packages
Step 2: Compile Kotlin/Android (NEW)
In packages/cli/src/build/request.rs, add compile_android_sources():

fn compile_android_sources(&self, android_artifacts: &[AndroidArtifactMetadata]) -> Result<()> {
    for artifact in android_artifacts {
        let source_dir = PathBuf::from(artifact.source_path.as_str());  // e.g., /path/to/src/android

        // 1. Run Gradle to build the library
        let gradle_cmd = resolve_gradle_command(&source_dir)?;
        Command::new(&gradle_cmd)
            .arg("assembleRelease")
            .current_dir(&source_dir)
            .env("JAVA_HOME", ...)
            .env("ANDROID_SDK_ROOT", ...)
            .status()?;

        // 2. Find the output AAR
        let aar_path = discover_release_aar(&source_dir)?;

        // 3. Copy AAR to app/libs/
        let libs_dir = self.root_dir().join("app").join("libs");
        fs::create_dir_all(&libs_dir)?;
        fs::copy(&aar_path, libs_dir.join(aar_path.file_name().unwrap()))?;

        // 4. Add to build.gradle.kts dependencies
        self.ensure_gradle_dependency(&build_gradle, &format!(
            "implementation(files(\"libs/{}\"))", aar_path.file_name().unwrap().to_string_lossy()
        ))?;

        // 5. Add any extra Gradle dependencies
        for dep in artifact.gradle_dependencies.as_str().lines() {
            self.ensure_gradle_dependency(&build_gradle, dep)?;
        }
    }
}
Step 3: Compile Swift/iOS (NEW)
In packages/cli/src/build/request.rs, add compile_swift_sources():

fn compile_swift_sources(&self, swift_packages: &[SwiftPackageMetadata]) -> Result<()> {
    for package in swift_packages {
        let package_dir = PathBuf::from(package.package_path.as_str());
        let product = package.product.as_str();

        // 1. Determine Swift target triple
        let (swift_target, sdk_name) = swift_target_and_sdk(&self.triple.to_string())?;
        let sdk_path = lookup_sdk_path(sdk_name)?;

        // 2. Run swift build
        let build_dir = self.target_dir().join("swift-build");
        Command::new("xcrun")
            .args(["swift", "build", "--package-path", package_dir.to_str().unwrap()])
            .args(["--configuration", self.profile_name()])
            .args(["--triple", &swift_target])
            .args(["--sdk", &sdk_path])
            .args(["--product", product])
            .args(["--build-path", build_dir.to_str().unwrap()])
            .status()?;

        // 3. Find the static library
        let lib_path = find_static_lib(&build_dir, product)?;

        // 4. Link the library (add to linker args)
        // The library gets linked automatically via the FFI extern declarations
    }
}
Step 4: Hook Into Build Pipeline
In packages/cli/src/build/request.rs, call these after collect_assets_and_permissions():

// After extracting metadata from binary
let (assets, permissions, android_artifacts, swift_packages) = self.collect_assets_and_permissions().await?;

// NEW: Compile native sources
if self.bundle == BundleFormat::Android && !android_artifacts.is_empty() {
    self.compile_android_sources(&android_artifacts)?;
}
if matches!(self.bundle, BundleFormat::Ios | BundleFormat::MacOS) && !swift_packages.is_empty() {
    self.compile_swift_sources(&swift_packages)?;
}
Files to Modify
File	Change
packages/cli/src/build/permissions.rs	Fix imports (manganis_core::Platform)
packages/cli/src/build/request.rs	Fix imports + add compile_android_sources() + compile_swift_sources()
packages/manganis/manganis-macro/src/platform_bridge/ios_plugin.rs:115	Fix copy_bytes path
packages/manganis/manganis-macro/src/platform_bridge/mod.rs	Add ffi_bridge module
packages/manganis/manganis-macro/src/lib.rs	Add #[proc_macro_attribute] pub fn ffi
packages/manganis/manganis/src/lib.rs	Export FfiError
packages/manganis/manganis-core/src/permissions.rs	Update metadata types if needed for source paths
packages/cli/assets/android/gen/app/build.gradle.kts.hbs	Add placeholder for plugin dependencies
Files to Create
File	Purpose
packages/manganis/manganis-macro/src/platform_bridge/ffi_bridge.rs	Macro parser & FFI codegen
packages/manganis/manganis/src/ffi_error.rs	FfiError type
Type Support (Phase 1)
Supported:
Primitives: bool, i8-i64, u8-u64, f32, f64
Strings: &str, String
Options: Option<T>
Opaque refs: &ForeignType
JNI Type Mapping:
Rust	JNI Sig	Kotlin
bool	Z	Boolean
i32	I	Int
i64	J	Long
String	Ljava/lang/String;	String
ObjC Type Mapping:
Rust	ObjC	Swift
bool	BOOL	Bool
i32	int32_t	Int32
String	NSString*	String
Implementation Order
Fix compile errors (Phase 1)
Create ffi_bridge.rs with parser
Implement Android codegen (JNI)
Implement iOS codegen (msg_send!)
Register macro in lib.rs
Add compile_android_sources() to CLI
Add compile_swift_sources() to CLI
Update geolocation example to use #[ffi] macro
