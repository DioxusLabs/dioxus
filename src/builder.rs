use crate::{
    config::{CrateConfig, ExecutableType},
    error::{Error, Result},
    DioxusConfig,
};
use std::{
    fs::{copy, create_dir_all, remove_dir_all, File},
    io::Read,
    panic,
    path::PathBuf,
    process::Command,
};
use wasm_bindgen_cli_support::Bindgen;

pub fn build(config: &CrateConfig) -> Result<()> {
    // [1] Build the project with cargo, generating a wasm32-unknown-unknown target (is there a more specific, better target to leverage?)
    // [2] Generate the appropriate build folders
    // [3] Wasm-bindgen the .wasm fiile, and move it into the {builddir}/modules/xxxx/xxxx_bg.wasm
    // [4] Wasm-opt the .wasm file with whatever optimizations need to be done
    // [5] Link up the html page to the wasm module

    let CrateConfig {
        out_dir,
        crate_dir,
        target_dir,
        asset_dir,
        executable,
        dioxus_config,
        ..
    } = config;

    let t_start = std::time::Instant::now();

    // [1] Build the .wasm module
    log::info!("🚅 Running build command...");
    let mut cmd = Command::new("cargo");
    cmd.current_dir(&crate_dir)
        .arg("build")
        .arg("--target")
        .arg("wasm32-unknown-unknown")
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit());

    if config.release {
        cmd.arg("--release");
    }

    match executable {
        ExecutableType::Binary(name) => cmd.arg("--bin").arg(name),
        ExecutableType::Lib(name) => cmd.arg("--lib").arg(name),
        ExecutableType::Example(name) => cmd.arg("--example").arg(name),
    };

    let output = cmd.output()?;

    if !output.status.success() {
        log::error!("Build failed!");
        let reason = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(Error::BuildFailed(reason));
    }

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
        log::error!("Bindgen build failed! \nThis is probably due to the Bindgen version, dioxus-cli using `0.2.79` Bindgen crate.");
        return Ok(());
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
                    let target_file = out_dir
                        .join("assets")
                        .join("dioxus")
                        .join(format!("{}_bg.wasm", dioxus_config.application.name));
                    if target_file.is_file() {
                        binaryen.call(
                            "wasm-opt",
                            vec![
                                target_file.to_str().unwrap(),
                                "-o",
                                target_file.to_str().unwrap(),
                            ],
                        )?;
                    }
                }
            }
        } else {
            log::warn!(
                "Binaryen tool not found, you can use `dioxus tool add binaryen` to install it."
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
            }
        }
    }

    let t_end = std::time::Instant::now();
    log::info!("🏁 Done in {}ms!", (t_end - t_start).as_millis());
    Ok(())
}

pub fn build_desktop(config: &CrateConfig, is_serve: bool) -> Result<()> {
    log::info!("🚅 Running build [Desktop] command...");

    let mut cmd = Command::new("cargo");
    cmd.current_dir(&config.crate_dir)
        .arg("build")
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit());

    if config.release {
        cmd.arg("--release");
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
                config
                    .target_dir
                    .join(release_type)
                    .join(name)
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
