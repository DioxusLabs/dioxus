use dioxus_cli_plugin::*;
use exports::plugins::main::definitions::{Event, Guest};
use plugins::main::{
    imports::{log, watch_path, watched_paths},
    toml::{Toml, TomlValue},
    types::PluginInfo,
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

    fn on_watched_paths_change(_: std::vec::Vec<std::string::String>) {}

    fn register() -> Result<(), ()> {
        log(&format!("{:?}", watched_paths()));

        watch_path("tests");

        log("Watched `tests` path!");
        Ok(())
    }

    fn metadata() -> exports::plugins::main::definitions::PluginInfo {
        PluginInfo {
            name: "TestPlugin".into(),
            version: "0.0.1".into(),
        }
    }

    fn before_event(event: Event) -> Result<(), ()> {
        log(&format!("Got before event in plugin: {event:?}"));
        Ok(())
    }

    fn after_event(event: Event) -> Result<(), ()> {
        log(&format!("Got after event in plugin: {event:?}"));
        Ok(())
    }
}

export_plugin!(Plugin);
