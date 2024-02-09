use std::{fs::File, io::Write, path::PathBuf};

use dioxus_cli_plugin::*;
use exports::plugins::main::definitions::Guest;
use plugins::main::{
    imports::{get_config, get_data, log, set_config, watched_paths},
    // toml::{Toml, TomlValue},
    types::{CommandEvent, PluginInfo, ResponseEvent, RuntimeEvent},
};
use railwind::parse_to_string;
use regex::Regex;

const PREFLIGHT: &'static str = include_str!("./tailwind_preflight.css");
const PREFLIGHT_LEN: usize = PREFLIGHT.len();

struct Plugin;

fn get_classes_regex(path: &PathBuf, regex: &Regex) -> Option<Vec<String>> {
    let file = std::fs::read_to_string(path).ok()?;
    let classes = regex
        .captures_iter(&file)
        .filter_map(|f| f.get(0).map(|f| f.as_str().to_string()))
        .collect();
    Some(classes)
}

fn get_classes_naive(path: &PathBuf, regex: &Regex) -> Option<Vec<String>> {
    let file = std::fs::read_to_string(path).ok()?;
    // Go through the entire file and return everything that is surrounded by quote and split at whitespace.
    let classes = regex
        .captures_iter(&file)
        .map(|f| {
            f.get(0)
                .unwrap()
                .as_str()
                .split_whitespace()
                .map(ToString::to_string)
        })
        .flatten()
        .collect();
    Some(classes)
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

    // Matches on rsx "class:" attributes
    let rsx_regex = Regex::new(r#"class:\s*(?:\"([^\"]+)\"|\'([^\']+)\')"#).unwrap();

    // Matches on anything in quotes, useful for components that piece together classes at runtime
    let naive_regex = Regex::new(r#"[^"\\]*(?:\\.[^"\\]*)*"#).unwrap();

    let naive = match get_config("naive_check") {
        Some(st) if &st == "true" => true,
        Some(st) if &st == "false" => false,
        Some(other) => {
            log(&format!("Incorrect value for naive_check: {other}"));
            return Err(());
        }
        None => {
            log(&format!("naive_check config value missing!"));
            return Err(());
        }
    };

    let classes: Vec<_> = if !naive {
        paths
            .iter()
            .flat_map(|f| get_classes_regex(f, &rsx_regex))
            .flatten()
            .map(|f| f.strip_prefix("class:").unwrap().trim().replace('"', ""))
            .collect()
    } else {
        paths
            .iter()
            .flat_map(|f| get_classes_naive(f, &naive_regex))
            .flatten()
            .collect()
    };

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

    let tailwind_output = "dist/tailwind.css";
    let mut file = File::create(tailwind_output).map_err(std::mem::drop)?;
    let written = file.write(parsed.as_bytes()).map_err(std::mem::drop)?;

    if written != parsed.len() {
        log("Could not write all the bytes to the tailwind file!");
        return Err(());
    }

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
            log(&format!("Found no watchable files in {}", path.display()));
            continue;
        };
        if let ResponseEvent::Refresh(paths) = parse_and_save_css(paths)? {
            event = ResponseEvent::Refresh(paths);
        }
    }
    Ok(event)
}

impl Guest for Plugin {
    fn on_watched_paths_change(
        _paths: std::vec::Vec<std::string::String>,
    ) -> Result<ResponseEvent, ()> {
        gen_tailwind()
    }

    fn register() -> Result<(), ()> {
        set_config("naive_check", "false");
        log("Registered Tailwind Plugin Successfully!");
        Ok(())
    }

    fn metadata() -> exports::plugins::main::definitions::PluginInfo {
        PluginInfo {
            name: "Tailwind".into(),
            version: "0.0.1".into(),
        }
    }

    fn before_command_event(_event: CommandEvent) -> Result<(), ()> {
        Ok(())
    }
    fn before_runtime_event(_event: RuntimeEvent) -> Result<ResponseEvent, ()> {
        Ok(ResponseEvent::None)
    }

    fn after_command_event(_event: CommandEvent) -> Result<(), ()> {
        gen_tailwind()?;
        Ok(())
    }

    fn after_runtime_event(_event: RuntimeEvent) -> Result<ResponseEvent, ()> {
        gen_tailwind()
    }
}

export_plugin!(Plugin);
