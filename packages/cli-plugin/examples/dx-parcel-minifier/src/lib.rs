use std::fs;

use dioxus_cli_plugin::*;
use exports::plugins::main::definitions::Guest;
use plugins::main::{
    imports::log,
    toml::{Toml, TomlValue},
    types::{CommandEvent, PluginInfo, Response, RuntimeEvent},
};

struct CSSMinifer;

fn minify_css() -> Result<Response, ()> {
    for entry in fs::read_dir("/dist").unwrap() {
        let entry = entry.unwrap();
        if !entry.file_name().as_encoded_bytes().ends_with(b".css") {
            continue;
        }
        let path = entry.path();
        log(&format!("Found {}", path.display()));
        let Ok(file) = fs::OpenOptions::new().read(true).write(true).open(&path) else {
            log(&format!(
                "Could not open file: {}, skipping..",
                path.display()
            ));
            continue;
        };

        let css_contents = match std::io::read_to_string(&file) {
            Ok(css) => css,
            Err(err) => {
                log(&format!(
                    "Could not read file to string: {} : {}, skipping..",
                    path.display(),
                    err
                ));
                continue;
            }
        };

        let minified_content = match minifier::css::minify(&css_contents) {
            Ok(minified_content) => minified_content,
            Err(err) => {
                log(err);
                continue;
            }
        };

        if let Err(err) = minified_content.write(file) {
            log(&err.to_string());
        };
    }

    Ok(Response::None)
}

impl Guest for CSSMinifer {
    fn get_default_config() -> Toml {
        Toml::new(TomlValue::Table(Vec::new()))
    }

    fn apply_config(_: Toml) -> Result<(), ()> {
        Ok(())
    }

    fn register() -> Result<(), ()> {
        Ok(())
    }

    fn metadata() -> PluginInfo {
        PluginInfo {
            name: "DX Parcel Minifier".to_string(),
            version: "0.0.1".to_string(),
        }
    }

    fn before_command_event(_event: CommandEvent) -> Result<(), ()> {
        Ok(())
    }
    fn before_runtime_event(_event: RuntimeEvent) -> Result<Response, ()> {
        Ok(Response::None)
    }
    fn after_command_event(_event: CommandEvent) -> Result<(), ()> {
        minify_css()?;
        Ok(())
    }
    fn after_runtime_event(_event: RuntimeEvent) -> Result<Response, ()> {
        minify_css()
    }
    fn on_watched_paths_change(
        _path: wit_bindgen::rt::vec::Vec<wit_bindgen::rt::string::String>,
    ) -> Result<Response, ()> {
        minify_css()
    }
}

export!(CSSMinifer);
