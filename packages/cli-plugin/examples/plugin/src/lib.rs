use crate::exports::plugins::main::definitions::Guest;
use crate::exports::plugins::main::definitions::Config;
use dioxus_cli_plugin::*;

struct Plugin;

impl Guest for Plugin {
    fn on_rebuild() -> bool {
        println!("Hello from on_rebuild!");
        true
    }

    fn on_hot_reload() {
        println!("Hello from on_hot_reload!");
    }

    fn on_watched_paths_change(_: std::vec::Vec<std::string::String>) {}

    fn register(conf:Config,) -> bool {
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
