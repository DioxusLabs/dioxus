use dioxus_cli_plugin::*;
use exports::plugins::main::definitions::Guest;
use plugins::main::{
    imports::{get_config, log, set_config},
    types::{CommandEvent, PluginInfo, Response, RuntimeEvent},
};

struct Plugin;

impl Guest for Plugin {
    fn get_default_config() -> exports::plugins::main::definitions::Toml {
        exports::plugins::main::definitions::Toml::new(plugins::main::toml::TomlValue::Table(
            Vec::new(),
        ))
    }

    fn apply_config(_: exports::plugins::main::definitions::Toml) -> Result<(), ()> {
        Ok(())
    }

    fn register() -> Result<(), ()> {
        set_config("test", "false");
        log("Registered Tailwind!");
        Ok(())
    }

    fn metadata() -> PluginInfo {
        PluginInfo {
            name: "Test Plugin".into(),
            version: "0.0.1".into(),
        }
    }

    fn before_command_event(_event: CommandEvent) -> Result<(), ()> {
        Ok(())
    }

    fn before_runtime_event(_event: RuntimeEvent) -> Result<Response, ()> {
        let Some(data) = get_config("test") else {
            log("Error ahhhhhh!");
            return Err(());
        };
        log(&data);
        Ok(Response::None)
    }

    fn after_command_event(_event: CommandEvent) -> Result<(), ()> {
        Ok(())
    }

    fn after_runtime_event(_event: RuntimeEvent) -> Result<Response, ()> {
        Ok(Response::None)
    }

    fn on_watched_paths_change(
        _path: wit_bindgen::rt::vec::Vec<wit_bindgen::rt::string::String>,
    ) -> Result<Response, ()> {
        Ok(Response::None)
    }
}

export!(Plugin);
