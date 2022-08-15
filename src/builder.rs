use crate::{
    config::{CrateConfig, ExecutableType},
    error::{Error, Result},
    tools::Tool,
    DioxusConfig,
};
use cargo_metadata::{diagnostic::Diagnostic, Message};
use indicatif::{ProgressBar, ProgressStyle};
use serde::Serialize;
use std::{
    fs::{copy, create_dir_all, remove_dir_all, File},
    io::Read,
    panic,
    path::PathBuf,
    process::Command,
    time::Duration,
};
use wasm_bindgen_cli_support::Bindgen;

#[derive(Serialize, Debug, Clone)]
pub struct BuildResult {
    pub warnings: Vec<Diagnostic>,
    pub elapsed_time: u128,
}

pub fn build(config: &CrateConfig, quiet: bool) -> Result<BuildResult> {
    // [1] Build the project with cargo, generating a wasm32-unknown-unknown target (is there a more specific, better target to leverage?)
    // [2] Generate the appropriate build folders
    // [3] Wasm-bindgen the .wasm fiile, and move it into the {builddir}/modules/xxxx/xxxx_bg.wasm
    // [4] Wasm-opt the .wasm file with whatever optimizations need to be done
    // [5][OPTIONAL] Builds the Tailwind CSS file using the Tailwind standalone binary
    // [6] Link up the html page to the wasm module

    let CrateConfig {
        out_dir,
        crate_dir,
        target_dir,
        asset_dir,
        executable,
        dioxus_config,
        ..
    } = config;

    // start to build the assets
    let ignore_files = build_assets(config)?;

    let t_start = std::time::Instant::now();

    // [1] Build the .wasm module
    log::info!("🚅 Running build command...");
    let mut cmd = Command::new("cargo");
    cmd.current_dir(&crate_dir)
        .arg("build")
        .arg("--target")
        .arg("wasm32-unknown-unknown")
        .arg("--message-format=json")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    if config.release {
        cmd.arg("--release");
    }
    if config.verbose {
        cmd.arg("--verbose");
    }

    if quiet {
        cmd.arg("--quiet");
    }

    if config.custom_profile.is_some() {
        let custom_profile = config.custom_profile.as_ref().unwrap();
        cmd.arg("--profile");
        cmd.arg(custom_profile);
    }

    if config.features.is_some() {
        let features_str = config.features.as_ref().unwrap().join(" ");
        cmd.arg("--features");
        cmd.arg(features_str);
    }

    match executable {
        ExecutableType::Binary(name) => cmd.arg("--bin").arg(name),
        ExecutableType::Lib(name) => cmd.arg("--lib").arg(name),
        ExecutableType::Example(name) => cmd.arg("--example").arg(name),
    };

    let warning_messages = prettier_build(cmd)?;

    // [2] Establish the output directory structure
    let bindgen_outdir = out_dir.join("assets").join("dioxus");

    let release_type = match config.release {
        true => "release",
        false => "debug",
    };

    let input_path = match executable {
        ExecutableType::Binary(name) | ExecutableType::Lib(name) => target_dir
            .join(format!("wasm32-unknown-unknown/{}", release_type))
            .join(format!("{}.wasm", name)),

        ExecutableType::Example(name) => target_dir
            .join(format!("wasm32-unknown-unknown/{}/examples", release_type))
            .join(format!("{}.wasm", name)),
    };

    let bindgen_result = panic::catch_unwind(move || {
        // [3] Bindgen the final binary for use easy linking
        let mut bindgen_builder = Bindgen::new();

        bindgen_builder
            .input_path(input_path)
            .web(true)
            .unwrap()
            .debug(true)
            .demangle(true)
            .keep_debug(true)
            .remove_name_section(false)
            .remove_producers_section(false)
            .out_name(&dioxus_config.application.name)
            .generate(&bindgen_outdir)
            .unwrap();
    });
    if bindgen_result.is_err() {
        return Err(Error::BuildFailed("Bindgen build failed! \nThis is probably due to the Bindgen version, dioxus-cli using `0.2.79` Bindgen crate.".to_string()));
    }

    // check binaryen:wasm-opt tool
    let dioxus_tools = dioxus_config.application.tools.clone().unwrap_or_default();
    if dioxus_tools.contains_key("binaryen") {
        let info = dioxus_tools.get("binaryen").unwrap();
        let binaryen = crate::tools::Tool::Binaryen;

        if binaryen.is_installed() {
            if let Some(sub) = info.as_table() {
                if sub.contains_key("wasm_opt")
                    && sub.get("wasm_opt").unwrap().as_bool().unwrap_or(false)
                {
                    log::info!("Optimizing WASM size with wasm-opt...");
                    let target_file = out_dir
                        .join("assets")
                        .join("dioxus")
                        .join(format!("{}_bg.wasm", dioxus_config.application.name));
                    if target_file.is_file() {
                        let mut args = vec![
                            target_file.to_str().unwrap(),
                            "-o",
                            target_file.to_str().unwrap(),
                        ];
                        if config.release == true {
                            args.push("-Oz");
                        }
                        binaryen.call("wasm-opt", args)?;
                    }
                }
            }
        } else {
            log::warn!(
                "Binaryen tool not found, you can use `dioxus tool add binaryen` to install it."
            );
        }
    }

    // [5][OPTIONAL] If tailwind is enabled and installed we run it to generate the CSS
    if dioxus_tools.contains_key("tailwindcss") {
        let info = dioxus_tools.get("tailwindcss").unwrap();
        let tailwind = crate::tools::Tool::Tailwind;

        if tailwind.is_installed() {
            if let Some(sub) = info.as_table() {
                log::info!("Building Tailwind bundle CSS file...");

                let input_path = match sub.get("input") {
                    Some(val) => val.as_str().unwrap(),
                    None => "./public",
                };
                let config_path = match sub.get("config") {
                    Some(val) => val.as_str().unwrap(),
                    None => "./src/tailwind.config.js",
                };
                let mut args = vec![
                    "-i",
                    input_path,
                    "-o",
                    "dist/tailwind.css",
                    "-c",
                    config_path,
                ];

                if config.release == true {
                    args.push("--minify");
                }

                tailwind.call("tailwindcss", args)?;
            }
        } else {
            log::warn!(
                "Tailwind tool not found, you can use `dioxus tool add tailwindcss` to install it."
            );
        }
    }

    // this code will copy all public file to the output dir
    let copy_options = fs_extra::dir::CopyOptions {
        overwrite: true,
        skip_exist: false,
        buffer_size: 64000,
        copy_inside: false,
        content_only: false,
        depth: 0,
    };
    if asset_dir.is_dir() {
        for entry in std::fs::read_dir(&asset_dir)? {
            let path = entry?.path();
            if path.is_file() {
                std::fs::copy(&path, out_dir.join(path.file_name().unwrap()))?;
            } else {
                match fs_extra::dir::copy(&path, out_dir, &copy_options) {
                    Ok(_) => {}
                    Err(_e) => {
                        log::warn!("Error copying dir: {}", _e);
                    }
                }
                for ignore in &ignore_files {
                    let ignore = ignore.strip_prefix(&config.asset_dir).unwrap();
                    let ignore = config.out_dir.join(ignore);
                    if ignore.is_file() {
                        std::fs::remove_file(ignore)?;
                    }
                }
            }
        }
    }

    let t_end = std::time::Instant::now();
    Ok(BuildResult {
        warnings: warning_messages,
        elapsed_time: (t_end - t_start).as_millis(),
    })
}

pub fn build_desktop(config: &CrateConfig, is_serve: bool) -> Result<()> {
    log::info!("🚅 Running build [Desktop] command...");

    let ignore_files = build_assets(config)?;

    let mut cmd = Command::new("cargo");
    cmd.current_dir(&config.crate_dir)
        .arg("build")
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit());

    if config.release {
        cmd.arg("--release");
    }
    if config.verbose {
        cmd.arg("--verbose");
    }

    if config.custom_profile.is_some() {
        let custom_profile = config.custom_profile.as_ref().unwrap();
        cmd.arg("--profile");
        cmd.arg(custom_profile);
    }

    if config.features.is_some() {
        let features_str = config.features.as_ref().unwrap().join(" ");
        cmd.arg("--features");
        cmd.arg(features_str);
    }

    match &config.executable {
        crate::ExecutableType::Binary(name) => cmd.arg("--bin").arg(name),
        crate::ExecutableType::Lib(name) => cmd.arg("--lib").arg(name),
        crate::ExecutableType::Example(name) => cmd.arg("--example").arg(name),
    };

    let output = cmd.output()?;

    if !output.status.success() {
        return Err(Error::BuildFailed("Program build failed.".into()));
    }

    if output.status.success() {
        // this code will clean the output dir.
        // if using the serve, we will not clean the out_dir.
        if config.out_dir.is_dir() && !is_serve {
            remove_dir_all(&config.out_dir)?;
        }

        let release_type = match config.release {
            true => "release",
            false => "debug",
        };

        let file_name: String;
        let mut res_path = match &config.executable {
            crate::ExecutableType::Binary(name) | crate::ExecutableType::Lib(name) => {
                file_name = name.clone();
                config.target_dir.join(release_type).join(name)
            }
            crate::ExecutableType::Example(name) => {
                file_name = name.clone();
                config
                    .target_dir
                    .join(release_type)
                    .join("examples")
                    .join(name)
            }
        };

        let target_file = if cfg!(windows) {
            res_path.set_extension("exe");
            format!("{}.exe", &file_name)
        } else {
            file_name
        };

        if !config.out_dir.is_dir() {
            create_dir_all(&config.out_dir)?;
        }
        copy(res_path, &config.out_dir.join(target_file))?;

        // this code will copy all public file to the output dir
        if config.asset_dir.is_dir() {
            let copy_options = fs_extra::dir::CopyOptions {
                overwrite: true,
                skip_exist: false,
                buffer_size: 64000,
                copy_inside: false,
                content_only: false,
                depth: 0,
            };

            for entry in std::fs::read_dir(&config.asset_dir)? {
                let path = entry?.path();
                if path.is_file() {
                    std::fs::copy(&path, &config.out_dir.join(path.file_name().unwrap()))?;
                } else {
                    match fs_extra::dir::copy(&path, &config.out_dir, &copy_options) {
                        Ok(_) => {}
                        Err(e) => {
                            log::warn!("Error copying dir: {}", e);
                        }
                    }
                    for ignore in &ignore_files {
                        let ignore = ignore.strip_prefix(&config.asset_dir).unwrap();
                        let ignore = config.out_dir.join(ignore);
                        if ignore.is_file() {
                            std::fs::remove_file(ignore)?;
                        }
                    }
                }
            }
        }

        log::info!(
            "🚩 Build completed: [./{}]",
            config
                .dioxus_config
                .application
                .out_dir
                .clone()
                .unwrap_or_else(|| PathBuf::from("dist"))
                .display()
        );
    }

    Ok(())
}

fn prettier_build(mut cmd: Command) -> anyhow::Result<Vec<Diagnostic>> {
    let mut warning_messages: Vec<Diagnostic> = vec![];

    let pb = ProgressBar::new_spinner();
    pb.enable_steady_tick(Duration::from_millis(200));
    pb.set_style(
        ProgressStyle::with_template("{spinner:.dim.bold} {wide_msg}")
            .unwrap()
            .tick_chars("/|\\- "),
    );
    pb.set_message("💼 Waiting to start build the project...");

    let mut command = cmd.spawn()?;
    let reader = std::io::BufReader::new(command.stdout.take().unwrap());
    for message in cargo_metadata::Message::parse_stream(reader) {
        match message.unwrap() {
            Message::CompilerMessage(msg) => {
                let message = msg.message;
                match message.level {
                    cargo_metadata::diagnostic::DiagnosticLevel::Error => {
                        return Err(anyhow::anyhow!(message
                            .rendered
                            .unwrap_or("Unknown".into())));
                    }
                    cargo_metadata::diagnostic::DiagnosticLevel::Warning => {
                        warning_messages.push(message.clone());
                    }
                    _ => {}
                }
            }
            Message::CompilerArtifact(artifact) => {
                pb.set_message(format!("Compiling {} ", artifact.package_id));
                pb.tick();
            }
            Message::BuildScriptExecuted(script) => {
                let _package_id = script.package_id.to_string();
            }
            Message::BuildFinished(finished) => {
                if finished.success {
                    pb.finish_and_clear();
                    log::info!("👑 Build done.");
                } else {
                    std::process::exit(1);
                }
            }
            _ => (), // Unknown message
        }
    }
    Ok(warning_messages)
}

pub fn gen_page(config: &DioxusConfig, serve: bool) -> String {
    let crate_root = crate::cargo::crate_root().unwrap();
    let custom_html_file = crate_root.join("index.html");
    let mut html = if custom_html_file.is_file() {
        let mut buf = String::new();
        let mut file = File::open(custom_html_file).unwrap();
        if file.read_to_string(&mut buf).is_ok() {
            buf
        } else {
            String::from(include_str!("./assets/index.html"))
        }
    } else {
        String::from(include_str!("./assets/index.html"))
    };

    let resouces = config.web.resource.clone();

    let mut style_list = resouces.style.unwrap_or_default();
    let mut script_list = resouces.script.unwrap_or_default();

    if serve {
        let mut dev_style = resouces.dev.style.clone().unwrap_or_default();
        let mut dev_script = resouces.dev.script.unwrap_or_default();
        style_list.append(&mut dev_style);
        script_list.append(&mut dev_script);
    }

    let mut style_str = String::new();
    for style in style_list {
        style_str.push_str(&format!(
            "<link rel=\"stylesheet\" href=\"{}\">\n",
            &style.to_str().unwrap(),
        ))
    }
    if config
        .application
        .tools
        .clone()
        .unwrap_or_default()
        .contains_key("tailwindcss")
    {
        style_str.push_str("<link rel=\"stylesheet\" href=\"tailwind.css\">\n");
    }
    html = html.replace("{style_include}", &style_str);

    let mut script_str = String::new();
    for script in script_list {
        script_str.push_str(&format!(
            "<script src=\"{}\"></script>\n",
            &script.to_str().unwrap(),
        ))
    }

    html = html.replace("{script_include}", &script_str);

    if serve {
        html += &format!(
            "<script>{}</script>",
            include_str!("./assets/autoreload.js")
        );
    }

    html = html.replace("{app_name}", &config.application.name);

    html = match &config.web.app.base_path {
        Some(path) => html.replace("{base_path}", path),
        None => html.replace("{base_path}", "."),
    };

    let title = config
        .web
        .app
        .title
        .clone()
        .unwrap_or_else(|| "dioxus | ⛺".into());

    html.replace("{app_title}", &title)
}

// this function will build some assets file
// like sass tool resources
// this function will return a array which file don't need copy to out_dir.
fn build_assets(config: &CrateConfig) -> Result<Vec<PathBuf>> {
    let mut result = vec![];

    let dioxus_config = &config.dioxus_config;
    let dioxus_tools = dioxus_config.application.tools.clone().unwrap_or_default();

    // check sass tool state
    let sass = Tool::Sass;
    if sass.is_installed() && dioxus_tools.contains_key("sass") {
        let sass_conf = dioxus_tools.get("sass").unwrap();
        if let Some(tab) = sass_conf.as_table() {
            let source_map = tab.contains_key("source_map");
            let source_map = if source_map && tab.get("source_map").unwrap().is_bool() {
                if tab.get("source_map").unwrap().as_bool().unwrap_or_default() {
                    "--source-map"
                } else {
                    "--no-source-map"
                }
            } else {
                "--source-map"
            };

            if tab.contains_key("input") {
                if tab.get("input").unwrap().is_str() {
                    let file = tab.get("input").unwrap().as_str().unwrap().trim();

                    if file == "*" {
                        // if the sass open auto, we need auto-check the assets dir.
                        let asset_dir = config.asset_dir.clone();
                        if asset_dir.is_dir() {
                            for entry in walkdir::WalkDir::new(&asset_dir)
                                .into_iter()
                                .filter_map(|e| e.ok())
                            {
                                let temp = entry.path();
                                if temp.is_file() {
                                    let suffix = temp.extension();
                                    if suffix.is_none() {
                                        continue;
                                    }
                                    let suffix = suffix.unwrap().to_str().unwrap();
                                    if suffix == "scss" || suffix == "sass" {
                                        // if file suffix is `scss` / `sass` we need transform it.
                                        let out_file = format!(
                                            "{}.css",
                                            temp.file_stem().unwrap().to_str().unwrap()
                                        );
                                        let target_path = config
                                            .out_dir
                                            .join(
                                                temp.strip_prefix(&asset_dir)
                                                    .unwrap()
                                                    .parent()
                                                    .unwrap(),
                                            )
                                            .join(out_file);
                                        let res = sass.call(
                                            "sass",
                                            vec![
                                                temp.to_str().unwrap(),
                                                target_path.to_str().unwrap(),
                                                source_map,
                                            ],
                                        );
                                        if res.is_ok() {
                                            result.push(temp.to_path_buf());
                                        }
                                    }
                                }
                            }
                        }
                    } else {
                        // just transform one file.
                        let relative_path = if &file[0..1] == "/" {
                            &file[1..file.len()]
                        } else {
                            file
                        };
                        let path = config.asset_dir.join(relative_path);
                        let out_file =
                            format!("{}.css", path.file_stem().unwrap().to_str().unwrap());
                        let target_path = config
                            .out_dir
                            .join(PathBuf::from(relative_path).parent().unwrap())
                            .join(out_file);
                        if path.is_file() {
                            let res = sass.call(
                                "sass",
                                vec![
                                    path.to_str().unwrap(),
                                    target_path.to_str().unwrap(),
                                    source_map,
                                ],
                            );
                            if res.is_ok() {
                                result.push(path);
                            } else {
                                log::error!("{:?}", res);
                            }
                        }
                    }
                } else if tab.get("input").unwrap().is_array() {
                    // check files list.
                    let list = tab.get("input").unwrap().as_array().unwrap();
                    for i in list {
                        if i.is_str() {
                            let path = i.as_str().unwrap();
                            let relative_path = if &path[0..1] == "/" {
                                &path[1..path.len()]
                            } else {
                                path
                            };
                            let path = config.asset_dir.join(relative_path);
                            let out_file =
                                format!("{}.css", path.file_stem().unwrap().to_str().unwrap());
                            let target_path = config
                                .out_dir
                                .join(PathBuf::from(relative_path).parent().unwrap())
                                .join(out_file);
                            if path.is_file() {
                                let res = sass.call(
                                    "sass",
                                    vec![
                                        path.to_str().unwrap(),
                                        target_path.to_str().unwrap(),
                                        source_map,
                                    ],
                                );
                                if res.is_ok() {
                                    result.push(path);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    // SASS END

    Ok(result)
}

// use binary_install::{Cache, Download};

// /// Attempts to find `wasm-opt` in `PATH` locally, or failing that downloads a
// /// precompiled binary.
// ///
// /// Returns `Some` if a binary was found or it was successfully downloaded.
// /// Returns `None` if a binary wasn't found in `PATH` and this platform doesn't
// /// have precompiled binaries. Returns an error if we failed to download the
// /// binary.
// pub fn find_wasm_opt(
//     cache: &Cache,
//     install_permitted: bool,
// ) -> Result<install::Status, failure::Error> {
//     // First attempt to look up in PATH. If found assume it works.
//     if let Ok(path) = which::which("wasm-opt") {
//         PBAR.info(&format!("found wasm-opt at {:?}", path));

//         match path.as_path().parent() {
//             Some(path) => return Ok(install::Status::Found(Download::at(path))),
//             None => {}
//         }
//     }

//     let version = "version_78";
//     Ok(install::download_prebuilt(
//         &install::Tool::WasmOpt,
//         cache,
//         version,
//         install_permitted,
//     )?)
// }
