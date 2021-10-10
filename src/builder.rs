use crate::{
    cli::BuildOptions,
    config::{Config, ExecutableType},
    error::Result,
};
use log::{info, warn};
use std::{io::Write, process::Command};
use wasm_bindgen_cli_support::Bindgen;

pub struct BuildConfig {}
impl Into<BuildConfig> for BuildOptions {
    fn into(self) -> BuildConfig {
        BuildConfig {}
    }
}
impl Default for BuildConfig {
    fn default() -> Self {
        Self {}
    }
}

pub fn build(config: &Config, _build_config: &BuildConfig) -> Result<()> {
    /*
    [1] Build the project with cargo, generating a wasm32-unknown-unknown target (is there a more specific, better target to leverage?)
    [2] Generate the appropriate build folders
    [3] Wasm-bindgen the .wasm fiile, and move it into the {builddir}/modules/xxxx/xxxx_bg.wasm
    [4] Wasm-opt the .wasm file with whatever optimizations need to be done
    [5] Link up the html page to the wasm module
    */

    let Config {
        out_dir,
        crate_dir,
        target_dir,
        static_dir,
        executable,
        ..
    } = config;

    let t_start = std::time::Instant::now();

    // [1] Build the .wasm module
    info!("Running build commands...");
    let mut cmd = Command::new("cargo");
    cmd.current_dir(&crate_dir)
        .arg("build")
        .arg("--target")
        .arg("wasm32-unknown-unknown")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    if config.release {
        cmd.arg("--release");
    }

    match executable {
        ExecutableType::Binary(name) => cmd.arg("--bin").arg(name),
        ExecutableType::Lib(name) => cmd.arg("--lib").arg(name),
        ExecutableType::Example(name) => cmd.arg("--example").arg(name),
    };

    let mut child = cmd.spawn()?;
    let _err_code = child.wait()?;

    info!("Build complete!");

    // [2] Establish the output directory structure
    let bindgen_outdir = out_dir.join("wasm");

    // [3] Bindgen the final binary for use easy linking
    let mut bindgen_builder = Bindgen::new();

    let release_type = match config.release {
        true => "release",
        false => "debug",
    };

    let input_path = match executable {
        ExecutableType::Binary(name) | ExecutableType::Lib(name) => target_dir
            // .join("wasm32-unknown-unknown/release")
            .join(format!("wasm32-unknown-unknown/{}", release_type))
            .join(format!("{}.wasm", name)),

        ExecutableType::Example(name) => target_dir
            // .join("wasm32-unknown-unknown/release/examples")
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
    info!("Writing to '{:#?}' directory...", out_dir);
    let mut file = std::fs::File::create(out_dir.join("index.html"))?;
    file.write_all(gen_page("./wasm/module.js").as_str().as_bytes())?;

    let copy_options = fs_extra::dir::CopyOptions::new();
    match fs_extra::dir::copy(static_dir, out_dir, &copy_options) {
        Ok(_) => {}
        Err(_e) => {
            warn!("Error copying dir");
        }
    }

    let t_end = std::time::Instant::now();
    log::info!("Done in {}ms! 🎉", (t_end - t_start).as_millis());
    Ok(())
}

fn gen_page(module: &str) -> String {
    format!(
        r#"
<html>
  <head>
    <meta content="text/html;charset=utf-8" http-equiv="Content-Type" />
    <meta charset="UTF-8" />
  </head>
  <body>
    <!-- Note the usage of `type=module` here as this is an ES6 module -->
    <script type="module">
      import init from "{}";
      init("./wasm/module_bg.wasm");
    </script>
    <div id="dioxusroot"> </div>
  </body>
</html>
"#,
        module
    )
}
