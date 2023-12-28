use std::path::PathBuf;

use dioxus_cli_plugin::*;
use exports::plugins::main::definitions::Guest;
use plugins::main::{
    imports::{get_data, get_project_info, log, set_data, watched_paths},
    toml::{Toml, TomlValue},
    types::{CompileEvent, PluginInfo, ResponseEvent, RuntimeEvent},
};

struct Plugin;

impl Guest for Plugin {
    fn apply_config(config: Toml) -> Result<(), ()> {
        log(&format!("{:?}", config.get()));
        Ok(())
    }

    fn get_default_config() -> Toml {
        log("Starting to make default config from plugin!");
        let tomls: Vec<Toml> = (0..10).map(TomlValue::Integer).map(Toml::new).collect();
        let res = Toml::new(TomlValue::Array(tomls));
        log("Got a default config from plugin!");
        res
    }

    fn on_watched_paths_change(
        paths: std::vec::Vec<std::string::String>,
    ) -> Result<ResponseEvent, ()> {
        if !paths.iter().any(|f| f.ends_with(".rs")) {
            log("Skipping tailwind reload, no change necessary");
            return Ok(ResponseEvent::None);
        };

        // Not necessary, just for testing
        let Some(tailwind_output) = get_data("tailwind_output").map(|data| {
            let path = String::from_utf8(data).unwrap();
            PathBuf::from(path)
        }) else {
            log("Tailwind Plugin not registered!");
            return Err(());
        };

        // Todo make this work
        match std::process::Command::new("npx")
            .args([
                "tailwindcss",
                "-i",
                "INPUT_CSS",
                "-o",
                tailwind_output.to_str().unwrap(),
            ])
            .output()
        {
            Ok(text) => {
                log(std::str::from_utf8(&text.stdout).expect("Invalid command output!"));
                Ok(ResponseEvent::Refresh(vec![tailwind_output
                    .to_str()
                    .unwrap()
                    .to_string()]))
            }
            Err(err) => {
                let err_text = format!("Tailwind err: {err}");
                log(&err_text);
                Err(())
            }
        }
    }

    fn register() -> Result<(), ()> {
        log(&format!("{:?}", watched_paths()));

        let project_info = get_project_info();

        let tailwind_path =
            std::path::PathBuf::from(project_info.asset_directory).join("tailwind.css");
        set_data(
            "tailwind_output",
            tailwind_path.as_os_str().as_encoded_bytes(),
        );

        log("Registered Tailwind Plugin Successfully!");
        Ok(())
    }

    fn metadata() -> exports::plugins::main::definitions::PluginInfo {
        PluginInfo {
            name: "TestPlugin".into(),
            version: "0.0.1".into(),
        }
    }

    fn before_compile_event(event: CompileEvent) -> Result<(), ()> {
        log(&format!("Got before event in plugin: {event:?}"));
        Ok(())
    }
    fn before_runtime_event(event: RuntimeEvent) -> Result<ResponseEvent, ()> {
        log(&format!("Got before event in plugin: {event:?}"));
        Ok(ResponseEvent::None)
    }

    fn after_compile_event(event: CompileEvent) -> Result<(), ()> {
        log(&format!("Got after event in plugin: {event:?}"));
        Ok(())
    }

    fn after_runtime_event(event: RuntimeEvent) -> Result<ResponseEvent, ()> {
        log(&format!("Got after event in plugin: {event:?}"));
        Ok(ResponseEvent::None)
    }
}

export_plugin!(Plugin);
