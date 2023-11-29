use crate::exports::plugins::main::definitions::Guest;
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
}

export_plugin!(Plugin);
