use dioxus_cli_plugin::*;
use exports::plugins::main::definitions::Guest;
use plugins::main::{
    imports::{log, PluginInfo},
    toml::{Toml, TomlValue},
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

    fn on_rebuild() -> Result<(), ()> {
        println!("Hello from on_rebuild!");
        Ok(())
    }

    fn on_hot_reload() {
        println!("Hello from on_hot_reload!");
    }

    fn on_watched_paths_change(_: std::vec::Vec<std::string::String>) {}

    fn register() -> Result<PluginInfo, ()> {
        Ok(PluginInfo {
            name: "TestPlugin".into(),
            version: "0.0.1".into(),
        })
    }

    fn before_build() -> Result<(), ()> {
        Ok(())
    }

    fn before_serve() -> Result<(), ()> {
        Ok(())
    }
}

export_plugin!(Plugin);
