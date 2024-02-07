use crate::{
    assets::{asset_manifest, create_assets_head, process_assets, AssetConfigDropGuard},
    error::{Error, Result},
    tools::Tool,
};
use cargo_metadata::{diagnostic::Diagnostic, Message};
use dioxus_cli_config::crate_root;
use dioxus_cli_config::CrateConfig;
use dioxus_cli_config::ExecutableType;
use indicatif::{ProgressBar, ProgressStyle};
use lazy_static::lazy_static;
use manganis_cli_support::{AssetManifest, ManganisSupportGuard};
use std::{
    env,
    fs::{copy, create_dir_all, File},
    io::Read,
    panic,
    path::PathBuf,
    time::Duration,
};
use wasm_bindgen_cli_support::Bindgen;

lazy_static! {
    static ref PROGRESS_BARS: indicatif::MultiProgress = indicatif::MultiProgress::new();
}

#[derive(Debug, Clone)]
pub struct BuildResult {
    pub warnings: Vec<Diagnostic>,
    pub elapsed_time: u128,
    pub assets: Option<AssetManifest>,
}

/// This trait is only created for the convenient and concise way to set
/// `RUSTFLAGS` environment variable for the `subprocess::Exec`.
pub trait ExecWithRustFlagsSetter {
    fn set_rust_flags(self, rust_flags: Option<String>) -> Self;
}

impl ExecWithRustFlagsSetter for subprocess::Exec {
    /// Sets (appends to, if already set) `RUSTFLAGS` environment variable if
    /// `rust_flags` is not `None`.
    fn set_rust_flags(self, rust_flags: Option<String>) -> Self {
        if let Some(rust_flags) = rust_flags {
            // Some `RUSTFLAGS` might be already set in the environment or provided
            // by the user. They should take higher priority than the default flags.
            // If no default flags are provided, then there is no point in
            // redefining the environment variable with the same value, if it is
            // even set. If no custom flags are set, then there is no point in
            // adding the unnecessary whitespace to the command.
            self.env(
                "RUSTFLAGS",
                if let Ok(custom_rust_flags) = env::var("RUSTFLAGS") {
                    rust_flags + " " + custom_rust_flags.as_str()
                } else {
                    rust_flags
                },
            )
        } else {
            self
        }
    }
}

/// Build client (WASM).
/// Note: `rust_flags` argument is only used for the fullstack platform.
pub fn build(
    config: &CrateConfig,
    _: bool,
    skip_assets: bool,
    rust_flags: Option<String>,
) -> Result<BuildResult> {
    // [1] Build the project with cargo, generating a wasm32-unknown-unknown target (is there a more specific, better target to leverage?)
    // [2] Generate the appropriate build folders
    // [3] Wasm-bindgen the .wasm file, and move it into the {builddir}/modules/xxxx/xxxx_bg.wasm
    // [4] Wasm-opt the .wasm file with whatever optimizations need to be done
    // [5][OPTIONAL] Builds the Tailwind CSS file using the Tailwind standalone binary
    // [6] Link up the html page to the wasm module

    let CrateConfig {
        crate_dir,
        target_dir,
        executable,
        dioxus_config,
        ..
    } = config;
    let out_dir = config.out_dir();
    let asset_dir = config.asset_dir();

    let _guard = AssetConfigDropGuard::new();
    let _manganis_support = ManganisSupportGuard::default();

    // start to build the assets
    let ignore_files = build_assets(config)?;

    let t_start = std::time::Instant::now();
    let _guard = dioxus_cli_config::__private::save_config(config);

    // [1] Build the .wasm module
    log::info!("🚅 Running build command...");

    let wasm_check_command = std::process::Command::new("rustup")
        .args(["show"])
        .output()?;
    let wasm_check_output = String::from_utf8(wasm_check_command.stdout).unwrap();
    if !wasm_check_output.contains("wasm32-unknown-unknown") {
        log::info!("wasm32-unknown-unknown target not detected, installing..");
        let _ = std::process::Command::new("rustup")
            .args(["target", "add", "wasm32-unknown-unknown"])
            .output()?;
    }

    let cmd = subprocess::Exec::cmd("cargo")
        .set_rust_flags(rust_flags)
        .env("CARGO_TARGET_DIR", target_dir)
        .cwd(crate_dir)
        .arg("build")
        .arg("--target")
        .arg("wasm32-unknown-unknown")
        .arg("--message-format=json");

    // TODO: make the initial variable mutable to simplify all the expressions
    // below. Look inside the `build_desktop()` as an example.
    let cmd = if config.release {
        cmd.arg("--release")
    } else {
        cmd
    };
    let cmd = if config.verbose {
        cmd.arg("--verbose")
    } else {
        cmd.arg("--quiet")
    };

    let cmd = if config.custom_profile.is_some() {
        let custom_profile = config.custom_profile.as_ref().unwrap();
        cmd.arg("--profile").arg(custom_profile)
    } else {
        cmd
    };

    let cmd = if config.features.is_some() {
        let features_str = config.features.as_ref().unwrap().join(" ");
        cmd.arg("--features").arg(features_str)
    } else {
        cmd
    };

    let cmd = cmd.args(&config.cargo_args);

    let cmd = match executable {
        ExecutableType::Binary(name) => cmd.arg("--bin").arg(name),
        ExecutableType::Lib(name) => cmd.arg("--lib").arg(name),
        ExecutableType::Example(name) => cmd.arg("--example").arg(name),
    };

    let warning_messages = prettier_build(cmd)?;

    // [2] Establish the output directory structure
    let bindgen_outdir = out_dir.join("assets").join("dioxus");

    let build_target = if config.custom_profile.is_some() {
        let build_profile = config.custom_profile.as_ref().unwrap();
        if build_profile == "dev" {
            "debug"
        } else {
            build_profile
        }
    } else if config.release {
        "release"
    } else {
        "debug"
    };

    let input_path = match executable {
        ExecutableType::Binary(name) | ExecutableType::Lib(name) => target_dir
            .join(format!("wasm32-unknown-unknown/{}", build_target))
            .join(format!("{}.wasm", name)),

        ExecutableType::Example(name) => target_dir
            .join(format!("wasm32-unknown-unknown/{}/examples", build_target))
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
        return Err(Error::BuildFailed("Bindgen build failed! \nThis is probably due to the Bindgen version, dioxus-cli using `0.2.81` Bindgen crate.".to_string()));
    }

    // check binaryen:wasm-opt tool
    let dioxus_tools = dioxus_config.application.tools.clone();
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
                        if config.release {
                            args.push("-Oz");
                        }
                        binaryen.call("wasm-opt", args)?;
                    }
                }
            }
        } else {
            log::warn!(
                "Binaryen tool not found, you can use `dx tool add binaryen` to install it."
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

                if config.release {
                    args.push("--minify");
                }

                tailwind.call("tailwindcss", args)?;
            }
        } else {
            log::warn!(
                "Tailwind tool not found, you can use `dx tool add tailwindcss` to install it."
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
        for entry in std::fs::read_dir(asset_dir)? {
            let path = entry?.path();
            if path.is_file() {
                std::fs::copy(&path, out_dir.join(path.file_name().unwrap()))?;
            } else {
                match fs_extra::dir::copy(&path, &out_dir, &copy_options) {
                    Ok(_) => {}
                    Err(_e) => {
                        log::warn!("Error copying dir: {}", _e);
                    }
                }
                for ignore in &ignore_files {
                    let ignore = ignore.strip_prefix(&config.asset_dir()).unwrap();
                    let ignore = out_dir.join(ignore);
                    if ignore.is_file() {
                        std::fs::remove_file(ignore)?;
                    }
                }
            }
        }
    }

    let assets = if !skip_assets {
        let assets = asset_manifest(config);
        process_assets(config, &assets)?;
        Some(assets)
    } else {
        None
    };

    Ok(BuildResult {
        warnings: warning_messages,
        elapsed_time: t_start.elapsed().as_millis(),
        assets,
    })
}

/// Note: `rust_flags` argument is only used for the fullstack platform
/// (server).
pub fn build_desktop(
    config: &CrateConfig,
    _is_serve: bool,
    skip_assets: bool,
    rust_flags: Option<String>,
) -> Result<BuildResult> {
    log::info!("🚅 Running build [Desktop] command...");

    let t_start = std::time::Instant::now();
    let ignore_files = build_assets(config)?;
    let _guard = dioxus_cli_config::__private::save_config(config);
    let _manganis_support = ManganisSupportGuard::default();
    let _guard = AssetConfigDropGuard::new();

    let mut cmd = subprocess::Exec::cmd("cargo")
        .set_rust_flags(rust_flags)
        .env("CARGO_TARGET_DIR", &config.target_dir)
        .cwd(&config.crate_dir)
        .arg("build")
        .arg("--message-format=json");

    if config.release {
        cmd = cmd.arg("--release");
    }
    if config.verbose {
        cmd = cmd.arg("--verbose");
    } else {
        cmd = cmd.arg("--quiet");
    }

    if config.custom_profile.is_some() {
        let custom_profile = config.custom_profile.as_ref().unwrap();
        cmd = cmd.arg("--profile").arg(custom_profile);
    }

    if config.features.is_some() {
        let features_str = config.features.as_ref().unwrap().join(" ");
        cmd = cmd.arg("--features").arg(features_str);
    }

    if let Some(target) = &config.target {
        cmd = cmd.arg("--target").arg(target);
    }

    let target_platform = config.target.as_deref().unwrap_or("");

    cmd = cmd.args(&config.cargo_args);

    let cmd = match &config.executable {
        ExecutableType::Binary(name) => cmd.arg("--bin").arg(name),
        ExecutableType::Lib(name) => cmd.arg("--lib").arg(name),
        ExecutableType::Example(name) => cmd.arg("--example").arg(name),
    };

    let warning_messages = prettier_build(cmd)?;

    let release_type = match config.release {
        true => "release",
        false => "debug",
    };

    let file_name: String;
    let mut res_path = match &config.executable {
        ExecutableType::Binary(name) | ExecutableType::Lib(name) => {
            file_name = name.clone();
            config
                .target_dir
                .join(target_platform)
                .join(release_type)
                .join(name)
        }
        ExecutableType::Example(name) => {
            file_name = name.clone();
            config
                .target_dir
                .join(target_platform)
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

    if !config.out_dir().is_dir() {
        create_dir_all(config.out_dir())?;
    }
    copy(res_path, config.out_dir().join(target_file))?;

    // this code will copy all public file to the output dir
    if config.asset_dir().is_dir() {
        let copy_options = fs_extra::dir::CopyOptions {
            overwrite: true,
            skip_exist: false,
            buffer_size: 64000,
            copy_inside: false,
            content_only: false,
            depth: 0,
        };

        for entry in std::fs::read_dir(config.asset_dir())? {
            let path = entry?.path();
            if path.is_file() {
                std::fs::copy(&path, &config.out_dir().join(path.file_name().unwrap()))?;
            } else {
                match fs_extra::dir::copy(&path, &config.out_dir(), &copy_options) {
                    Ok(_) => {}
                    Err(e) => {
                        log::warn!("Error copying dir: {}", e);
                    }
                }
                for ignore in &ignore_files {
                    let ignore = ignore.strip_prefix(&config.asset_dir()).unwrap();
                    let ignore = config.out_dir().join(ignore);
                    if ignore.is_file() {
                        std::fs::remove_file(ignore)?;
                    }
                }
            }
        }
    }

    let assets = if !skip_assets {
        let assets = asset_manifest(config);
        // Collect assets
        process_assets(config, &assets)?;
        // Create the __assets_head.html file for bundling
        create_assets_head(config, &assets)?;
        Some(assets)
    } else {
        None
    };

    log::info!(
        "🚩 Build completed: [./{}]",
        config.dioxus_config.application.out_dir.clone().display()
    );

    println!("build desktop done");

    Ok(BuildResult {
        warnings: warning_messages,
        elapsed_time: t_start.elapsed().as_millis(),
        assets,
    })
}

fn prettier_build(cmd: subprocess::Exec) -> anyhow::Result<Vec<Diagnostic>> {
    let mut warning_messages: Vec<Diagnostic> = vec![];

    let mut pb = ProgressBar::new_spinner();
    pb.enable_steady_tick(Duration::from_millis(200));
    pb = PROGRESS_BARS.add(pb);
    pb.set_style(
        ProgressStyle::with_template("{spinner:.dim.bold} {wide_msg}")
            .unwrap()
            .tick_chars("/|\\- "),
    );
    pb.set_message("💼 Waiting to start building the project...");

    let stdout = cmd.detached().stream_stdout()?;
    let reader = std::io::BufReader::new(stdout);

    for message in cargo_metadata::Message::parse_stream(reader) {
        match message.unwrap() {
            Message::CompilerMessage(msg) => {
                let message = msg.message;
                match message.level {
                    cargo_metadata::diagnostic::DiagnosticLevel::Error => {
                        return {
                            Err(anyhow::anyhow!(message
                                .rendered
                                .unwrap_or("Unknown".into())))
                        };
                    }
                    cargo_metadata::diagnostic::DiagnosticLevel::Warning => {
                        warning_messages.push(message.clone());
                    }
                    _ => {}
                }
            }
            Message::CompilerArtifact(artifact) => {
                pb.set_message(format!("⚙️ Compiling {} ", artifact.package_id));
                pb.tick();
            }
            Message::BuildScriptExecuted(script) => {
                let _package_id = script.package_id.to_string();
            }
            Message::BuildFinished(finished) => {
                if finished.success {
                    log::info!("👑 Build done.");
                } else {
                    std::process::exit(1);
                }
            }
            _ => {
                // Unknown message
            }
        }
    }
    Ok(warning_messages)
}

pub fn gen_page(config: &CrateConfig, manifest: Option<&AssetManifest>, serve: bool) -> String {
    let _guard = AssetConfigDropGuard::new();

    let crate_root = crate_root().unwrap();
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

    let resources = config.dioxus_config.web.resource.clone();

    let mut style_list = resources.style.unwrap_or_default();
    let mut script_list = resources.script.unwrap_or_default();

    if serve {
        let mut dev_style = resources.dev.style.clone();
        let mut dev_script = resources.dev.script.clone();
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
        .dioxus_config
        .application
        .tools
        .clone()
        .contains_key("tailwindcss")
    {
        style_str.push_str("<link rel=\"stylesheet\" href=\"/{base_path}/tailwind.css\">\n");
    }
    if let Some(manifest) = manifest {
        style_str.push_str(&manifest.head());
    }

    replace_or_insert_before("{style_include}", &style_str, "</head", &mut html);

    let mut script_str = String::new();
    for script in script_list {
        script_str.push_str(&format!(
            "<script src=\"{}\"></script>\n",
            &script.to_str().unwrap(),
        ))
    }

    replace_or_insert_before("{script_include}", &script_str, "</body", &mut html);

    if serve {
        html += &format!(
            "<script>{}</script>",
            include_str!("./assets/autoreload.js")
        );
    }

    let base_path = match &config.dioxus_config.web.app.base_path {
        Some(path) => path,
        None => ".",
    };
    let app_name = &config.dioxus_config.application.name;
    // Check if a script already exists
    if html.contains("{app_name}") && html.contains("{base_path}") {
        html = html.replace("{app_name}", app_name);

        html = html.replace("{base_path}", base_path);
    } else {
        // If not, insert the script
        html = html.replace(
            "</body",
            &format!(
                r#"<script type="module">
    import init from "/{base_path}/assets/dioxus/{app_name}.js";
    init("/{base_path}/assets/dioxus/{app_name}_bg.wasm").then(wasm => {{
      if (wasm.__wbindgen_start == undefined) {{
        wasm.main();
      }}
    }});
    </script>
    </body"#
            ),
        );
    }

    let title = config.dioxus_config.web.app.title.clone();

    replace_or_insert_before("{app_title}", &title, "</title", &mut html);

    html
}

fn replace_or_insert_before(
    replace: &str,
    with: &str,
    or_insert_before: &str,
    content: &mut String,
) {
    if content.contains(replace) {
        *content = content.replace(replace, with);
    } else {
        *content = content.replace(or_insert_before, &format!("{}{}", with, or_insert_before));
    }
}

// this function will build some assets file
// like sass tool resources
// this function will return a array which file don't need copy to out_dir.
fn build_assets(config: &CrateConfig) -> Result<Vec<PathBuf>> {
    let mut result = vec![];

    let dioxus_config = &config.dioxus_config;
    let dioxus_tools = dioxus_config.application.tools.clone();

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
                        let asset_dir = config.asset_dir().clone();
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
                                            .out_dir()
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
                        let path = config.asset_dir().join(relative_path);
                        let out_file =
                            format!("{}.css", path.file_stem().unwrap().to_str().unwrap());
                        let target_path = config
                            .out_dir()
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
                            let path = config.asset_dir().join(relative_path);
                            let out_file =
                                format!("{}.css", path.file_stem().unwrap().to_str().unwrap());
                            let target_path = config
                                .out_dir()
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
