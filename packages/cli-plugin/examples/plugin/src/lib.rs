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

fn get_parsable_files(path: &PathBuf) -> Option<Vec<PathBuf>> {
    Some(
        std::fs::read_dir(path)
            .ok()?
            .map(|e| e.unwrap().path())
            .filter(|p| {
                p.to_str()
                    .map(|s| s.ends_with(".rs") || s.ends_with(".html"))
                    .unwrap_or(false)
            })
            .collect(),
    )
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

    // If it's empty then nothing will change
    if classes.is_empty() {
        log("Skipping tailwind reload, no change necessary");
        return Ok(ResponseEvent::None);
    }

    let classes = classes.join(" ");

    let mut warnings = vec![];
    let parsed = parse_to_string(
        railwind::Source::String(classes, railwind::CollectionOptions::String),
        true,
        &mut warnings,
    );

    let tailwind_output = "assets/tailwind.css";
    let mut file = File::create(tailwind_output).unwrap();
    file.write(parsed.as_bytes()).unwrap();

    for warning in warnings.iter() {
        log(&warning.to_string())
    }

    Ok(ResponseEvent::Refresh(vec!["tailwind.css".into()]))
}

fn gen_tailwind() -> Result<ResponseEvent, ()> {
    let watched_paths: Vec<_> = watched_paths().into_iter().map(PathBuf::from).collect();
    let mut event = ResponseEvent::None;
    for path in watched_paths.iter() {
        let Some(paths) = get_parsable_files(path) else {
            continue;
        };
        if let ResponseEvent::Refresh(paths) = parse_and_save_css(paths)? {
            event = ResponseEvent::Refresh(paths);
        }
    }
    Ok(event)
}

impl Guest for Plugin {
    fn apply_config(_config: Toml) -> Result<(), ()> {
        Ok(())
    }

    fn get_default_config() -> Toml {
        Toml::new(TomlValue::Integer(0))
    }

    fn on_watched_paths_change(
        _paths: std::vec::Vec<std::string::String>,
    ) -> Result<ResponseEvent, ()> {
        gen_tailwind()
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
        gen_tailwind()?;
        Ok(())
    }

    fn after_runtime_event(_event: RuntimeEvent) -> Result<ResponseEvent, ()> {
        gen_tailwind()
    }
}

export_plugin!(Plugin);
