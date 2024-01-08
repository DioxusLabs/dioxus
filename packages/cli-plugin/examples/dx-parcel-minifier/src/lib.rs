use std::fs;

use dioxus_cli_plugin::*;
use exports::plugins::main::definitions::Guest;
use plugins::main::{
    imports::{get_project_info, log},
    toml::{Toml, TomlValue},
    types::{CommandEvent, PluginInfo, ResponseEvent, RuntimeEvent},
};

struct CSSMinifer;

fn minify_css() -> Result<ResponseEvent, ()> {
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

    Ok(ResponseEvent::None)
}

impl Guest for CSSMinifer {
    #[doc = " Get the default layout for the plugin to put"]
    #[doc = " into `Dioxus.toml`"]
    fn get_default_config() -> Toml {
        Toml::new(TomlValue::Integer(0))
    }

    #[doc = " Take config from `Dioxus.toml` and apply"]
    #[doc = " to the plugin, returns false if couldn\\'t apply"]
    fn apply_config(_config: Toml) -> Result<(), ()> {
        Ok(())
    }

    #[doc = " Initialize the plugin. This will be called once after the plugin is added"]
    fn register() -> Result<(), ()> {
        if !get_project_info().has_output_directory {
            log("No output directory detected, minifier won't find anything!");
        }
        Ok(())
    }

    #[doc = " Get the metadata of the plugin"]
    fn metadata() -> PluginInfo {
        PluginInfo {
            name: "DX Parcel Minifier".to_string(),
            version: "0.0.1".to_string(),
        }
    }

    #[doc = " Called right before the event given"]
    #[doc = " These are the compile-time functions like `Build`, `Translate`, etc"]
    fn before_command_event(_event: CommandEvent) -> Result<(), ()> {
        Ok(())
    }

    #[doc = " Called right before the event given"]
    #[doc = " These are the runtime-functions like `HotReload` and `Serve`"]
    fn before_runtime_event(_event: RuntimeEvent) -> Result<ResponseEvent, ()> {
        Ok(ResponseEvent::None)
    }

    #[doc = " Called right after the event given"]
    fn after_command_event(_event: CommandEvent) -> Result<(), ()> {
        minify_css()?;
        Ok(())
    }

    #[doc = " Called right after the event given"]
    fn after_runtime_event(_event: RuntimeEvent) -> Result<ResponseEvent, ()> {
        minify_css()
    }

    #[doc = " Gives a list of the paths that have been changed"]
    fn on_watched_paths_change(
        _path: wit_bindgen::rt::vec::Vec<wit_bindgen::rt::string::String>,
    ) -> Result<ResponseEvent, ()> {
        minify_css()
    }
}

export_plugin!(CSSMinifer);
