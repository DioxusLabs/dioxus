use std::{fs::File, io::Write, path::PathBuf};

use dioxus_cli_plugin::*;
use exports::plugins::main::definitions::Guest;
use plugins::main::{
    imports::{get_data, log, set_data, watched_paths},
    toml::{Toml, TomlValue},
    types::{CompileEvent, PluginInfo, ResponseEvent, RuntimeEvent},
};
use railwind::{parse_to_string, SourceOptions};

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
        let paths: Vec<PathBuf> = paths
            .into_iter()
            .filter_map(|f| f.ends_with(".rs").then(|| PathBuf::from(f)))
            .collect();

        if paths.is_empty() {
            log("Skipping tailwind reload, no change necessary");
            return Ok(ResponseEvent::None);
        };

        let sources: Vec<_> = paths
            .iter()
            .map(|input| SourceOptions {
                input,
                option: railwind::CollectionOptions::Html,
            })
            .collect();

        // Not necessary, just for testing
        let Some(tailwind_output) =
            get_data("tailwind_output").map(|f| String::from_utf8(f).unwrap())
        else {
            log("Tailwind Plugin not registered!");
            return Err(());
        };
        log(&tailwind_output);

        let mut warnings = vec![];
        let parsed = parse_to_string(railwind::Source::Files(sources), false, &mut warnings);

        let mut file = File::create(&tailwind_output).unwrap();

        file.write(parsed.as_bytes()).unwrap();

        match warnings.is_empty() {
            true => Ok(ResponseEvent::Refresh(vec![tailwind_output])),
            false => Err(()),
        }
    }

    fn register() -> Result<(), ()> {
        log(&format!("{:?}", watched_paths()));

        // Todo make this automatically get asset directory
        let tailwind_path = std::path::PathBuf::from("/public").join("tailwind.css");
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
