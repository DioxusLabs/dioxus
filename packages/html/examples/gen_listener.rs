use std::{
    collections::HashMap, fmt::Write as FmtWrite, fs::File, io::Write as IoWrite, path::Path,
};

use serde::{Deserialize, Serialize};

fn main() {
    let input = include_str!("./listeners.toml");
    let root: TomlRoot = toml::from_str(input).unwrap();

    let schema: TomlRoot = toml::from_str(input).unwrap();
    let root_dir = std::env::current_dir().unwrap().join("src");
    let code_gen_dir = root_dir.join("codegen");

    let element_dir = code_gen_dir.join("listeners");

    // let mut mod_dir = File::create(code_gen_dir.join("mod.rs")).unwrap();

    // writeln!(mod_dir, "pub mod elements {{").unwrap();

    for (element, props) in schema.listeners {
        // writeln!(mod_dir, "    pub mod {element};").unwrap();
        // writeln!(mod_dir, "    pub use {element}::{element};\n").unwrap();

        let mut element_file = File::create(element_dir.join(format!("{element}.rs"))).unwrap();
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct TomlRoot {
    listeners: HashMap<String, Vec<String>>,
}
