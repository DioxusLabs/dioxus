use manganis_cli_support::AssetManifestExt;
use std::{path::PathBuf, process::Command};

// This is the location where the assets will be copied to in the filesystem
const ASSETS_FILE_LOCATION: &str = "./assets";

// This is the location where the assets will be served from
const ASSETS_SERVE_LOCATION: &str = "./assets/";

fn main() {
    tracing_subscriber::fmt::init();

    // Handle the commands.
    let args: Vec<String> = std::env::args().collect();

    if let Some(arg) = args.get(1) {
        if arg == "link" {
            link();
            return;
        } else if arg == "build" {
            println!("Building!");
            build();
            return;
        }
    }

    println!("Unknown Command");
}

fn build() {
    // Build your application
    let current_dir = std::env::current_dir().unwrap();

    let args = ["--release"];
    Command::new("cargo")
        .current_dir(&current_dir)
        .arg("build")
        .args(args)
        .env("MG_BASEPATH", "/blah/")
        .spawn()
        .unwrap()
        .wait()
        .unwrap();

    // Call the helper function to intercept the Rust linker.
    // We will pass the current working directory as it may get lost.
    let work_dir = std::env::current_dir().unwrap();
    let link_args = vec![format!("{}", work_dir.display())];
    manganis_cli_support::start_linker_intercept("link", args, Some(link_args)).unwrap();
}

fn link() {
    let (link_args, object_files) =
        manganis_cli_support::linker_intercept(std::env::args()).unwrap();

    // // Extract the assets
    // let assets = AssetManifest::load_from_objects(object_files);

    // let working_dir = PathBuf::from(link_args.first().unwrap());
    // let assets_dir = working_dir.join(working_dir.join(ASSETS_FILE_LOCATION));

    // // Remove the old assets
    // let _ = std::fs::remove_dir_all(&assets_dir);

    // // And copy the static assets to the public directory
    // assets.copy_static_assets_to(&assets_dir).unwrap();

    // // Then collect the tailwind CSS
    // let css = assets.collect_tailwind_css(true, &mut Vec::new());

    // // And write the CSS to the public directory
    // let tailwind_path = assets_dir.join("tailwind.css");
    // std::fs::write(tailwind_path, css).unwrap();
}
