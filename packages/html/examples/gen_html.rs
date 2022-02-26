use std::{collections::HashMap, fs::File, io::Write as IoWrite};

use serde::{Deserialize, Serialize};

fn main() {
    let input = include_str!("./html_elements.toml");

    let schema: TomlRoot = toml::from_str(input).unwrap();
    let root_dir = std::env::current_dir().unwrap().join("src");
    let code_gen_dir = root_dir.join("codegen");

    let element_dir = code_gen_dir.join("elements");

    let mut mod_dir = File::create(code_gen_dir.join("mod.rs")).unwrap();

    writeln!(mod_dir, "pub mod elements {{").unwrap();

    for (element, props) in schema.elements {
        writeln!(mod_dir, "    pub mod {element};").unwrap();
        writeln!(mod_dir, "    pub use {element}::{element};\n").unwrap();

        let mut element_file = File::create(element_dir.join(format!("{element}.rs"))).unwrap();

        let element_name = element.as_str();

        let mut struct_name = element.clone();
        struct_name.get_mut(0..1).unwrap().make_ascii_uppercase();

        writeln!(
            element_file,
            r#"//! Declarations for the `{element}` element.

use crate::builder::{{ElementBuilder, IntoAttributeValue}};
use dioxus_core::ScopeState;

pub struct {struct_name};

/// Build a
/// [`<{element_name}>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/{element_name})
/// element.
pub fn {element_name}(cx: &ScopeState) -> ElementBuilder<{struct_name}> {{
    ElementBuilder::new(cx, {struct_name}, "{element_name}")
}}
"#
        )
        .unwrap();

        if !props.is_empty() {
            writeln!(
                element_file,
                "impl<'a> ElementBuilder<'a, {struct_name}> {{"
            )
            .unwrap();

            for (name, _prop) in props {
                let rust_safe_name = match name.as_str() {
                    "type" => "r#type",
                    "for" => "r#for",
                    "loop" => "r#loop",
                    "as" => "r#as",
                    _ => name.as_str(),
                };

                writeln!(
                    element_file,
                    r#"    #[inline]
    pub fn {rust_safe_name}(mut self, val: impl IntoAttributeValue<'a>) -> Self {{
        self.push_attr("{name}", val);
        self
    }}"#
                )
                .unwrap();
            }

            writeln!(element_file, r#"}} "#).unwrap();
        }
    }

    writeln!(mod_dir, "}}").unwrap();
}

#[derive(Serialize, Deserialize, Debug)]
struct TomlRoot {
    elements: HashMap<String, HashMap<String, String>>,
}

#[derive(Serialize, Deserialize, Debug)]
struct TomlElement {
    name: String,
}
