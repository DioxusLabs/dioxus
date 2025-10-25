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

/// Build the iOS shim using Objective-C (simpler than Swift)
fn build_ios() {
    println!("cargo:rerun-if-changed=ios-shim/Sources");
    println!("cargo:rerun-if-changed=ios-shim/include");

    let out_dir = env::var("OUT_DIR").expect("OUT_DIR not set");
    let target_triple = env::var("TARGET").unwrap_or_default();

    println!(
        "cargo:warning=Building iOS shim for target: {}",
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

    // Create a simple Objective-C implementation
    let objc_file = PathBuf::from(&out_dir).join("GeolocationShim.m");
    let obj_file = PathBuf::from(&out_dir).join("GeolocationShim.o");
    let output_lib = PathBuf::from(&out_dir).join("libGeolocationShim.a");
    
    // Write the Objective-C implementation
    let objc_code = r#"
#import <CoreLocation/CoreLocation.h>
#import <Foundation/Foundation.h>

// Global location manager instance
static CLLocationManager* g_locationManager = nil;

// Initialize the location manager
void ios_geoloc_init() {
    if (g_locationManager == nil) {
        g_locationManager = [[CLLocationManager alloc] init];
    }
}

// Get the last known location
double* ios_geoloc_last_known() {
    ios_geoloc_init();
    
    CLLocation* location = [g_locationManager location];
    if (location == nil) {
        return NULL;
    }
    
    double* result = malloc(2 * sizeof(double));
    if (result == NULL) {
        return NULL;
    }
    
    result[0] = location.coordinate.latitude;
    result[1] = location.coordinate.longitude;
    
    return result;
}

// Request location authorization
void ios_geoloc_request_authorization() {
    ios_geoloc_init();
    [g_locationManager requestWhenInUseAuthorization];
}

// Check if location services are enabled
int32_t ios_geoloc_services_enabled() {
    return [CLLocationManager locationServicesEnabled] ? 1 : 0;
}

// Get authorization status
int32_t ios_geoloc_authorization_status() {
    ios_geoloc_init();
    CLAuthorizationStatus status = [g_locationManager authorizationStatus];
    switch (status) {
        case kCLAuthorizationStatusNotDetermined:
            return 0;
        case kCLAuthorizationStatusRestricted:
            return 1;
        case kCLAuthorizationStatusDenied:
            return 2;
        case kCLAuthorizationStatusAuthorizedAlways:
            return 3;
        case kCLAuthorizationStatusAuthorizedWhenInUse:
            return 4;
        default:
            return 0;
    }
}
"#;

    // Write the Objective-C file
    if let Err(e) = std::fs::write(&objc_file, objc_code) {
        println!("cargo:warning=Failed to write Objective-C file: {}", e);
        return;
    }

    // Get the SDK path first
    let sdk_path = std::process::Command::new("xcrun")
        .args(&["--sdk", sdk, "--show-sdk-path"])
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                String::from_utf8(output.stdout).ok()
                    .map(|s| s.trim().to_string())
            } else {
                None
            }
        })
        .unwrap_or_else(|| {
            println!("cargo:warning=Failed to get SDK path, using default");
            "/Applications/Xcode.app/Contents/Developer/Platforms/iPhoneOS.platform/Developer/SDKs/iPhoneOS.sdk".to_string()
        });

    // Compile the Objective-C file
    let mut cmd = std::process::Command::new("clang");
    cmd.args(&[
        "-c",
        "-o", obj_file.to_str().unwrap(),
        "-arch", if is_simulator { "arm64" } else { "arm64" },
        "-isysroot", &sdk_path,
        "-fobjc-arc",
        "-framework", "CoreLocation",
        "-framework", "Foundation",
        objc_file.to_str().unwrap()
    ]);

    println!("cargo:warning=Running: {:?}", cmd);
    
    let status = cmd.status();

    match status {
        Ok(s) if s.success() => {
            println!("cargo:warning=Objective-C compilation succeeded");
            
            // Create static library from object file
            let mut ar_cmd = std::process::Command::new("ar");
            ar_cmd.args(&[
                "rcs",
                output_lib.to_str().unwrap(),
                obj_file.to_str().unwrap()
            ]);
            
            match ar_cmd.status() {
                Ok(ar_status) if ar_status.success() => {
                    println!("cargo:warning=Static library created successfully");
                    println!("cargo:rustc-link-search=native={}", out_dir);
                }
                Ok(ar_status) => {
                    println!("cargo:warning=ar failed with status: {}", ar_status);
                }
                Err(e) => {
                    println!("cargo:warning=Failed to run ar: {}", e);
                }
            }
        }
        Ok(s) => {
            println!("cargo:warning=Objective-C compilation failed with status: {}", s);
            println!(
                "cargo:warning=Continuing without iOS shim (iOS functionality will not work)"
            );
        }
        Err(e) => {
            println!("cargo:warning=Failed to execute clang: {}", e);
            println!("cargo:warning=Make sure Xcode command line tools are installed");
            println!(
                "cargo:warning=Continuing without iOS shim (iOS functionality will not work)"
            );
        }
    }

    // Only link frameworks/libraries if the shim was built successfully
    if output_lib.exists() {
        println!("cargo:rustc-link-lib=framework=CoreLocation");
        println!("cargo:rustc-link-lib=framework=Foundation");
        println!("cargo:rustc-link-lib=static=GeolocationShim");
    } else {
        println!("cargo:warning=Skipping iOS framework linking (shim not built)");
    }
}
