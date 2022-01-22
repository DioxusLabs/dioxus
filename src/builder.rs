use crate::{
    config::{CrateConfig, ExecutableType},
    error::{Error, Result},
};
use std::{
    io::Write,
    process::Command,
};
use wasm_bindgen_cli_support::Bindgen;

pub fn build(config: &CrateConfig) -> Result<()> {
    /*
    [1] Build the project with cargo, generating a wasm32-unknown-unknown target (is there a more specific, better target to leverage?)
    [2] Generate the appropriate build folders
    [3] Wasm-bindgen the .wasm fiile, and move it into the {builddir}/modules/xxxx/xxxx_bg.wasm
    [4] Wasm-opt the .wasm file with whatever optimizations need to be done
    [5] Link up the html page to the wasm module
    */

    let CrateConfig {
        out_dir,
        crate_dir,
        target_dir,
        static_dir,
        executable,
        ..
    } = config;

    let t_start = std::time::Instant::now();

    // [1] Build the .wasm module
    log::info!("🚅 Running build commands...");
    let mut cmd = Command::new("cargo");
    cmd.current_dir(&crate_dir)
        .arg("build")
        .arg("--target")
        .arg("wasm32-unknown-unknown")
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
    ;

    if config.release {
        cmd.arg("--release");
    }

    match executable {
        ExecutableType::Binary(name) => cmd.arg("--bin").arg(name),
        ExecutableType::Lib(name) => cmd.arg("--lib").arg(name),
        ExecutableType::Example(name) => cmd.arg("--example").arg(name),
    };

    let output = cmd.output()?;

    if output.status.success() {
        std::io::stdout().write_all(&output.stdout).unwrap();
    } else {
        log::error!("Build failed!");
        let reason = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(Error::BuildFailed(reason));
    }

    // [2] Establish the output directory structure
    let bindgen_outdir = out_dir.join("assets");

    // [3] Bindgen the final binary for use easy linking
    let mut bindgen_builder = Bindgen::new();

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

    bindgen_builder
        .input_path(input_path)
        .web(true)?
        .debug(true)
        .demangle(true)
        .keep_debug(true)
        .remove_name_section(false)
        .remove_producers_section(false)
        .out_name("module")
        .generate(&bindgen_outdir)?;

    // [4]
    // TODO: wasm-opt

    // [5] Generate the html file with the module name
    // TODO: support names via options

    // log::info!("Writing to '{:#?}' directory...", out_dir);
    let copy_options = fs_extra::dir::CopyOptions::new();
    if static_dir.is_dir() {
        match fs_extra::dir::copy(static_dir, out_dir, &copy_options) {
            Ok(_) => {}
            Err(_e) => {
                log::warn!("Error copying dir: {}", _e);
            }
        }
    }

    let t_end = std::time::Instant::now();
    log::info!("🏁 Done in {}ms!", (t_end - t_start).as_millis());
    Ok(())
}

pub fn gen_page(module: &str) -> String {
    format!(
        r#"
<html>
  <head>
    <meta content="text/html;charset=utf-8" http-equiv="Content-Type" />
    <meta charset="UTF-8" />
  </head>
  <body>
    <div id="main">
    </div>
    <!-- Note the usage of `type=module` here as this is an ES6 module -->
    <script type="module">
      import init from "{}";
      init("./assets/module_bg.wasm");
    </script>
    <div id="dioxusroot"> </div>
  </body>
</html>
"#,
        module
    )
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
