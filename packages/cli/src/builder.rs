use crate::{
    assets::{
        asset_manifest, copy_assets_dir, create_assets_head, pre_compress_folder, process_assets,
        AssetConfigDropGuard,
    },
    error::{Error, Result},
};
use anyhow::Context;
use cargo_metadata::{diagnostic::Diagnostic, Message};
use dioxus_cli_config::{crate_root, CrateConfig, ExecutableType, WasmOptLevel};
use indicatif::{ProgressBar, ProgressStyle};
use lazy_static::lazy_static;
use manganis_cli_support::{AssetManifest, ManganisSupportGuard};
use std::{
    env,
    fs::{copy, create_dir_all, File},
    io::{self, IsTerminal, Read},
    panic,
    path::PathBuf,
    process::Command,
    time::Duration,
};
use wasm_bindgen_cli_support::Bindgen;

lazy_static! {
    static ref PROGRESS_BARS: indicatif::MultiProgress = indicatif::MultiProgress::new();
}

#[derive(Debug, Clone)]
pub struct BuildResult {
    pub warnings: Vec<Diagnostic>,
    pub executable: Option<PathBuf>,
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
pub fn build_web(
    config: &CrateConfig,
    skip_assets: bool,
    rust_flags: Option<String>,
) -> Result<BuildResult> {
    // [1] Build the project with cargo, generating a wasm32-unknown-unknown target (is there a more specific, better target to leverage?)
    // [2] Generate the appropriate build folders
    // [3] Wasm-bindgen the .wasm file, and move it into the {builddir}/modules/xxxx/xxxx_bg.wasm
    // [4] Wasm-opt the .wasm file with whatever optimizations need to be done

    let CrateConfig {
        crate_dir,
        target_dir,
        executable,
        dioxus_config,
        ..
    } = config;
    let out_dir = config.out_dir();

    let _guard = AssetConfigDropGuard::new();
    let _manganis_support = ManganisSupportGuard::default();

    let t_start = std::time::Instant::now();
    let _guard = dioxus_cli_config::__private::save_config(config);

    // [1] Build the .wasm module
    tracing::info!("🚅 Running build command...");

    // If the user has rustup, we can check if the wasm32-unknown-unknown target is installed
    // Otherwise we can just assume it is installed - which i snot great...
    // Eventually we can poke at the errors and let the user know they need to install the target
    if let Ok(wasm_check_command) = Command::new("rustup").args(["show"]).output() {
        let wasm_check_output = String::from_utf8(wasm_check_command.stdout).unwrap();
        if !wasm_check_output.contains("wasm32-unknown-unknown") {
            tracing::info!("wasm32-unknown-unknown target not detected, installing..");
            let _ = Command::new("rustup")
                .args(["target", "add", "wasm32-unknown-unknown"])
                .output()?;
        }
    }

    let cmd = subprocess::Exec::cmd("cargo")
        .set_rust_flags(rust_flags)
        .env("CARGO_TARGET_DIR", target_dir)
        .cwd(crate_dir)
        .arg("build")
        .arg("--target")
        .arg("wasm32-unknown-unknown")
        .arg("--message-format=json-render-diagnostics");

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

    let CargoBuildResult {
        warnings,
        output_location,
    } = prettier_build(cmd)?;
    let output_location = output_location.context("No output location found")?;

    // [2] Establish the output directory structure
    let bindgen_outdir = out_dir.join("assets").join("dioxus");

    let input_path = output_location.with_extension("wasm");

    tracing::info!("Running wasm-bindgen");
    let run_wasm_bindgen = || {
        // [3] Bindgen the final binary for use easy linking
        let mut bindgen_builder = Bindgen::new();

        let keep_debug = dioxus_config.web.wasm_opt.debug || (!config.release);

        bindgen_builder
            .input_path(&input_path)
            .web(true)
            .unwrap()
            .debug(keep_debug)
            .demangle(keep_debug)
            .keep_debug(keep_debug)
            .reference_types(true)
            .remove_name_section(!keep_debug)
            .remove_producers_section(!keep_debug)
            .out_name(&dioxus_config.application.name)
            .generate(&bindgen_outdir)
            .unwrap();
    };
    let bindgen_result = panic::catch_unwind(run_wasm_bindgen);

    // WASM bindgen requires the exact version of the bindgen schema to match the version the CLI was built with
    // If we get an error, we can try to recover by pinning the user's wasm-bindgen version to the version we used
    if let Err(err) = bindgen_result {
        tracing::error!("Bindgen build failed: {:?}", err);
        update_wasm_bindgen_version()?;
        run_wasm_bindgen();
    }

    // Run wasm-opt if this is a release build
    if config.release {
        tracing::info!("Running optimization with wasm-opt...");
        let mut options = match dioxus_config.web.wasm_opt.level {
            WasmOptLevel::Z => wasm_opt::OptimizationOptions::new_optimize_for_size_aggressively(),
            WasmOptLevel::S => wasm_opt::OptimizationOptions::new_optimize_for_size(),
            WasmOptLevel::Zero => wasm_opt::OptimizationOptions::new_opt_level_0(),
            WasmOptLevel::One => wasm_opt::OptimizationOptions::new_opt_level_1(),
            WasmOptLevel::Two => wasm_opt::OptimizationOptions::new_opt_level_2(),
            WasmOptLevel::Three => wasm_opt::OptimizationOptions::new_opt_level_3(),
            WasmOptLevel::Four => wasm_opt::OptimizationOptions::new_opt_level_4(),
        };
        let wasm_file = bindgen_outdir.join(format!("{}_bg.wasm", dioxus_config.application.name));
        let old_size = wasm_file.metadata()?.len();
        options
            // WASM bindgen relies on reference types
            .enable_feature(wasm_opt::Feature::ReferenceTypes)
            .debug_info(dioxus_config.web.wasm_opt.debug)
            .run(&wasm_file, &wasm_file)
            .map_err(|err| Error::Other(anyhow::anyhow!(err)))?;
        let new_size = wasm_file.metadata()?.len();
        tracing::info!(
            "wasm-opt reduced WASM size from {} to {} ({:2}%)",
            old_size,
            new_size,
            (new_size as f64 - old_size as f64) / old_size as f64 * 100.0
        );
    }

    // If pre-compressing is enabled, we can pre_compress the wasm-bindgen output
    pre_compress_folder(&bindgen_outdir, config.should_pre_compress_web_assets())?;

    // this code will copy all public file to the output dir
    copy_assets_dir(config, dioxus_cli_config::Platform::Web)?;

    let assets = if !skip_assets {
        tracing::info!("Processing assets");
        let assets = asset_manifest(executable.executable(), config);
        process_assets(config, &assets)?;
        Some(assets)
    } else {
        None
    };

    Ok(BuildResult {
        warnings,
        executable: Some(output_location),
        elapsed_time: t_start.elapsed().as_millis(),
        assets,
    })
}

// Attempt to automatically recover from a bindgen failure by updating the wasm-bindgen version
fn update_wasm_bindgen_version() -> Result<()> {
    let cli_bindgen_version = wasm_bindgen_shared::version();
    tracing::info!("Attempting to recover from bindgen failure by setting the wasm-bindgen version to {cli_bindgen_version}...");

    let output = Command::new("cargo")
        .args([
            "update",
            "-p",
            "wasm-bindgen",
            "--precise",
            &cli_bindgen_version,
        ])
        .output();
    let mut error_message = None;
    if let Ok(output) = output {
        if output.status.success() {
            tracing::info!("Successfully updated wasm-bindgen to {cli_bindgen_version}");
            return Ok(());
        } else {
            error_message = Some(output);
        }
    }

    if let Some(output) = error_message {
        tracing::error!("Failed to update wasm-bindgen: {:#?}", output);
    }

    Err(Error::BuildFailed(format!("WASM bindgen build failed!\nThis is probably due to the Bindgen version, dioxus-cli is using `{cli_bindgen_version}` which is not compatible with your crate.\nPlease reinstall the dioxus cli to fix this issue.\nYou can reinstall the dioxus cli by running `cargo install dioxus-cli --force` and then rebuild your project")))
}

/// Note: `rust_flags` argument is only used for the fullstack platform
/// (server).
pub fn build_desktop(
    config: &CrateConfig,
    _is_serve: bool,
    skip_assets: bool,
    rust_flags: Option<String>,
) -> Result<BuildResult> {
    tracing::info!("🚅 Running build [Desktop] command...");

    let t_start = std::time::Instant::now();
    let _guard = dioxus_cli_config::__private::save_config(config);
    let _manganis_support = ManganisSupportGuard::default();
    let _guard = AssetConfigDropGuard::new();

    let mut cmd = subprocess::Exec::cmd("cargo")
        .set_rust_flags(rust_flags)
        .env("CARGO_TARGET_DIR", &config.target_dir)
        .cwd(&config.crate_dir)
        .arg("build")
        .arg("--message-format=json-render-diagnostics");

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

    cmd = cmd.args(&config.cargo_args);

    let cmd = match &config.executable {
        ExecutableType::Binary(name) => cmd.arg("--bin").arg(name),
        ExecutableType::Lib(name) => cmd.arg("--lib").arg(name),
        ExecutableType::Example(name) => cmd.arg("--example").arg(name),
    };

    let warning_messages = prettier_build(cmd)?;

    let file_name: String = config.executable.executable().unwrap().to_string();

    let target_file = if cfg!(windows) {
        format!("{}.exe", &file_name)
    } else {
        file_name
    };

    if !config.out_dir().is_dir() {
        create_dir_all(config.out_dir())?;
    }
    let output_path = config.out_dir().join(target_file);
    if let Some(res_path) = &warning_messages.output_location {
        copy(res_path, &output_path)?;
    }

    copy_assets_dir(config, dioxus_cli_config::Platform::Desktop)?;

    let assets = if !skip_assets {
        tracing::info!("Processing assets");
        let assets = asset_manifest(config.executable.executable(), config);
        // Collect assets
        process_assets(config, &assets)?;
        // Create the __assets_head.html file for bundling
        create_assets_head(config, &assets)?;
        Some(assets)
    } else {
        None
    };

    tracing::info!(
        "🚩 Build completed: [./{}]",
        config.dioxus_config.application.out_dir.clone().display()
    );

    println!("build desktop done");

    Ok(BuildResult {
        warnings: warning_messages.warnings,
        executable: Some(output_path),
        elapsed_time: t_start.elapsed().as_millis(),
        assets,
    })
}

struct CargoBuildResult {
    warnings: Vec<Diagnostic>,
    output_location: Option<PathBuf>,
}

struct Outputter {
    progress_bar: Option<ProgressBar>,
}

impl Outputter {
    pub fn new() -> Self {
        let stdout = io::stdout().lock();

        let mut myself = Self { progress_bar: None };

        if stdout.is_terminal() {
            let mut pb = ProgressBar::new_spinner();
            pb.enable_steady_tick(Duration::from_millis(200));
            pb = PROGRESS_BARS.add(pb);
            pb.set_style(
                ProgressStyle::with_template("{spinner:.dim.bold} {wide_msg}")
                    .unwrap()
                    .tick_chars("/|\\- "),
            );

            myself.progress_bar = Some(pb);
        }

        myself
    }

    pub fn println(&self, msg: impl ToString) {
        let msg = msg.to_string();
        if let Some(pb) = &self.progress_bar {
            pb.set_message(msg)
        } else {
            println!("{msg}");
        }
    }

    pub fn finish_with_message(&self, msg: impl ToString) {
        let msg = msg.to_string();
        if let Some(pb) = &self.progress_bar {
            pb.finish_with_message(msg)
        } else {
            println!("{msg}");
        }
    }
}

fn prettier_build(cmd: subprocess::Exec) -> anyhow::Result<CargoBuildResult> {
    let mut warning_messages: Vec<Diagnostic> = vec![];

    let output = Outputter::new();
    output.println("💼 Waiting to start building the project...");

    let stdout = cmd.detached().stream_stdout()?;
    let reader = std::io::BufReader::new(stdout);
    let mut output_location = None;

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
                output.println(format!("⚙ Compiling {} ", artifact.package_id));
                if let Some(executable) = artifact.executable {
                    output_location = Some(executable.into());
                }
            }
            Message::BuildScriptExecuted(script) => {
                let _package_id = script.package_id.to_string();
            }
            Message::BuildFinished(finished) => {
                if finished.success {
                    output.finish_with_message("👑 Build done.");
                } else {
                    output.finish_with_message("❌ Build failed.");
                    return Err(anyhow::anyhow!("Build failed"));
                }
            }
            _ => {
                // Unknown message
            }
        }
    }

    Ok(CargoBuildResult {
        warnings: warning_messages,
        output_location,
    })
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
        html += &format!("<script>{}</script>", dioxus_hot_reload::RECONNECT_SCRIPT);
    }

    let base_path = match &config.dioxus_config.web.app.base_path {
        Some(path) => path.trim_matches('/'),
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

        // And try to insert preload links for the wasm and js files
        html = html.replace(
            "</head",
            &format!(
                r#"<link rel="preload" href="/{base_path}/assets/dioxus/{app_name}_bg.wasm" as="fetch" type="application/wasm" crossorigin="">
                    <link rel="preload" href="/{base_path}/assets/dioxus/{app_name}.js" as="script">
    </head"#
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
