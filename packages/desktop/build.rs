use std::{io::Write as _, path::PathBuf};

fn main() {
    check_gnu();

    maybe_copy_doc_scrope();
}

fn check_gnu() {
    // WARN about wry support on windows gnu targets. GNU windows targets don't work well in wry currently
    if std::env::var("CARGO_CFG_WINDOWS").is_ok()
        && std::env::var("CARGO_CFG_TARGET_ENV").unwrap() == "gnu"
        && !cfg!(feature = "gnu")
    {
        println!("cargo:warning=GNU windows targets have some limitations within Wry. Using the MSVC windows toolchain is recommended. If you would like to use continue using GNU, you can read https://github.com/wravery/webview2-rs#cross-compilation and disable this warning by adding the gnu feature to dioxus-desktop in your Cargo.toml")
    }
}

// todo: maybe we don't want to do this in this build script
fn maybe_copy_doc_scrope() {
    // To prepare for a release, we add extra examples to desktop for doc scraping and copy assets from the workspace to make those examples compile
    if option_env!("DIOXUS_RELEASE").is_some() {
        // Append EXAMPLES_TOML to the cargo.toml
        let cargo_toml = std::fs::OpenOptions::new()
            .append(true)
            .open("Cargo.toml")
            .unwrap();
        let mut write = std::io::BufWriter::new(cargo_toml);
        write.write_all(EXAMPLES_TOML.as_bytes()).unwrap();

        // Copy the assets from the workspace to the examples directory
        let crate_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
        let workspace_dir = crate_dir.parent().unwrap().parent().unwrap();
        let workspace_assets_dir = workspace_dir.join("examples").join("assets");
        let desktop_assets_dir = PathBuf::from("examples").join("assets");
        std::fs::create_dir_all(&desktop_assets_dir).unwrap();
        // move all files from the workspace assets dir to the desktop assets dir
        for entry in std::fs::read_dir(workspace_assets_dir).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.is_file() {
                std::fs::copy(&path, desktop_assets_dir.join(path.file_name().unwrap())).unwrap();
            }
        }
    }
}

const EXAMPLES_TOML: &str = r#"
# Most of the examples live in the workspace. We include some here so that docs.rs can scrape our examples for better inline docs
[[example]]
name = "video_stream"
path = "../../examples/video_stream.rs"
doc-scrape-examples = true

[[example]]
name = "suspense"
path = "../../examples/suspense.rs"
doc-scrape-examples = true

[[example]]
name = "calculator_mutable"
path = "../../examples/calculator_mutable.rs"
doc-scrape-examples = true

[[example]]
name = "custom_html"
path = "../../examples/custom_html.rs"
doc-scrape-examples = true

[[example]]
name = "custom_menu"
path = "../../examples/custom_menu.rs"
doc-scrape-examples = true

[[example]]
name = "errors"
path = "../../examples/errors.rs"
doc-scrape-examples = true

[[example]]
name = "file_explorer"
path = "../../examples/file_explorer.rs"
doc-scrape-examples = true

[[example]]
name = "future"
path = "../../examples/future.rs"
doc-scrape-examples = true

[[example]]
name = "hydration"
path = "../../examples/hydration.rs"
doc-scrape-examples = true

[[example]]
name = "multiwindow"
path = "../../examples/multiwindow.rs"
doc-scrape-examples = true

[[example]]
name = "overlay"
path = "../../examples/overlay.rs"
doc-scrape-examples = true

[[example]]
name = "popup"
path = "../../examples/popup.rs"
doc-scrape-examples = true

[[example]]
name = "read_size"
path = "../../examples/read_size.rs"
doc-scrape-examples = true

[[example]]
name = "shortcut"
path = "../../examples/shortcut.rs"
doc-scrape-examples = true

[[example]]
name = "streams"
path = "../../examples/streams.rs"
doc-scrape-examples = true

[[example]]
name = "window_event"
path = "../../examples/window_event.rs"
doc-scrape-examples = true

[[example]]
name = "window_focus"
path = "../../examples/window_focus.rs"
doc-scrape-examples = true

[[example]]
name = "window_zoom"
path = "../../examples/window_zoom.rs"
doc-scrape-examples = true"#;
