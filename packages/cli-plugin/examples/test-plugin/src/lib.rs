use dioxus_cli_plugin::*;
use exports::plugins::main::definitions::Guest;
use plugins::main::{
    imports::{get_config, get_data, log, set_config},
    types::{CommandEvent, PluginInfo, ResponseEvent, RuntimeEvent},
};

struct Plugin;

impl Guest for Plugin {
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

    fn before_runtime_event(_event: RuntimeEvent) -> Result<ResponseEvent, ()> {
        let Some(data) = get_config("test") else {
            log("Error ahhhhhh!");
            return Err(());
        };
        log(&data);
        Ok(ResponseEvent::None)
    }

    fn after_command_event(_event: CommandEvent) -> Result<(), ()> {
        Ok(())
    }

    fn after_runtime_event(_event: RuntimeEvent) -> Result<ResponseEvent, ()> {
        Ok(ResponseEvent::None)
    }

    fn on_watched_paths_change(
        _path: wit_bindgen::rt::vec::Vec<wit_bindgen::rt::string::String>,
    ) -> Result<ResponseEvent, ()> {
        Ok(ResponseEvent::None)
    }
}

export_plugin!(Plugin);
