use std::{fs::File, io::Write, path::PathBuf};

use dioxus_cli_plugin::*;
use exports::plugins::main::definitions::Guest;
use plugins::main::{
    imports::{log, watched_paths},
    toml::{Toml, TomlValue},
    types::{CompileEvent, PluginInfo, ResponseEvent, RuntimeEvent},
};
use railwind::parse_to_string;
use regex::Regex;

struct Plugin;

fn get_classes(path: &PathBuf, regex: &Regex) -> Vec<String> {
    let file = std::fs::read_to_string(path).unwrap();
    regex
        .captures_iter(&file)
        .filter_map(|f| f.get(0).map(|f| f.as_str().to_string()))
        .collect()
}

fn get_parsable_files(path: &PathBuf) -> Vec<PathBuf> {
    std::fs::read_dir(path)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| {
            p.to_str()
                .map(|s| s.ends_with(".rs") || s.ends_with(".html"))
                .unwrap_or(false)
        })
        .collect()
}

fn parse_and_save_css(paths: Vec<PathBuf>) -> Result<ResponseEvent, ()> {
    if paths.is_empty() {
        log("Skipping tailwind reload, no change necessary");
        return Ok(ResponseEvent::None);
    };

    let paths: Vec<PathBuf> = paths.into_iter().map(PathBuf::from).collect();

    let rsx_regex = Regex::new(r#"class:\s*(?:\"([^\"]+)\"|\'([^\']+)\')"#).unwrap();

    let classes: Vec<_> = paths
        .iter()
        .map(|f| get_classes(f, &rsx_regex))
        .flatten()
        .map(|f| f.strip_prefix("class:").unwrap().trim().replace('"', ""))
        .collect();

    let classes = classes.join(" ");

    // TODO Automatically set
    let tailwind_output = "public/tailwind.css".to_string();

    let mut warnings = vec![];
    let parsed = parse_to_string(
        railwind::Source::String(classes, railwind::CollectionOptions::String),
        true,
        &mut warnings,
    );

    let mut file = File::create(&tailwind_output).unwrap();

    file.write(parsed.as_bytes()).unwrap();

    match warnings.is_empty() {
        true => Ok(ResponseEvent::Refresh(vec![tailwind_output])),
        false => Err(()),
    }
}

impl Guest for Plugin {
    fn apply_config(_config: Toml) -> Result<(), ()> {
        Ok(())
    }

    fn get_default_config() -> Toml {
        Toml::new(TomlValue::Integer(0))
    }

    fn on_watched_paths_change(
        paths: std::vec::Vec<std::string::String>,
    ) -> Result<ResponseEvent, ()> {
        parse_and_save_css(paths.into_iter().map(PathBuf::from).collect())
    }

    fn register() -> Result<(), ()> {
        log("Registered Tailwind Plugin Successfully!");
        Ok(())
    }

    fn metadata() -> exports::plugins::main::definitions::PluginInfo {
        PluginInfo {
            name: "Tailwind".into(),
            version: "0.0.1".into(),
        }
    }

    fn before_compile_event(_event: CompileEvent) -> Result<(), ()> {
        Ok(())
    }
    fn before_runtime_event(_event: RuntimeEvent) -> Result<ResponseEvent, ()> {
        Ok(ResponseEvent::None)
    }

    fn after_compile_event(_event: CompileEvent) -> Result<(), ()> {
        let watched_paths: Vec<_> = watched_paths().into_iter().map(PathBuf::from).collect();
        for path in watched_paths.iter() {
            let paths = get_parsable_files(path);
            parse_and_save_css(paths)?;
        }
        Ok(())
    }

    fn after_runtime_event(_event: RuntimeEvent) -> Result<ResponseEvent, ()> {
        let watched_paths: Vec<_> = watched_paths().into_iter().map(PathBuf::from).collect();
        for path in watched_paths.iter() {
            let paths = get_parsable_files(path);
            parse_and_save_css(paths)?;
        }
        Ok(ResponseEvent::None)
    }
}

export_plugin!(Plugin);
