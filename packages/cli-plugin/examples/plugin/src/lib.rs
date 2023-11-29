use dioxus_cli_plugin::*;

struct Plugin;

impl DynGuest for Plugin {
    fn on_rebuild(&self) -> bool {
        println!("Hello from on_rebuild!");
        true
    }

    fn on_hot_reload(&self) -> bool {
        println!("Hello from on_hot_reload!");
        true
    }
}

export_plugin!(Plugin);
