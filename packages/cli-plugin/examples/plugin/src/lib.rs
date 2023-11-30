use dioxus_cli_plugin::*;
use exports::plugins::main::definitions::Guest;
use plugins::main::{
    imports::log,
    toml::{Array, Table, Toml, TomlValue},
};
struct Plugin;

impl Guest for Plugin {
    fn apply_config(config: Toml) -> bool {
        log(&format!("{:?}", config.get()));
        true
    }

    fn get_default_config() -> Toml {
        log("Starting to make default config from plugin!");
        let tomls: Vec<Toml> = (0..10).map(TomlValue::Integer).map(Toml::new).collect();
        let res = Toml::new(TomlValue::Array(tomls));
        log("Got a default config from plugin!");
        res
    }

    fn on_rebuild() -> bool {
        println!("Hello from on_rebuild!");
        true
    }

    fn on_hot_reload() {
        println!("Hello from on_hot_reload!");
    }

    fn on_watched_paths_change(_: std::vec::Vec<std::string::String>) {}

    fn register() -> bool {
        true
    }

    fn before_build() -> bool {
        true
    }

    fn before_serve() -> bool {
        true
    }
}

export_plugin!(Plugin);
