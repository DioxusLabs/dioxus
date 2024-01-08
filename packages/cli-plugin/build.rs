//! Construct version in the `commit-hash date channel` format

use std::{env, path::PathBuf};

fn main() {
    let manifest_dir = PathBuf::from(
        env::var("CARGO_MANIFEST_DIR").expect("`CARGO_MANIFEST_DIR` is always set by cargo."),
    );

    let head_ref = manifest_dir.join("./wit/plugin.wit");
    if head_ref.exists() {
        println!("cargo:rerun-if-changed={}", head_ref.display());
    }

    // Create a file with a macro rules macro that expands into a wit bindgen macro call with inline wit code
    let wit_file = manifest_dir.join("./wit/plugin.wit");
    let wit_source = std::fs::read_to_string(wit_file).unwrap();
    let code = format!(
        r#"#[macro_export]
macro_rules! export_plugin {{
    ($name:ident) => {{
        ::wit_bindgen::generate!({{
            inline: "{}",
            world: "plugin-world",
            exports: {{
                world: $name,
                "plugins:main/definitions": $name
            }},
        }});
    }};
}}
"#,
        wit_source
    );

    let out_dir = manifest_dir.join("./src/export_plugin.rs");
    std::fs::write(out_dir, code).unwrap();
}
