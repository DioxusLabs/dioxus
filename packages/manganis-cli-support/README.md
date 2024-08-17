# Manganis CLI Support

This crate provides utilities to collect assets that integrate with the Manganis macro. It makes it easy to integrate an asset collection and optimization system into a build tool.

```rust, no_run
use manganis_cli_support::{AssetManifestExt, ManganisSupportGuard};
use manganis_common::{AssetManifest, Config};
use std::process::Command;

// This is the location where the assets will be copied to in the filesystem
let assets_file_location = "./assets";
// This is the location where the assets will be served from
let assets_serve_location = "/assets";

// First set any settings you need for the build.
Config::default()
    .with_assets_serve_location(assets_serve_location)
    .save();

// Tell manganis that you support assets
let _guard = ManganisSupportGuard::default();

// Determine if Rust is trying to link:
if let Some((_working_dir, object_files)) = manganis_cli_support::linker_intercept(std::env::args()) {
    // If it is, collect the assets.
    let manifest = AssetManifest::load(object_files);

    // Remove the old assets
    let _ = std::fs::remove_dir_all(assets_file_location);

    // And copy the static assets to the public directory
    manifest
        .copy_static_assets_to(assets_file_location)
        .unwrap();

    // Then collect the tailwind CSS
    let css = manifest.collect_tailwind_css(true, &mut Vec::new());

    // And write the CSS to the public directory
    std::fs::write(format!("{}/tailwind.css", assets_file_location), css).unwrap();
    
} else {
    // If it isn't, build your app and initiate the helper function `start_linker_intercept()`

    // Put any cargo args in a slice that should also be passed 
    // to manganis toreproduce the same build. e.g. the `--release` flag
    let args: Vec<&str> = vec![];
    Command::new("cargo")
        .arg("build")
        .args(args.clone())
        .spawn()
        .unwrap()
        .wait()
        .unwrap();

    manganis_cli_support::start_linker_intercept(None, args).unwrap();
}
```
