use manganis_cli_support::AssetManifestExt;
use manganis_common::AssetType;
use std::path::PathBuf;
use std::process::{Command, Stdio};

// #[test]
// fn collects_assets() {
//     tracing_subscriber::fmt::init();

//     // Get args and default to "build"
//     let args: Vec<String> = std::env::args().collect();
//     let command = match args.get(1) {
//         Some(a) => a.clone(),
//         None => "build".to_string(),
//     };

//     // Check if rustc is trying to link
//     if command == "link" {
//         link();
//     } else {
//         build();
//     }
// }

// fn build() {
//     // Find the test package directory which is up one directory from this package
//     let mut test_package_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
//         .parent()
//         .unwrap()
//         .to_path_buf();
//     test_package_dir.push("test-package");

//     println!("running the CLI from {test_package_dir:?}");

//     // Then build your application
//     let args = ["--target", "wasm32-unknown-unknown", "--release"];
//     Command::new("cargo")
//         .arg("build")
//         .args(args)
//         .current_dir(&test_package_dir)
//         .stdout(Stdio::piped())
//         .spawn()
//         .unwrap()
//         .wait()
//         .unwrap();

//     println!("Collecting Assets");

//     // Call the helper function to intercept the Rust linker.
//     // We will pass the current working directory as it may get lost.
//     let link_args = vec![format!("{}", test_package_dir.display())];
//     manganis_cli_support::start_linker_intercept("link", args, Some(link_args)).unwrap();
// }

// fn link() {
//     let (link_args, object_files) =
//         manganis_cli_support::linker_intercept(std::env::args()).unwrap();

//     // Recover the working directory from the link args.
//     let working_dir = PathBuf::from(link_args.first().unwrap());

//     // Then collect the assets
//     let assets = AssetManifest::load_from_objects(object_files);

//     let all_assets = assets.assets();
//     println!("{:#?}", all_assets);

//     let locations = all_assets
//         .iter()
//         .filter_map(|a| match a {
//             AssetType::Resource(f) => Some(f.location()),
//             _ => None,
//         })
//         .collect::<Vec<_>>();

//     // Make sure the right number of assets were collected
//     assert_eq!(locations.len(), 16);

//     // Then copy the assets to a temporary directory and run the application
//     let assets_dir = PathBuf::from("./assets");
//     assets.copy_static_assets_to(assets_dir).unwrap();

//     // Then run the application
//     let status = Command::new("cargo")
//         .arg("run")
//         .arg("--release")
//         .current_dir(&working_dir)
//         .status()
//         .unwrap();

//     // Make sure the application exited successfully
//     assert!(status.success());
// }
