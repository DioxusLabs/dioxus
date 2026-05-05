//! ### Web:
//!
//! Create a folder that is somewhat similar to an app-image (exe + asset)
//! The server is dropped into the `web` folder, even if there's no `public` folder.
//! If there's no server (SPA), we still use the `web` folder, but it only contains the
//! public folder.
//!
//! ```
//! web/
//!     server
//!     assets/
//!     public/
//!         index.html
//!         wasm/
//!            app.wasm
//!            glue.js
//!            snippets/
//!                ...
//!         assets/
//!            logo.png
//! ```
//!
//! ### Linux:
//!
//! <https://docs.appimage.org/reference/appdir.html#ref-appdir>
//! current_exe.join("Assets")
//! ```
//! app.appimage/
//!     AppRun
//!     app.desktop
//!     package.json
//!     assets/
//!         logo.png
//! ```

use crate::{BuildContext, BundleFormat, Result, TraceSrc, WasmBindgen, WasmOptConfig};
use crate::{
    BuildMode, BuildRequest,
    opt::{AppManifest, js_is_module},
};
use anyhow::Context;
use dioxus_cli_config::format_base_path_meta_element;
use manganis::AssetOptions;
use manganis_core::AssetVariant;
use std::{
    io::Write,
    path::{Path, PathBuf},
};
use uuid::Uuid;

impl BuildRequest {
    pub async fn verify_web_tooling(&self) -> Result<()> {
        // Wasm bindgen
        let krate_bindgen_version =
            self.workspace
                .wasm_bindgen_version()
                .ok_or(anyhow::anyhow!(
                    "failed to detect wasm-bindgen version, unable to proceed"
                ))?;

        WasmBindgen::verify_install(&krate_bindgen_version).await?;

        // esbuild is used for JS asset processing
        let _esbuild_path = crate::esbuild::Esbuild::get_or_install().await?;

        Ok(())
    }

    /// Bundle the web app
    /// - Run wasm-bindgen
    /// - Bundle split
    /// - Run wasm-opt
    /// - Register the .wasm and .js files with the asset system
    pub async fn bundle_web(
        &self,
        ctx: &BuildContext,
        exe: &Path,
        assets: &mut AppManifest,
    ) -> Result<()> {
        use crate::{wasm_bindgen::WasmBindgen, wasm_opt};
        use std::fmt::Write;

        // Locate the output of the build files and the bindgen output
        // We'll fill these in a second if they don't already exist
        let bindgen_outdir = self.wasm_bindgen_out_dir();
        let post_bindgen_wasm = self.wasm_bindgen_wasm_output_file();
        let should_bundle_split: bool = self.wasm_split;
        let bindgen_version = self
            .workspace
            .wasm_bindgen_version()
            .expect("this should have been checked by tool verification");

        // Prepare any work dirs
        _ = std::fs::remove_dir_all(&bindgen_outdir);
        std::fs::create_dir_all(&bindgen_outdir)?;

        // Lift the internal functions to exports
        if ctx.mode == BuildMode::Fat {
            let unprocessed = std::fs::read(exe)?;
            let all_exported_bytes = crate::build::prepare_wasm_base_module(&unprocessed)?;
            std::fs::write(exe, all_exported_bytes)?;
        }

        // Prepare our configuration
        //
        // we turn on debug symbols in dev mode
        //
        // We leave demangling to false since it's faster and these tools seem to prefer the raw symbols.
        // todo(jon): investigate if the chrome extension needs them demangled or demangles them automatically.
        let keep_debug = self.config.web.wasm_opt.debug
            || self.debug_symbols
            || self.wasm_split
            || !self.release
            || ctx.mode == BuildMode::Fat;
        let keep_names = self.config.web.wasm_opt.keep_names
            || self.keep_names
            || self.wasm_split
            || ctx.mode == BuildMode::Fat;
        let demangle = false;
        let wasm_opt_options = WasmOptConfig {
            memory_packing: self.wasm_split,
            debug: self.debug_symbols,
            ..self.config.web.wasm_opt.clone()
        };

        // Run wasm-bindgen. Some of the options are not "optimal" but will be fixed up by wasm-opt
        //
        // There's performance implications here. Running with --debug is slower than without
        // We're keeping around lld sections and names but wasm-opt will fix them
        // todo(jon): investigate a good balance of wiping debug symbols during dev (or doing a double build?)
        ctx.status_wasm_bindgen_start();
        tracing::debug!(dx_src = ?TraceSrc::Bundle, "Running wasm-bindgen");
        let start = std::time::Instant::now();
        WasmBindgen::new(&bindgen_version)
            .input_path(exe)
            .target("web")
            .debug(keep_debug)
            .demangle(demangle)
            .keep_debug(keep_debug)
            .keep_lld_exports(true)
            .out_name(self.executable_name())
            .out_dir(&bindgen_outdir)
            .remove_name_section(!keep_names)
            .remove_producers_section(!keep_names)
            .run()
            .await
            .context("Failed to generate wasm-bindgen bindings")?;
        tracing::debug!(dx_src = ?TraceSrc::Bundle, "wasm-bindgen complete in {:?}", start.elapsed());

        // Run bundle splitting if the user has requested it
        // It's pretty expensive but because of rayon should be running separate threads, hopefully
        // not blocking this thread. Dunno if that's true
        if should_bundle_split {
            ctx.status_splitting_bundle();

            // Load the contents of these binaries since we need both of them
            // We're going to use the default makeLoad glue from wasm-split
            let original = std::fs::read(exe)?;
            let bindgened = std::fs::read(&post_bindgen_wasm)?;
            let mut glue = wasm_split_cli::MAKE_LOAD_JS.to_string();

            // Run the emitter
            let splitter = wasm_split_cli::Splitter::new(&original, &bindgened);
            let modules = splitter
                .context("Failed to parse wasm for splitter")?
                .emit()
                .context("Failed to emit wasm split modules")?;

            // Write the chunks that contain shared imports
            // These will be in the format of chunk_0_modulename.wasm - this is hardcoded in wasm-split
            tracing::debug!("Writing split chunks to disk");
            for (idx, chunk) in modules.chunks.iter().enumerate() {
                let path = bindgen_outdir.join(format!("chunk_{}_{}.wasm", idx, chunk.module_name));
                wasm_opt::write_wasm(&chunk.bytes, &path, &wasm_opt_options).await?;
                writeln!(
                    glue,
                    "export const __wasm_split_load_chunk_{idx} = makeLoad(\"/{base_path}/assets/{url}\", [], fusedImports);",
                    base_path = self.base_path_or_default(),
                    url = assets
                        .register_asset(&path, AssetOptions::builder().into_asset_options())?
                        .bundled_path(),
                )?;
            }

            // Write the modules that contain the entrypoints
            tracing::debug!("Writing split modules to disk");
            for (idx, module) in modules.modules.iter().enumerate() {
                let comp_name = module
                    .component_name
                    .as_ref()
                    .context("generated bindgen module has no name?")?;

                let path = bindgen_outdir.join(format!("module_{idx}_{comp_name}.wasm"));
                wasm_opt::write_wasm(&module.bytes, &path, &wasm_opt_options).await?;

                let hash_id = module
                    .hash_id
                    .as_ref()
                    .context("generated wasm-split bindgen module has no hash id?")?;

                writeln!(
                    glue,
                    "export const __wasm_split_load_{module}_{hash_id}_{comp_name} = makeLoad(\"/{base_path}/assets/{url}\", [{deps}], fusedImports);",
                    module = module.module_name,
                    base_path = self.base_path_or_default(),
                    // Again, register this wasm with the asset system
                    url = assets
                        .register_asset(&path, AssetOptions::builder().into_asset_options())?
                        .bundled_path(),
                    // This time, make sure to write the dependencies of this chunk
                    // The names here are again, hardcoded in wasm-split - fix this eventually.
                    deps = module
                        .relies_on_chunks
                        .iter()
                        .map(|idx| format!("__wasm_split_load_chunk_{idx}"))
                        .collect::<Vec<_>>()
                        .join(", ")
                )?;
            }

            // Write the js binding
            // It's not registered as an asset since it will get included in the main.js file
            let js_output_path = bindgen_outdir.join("__wasm_split.js");
            std::fs::write(&js_output_path, &glue)?;

            // Make sure to write some entropy to the main.js file so it gets a new hash
            // If we don't do this, the main.js file will be cached and never pick up the chunk names
            let uuid = Uuid::new_v5(&Uuid::NAMESPACE_URL, glue.as_bytes());
            std::fs::OpenOptions::new()
                .append(true)
                .open(self.wasm_bindgen_js_output_file())
                .context("Failed to open main.js file")?
                .write_all(format!("/*{uuid}*/").as_bytes())?;

            // Write the main wasm_bindgen file and register it with the asset system
            // This will overwrite the file in place
            // We will wasm-opt it in just a second...
            std::fs::write(&post_bindgen_wasm, modules.main.bytes).unwrap();
        }

        if matches!(ctx.mode, BuildMode::Fat) {
            // add `export { __wbg_get_imports };` to the end of the wasmbindgen js file
            let mut js = std::fs::read(self.wasm_bindgen_js_output_file())?;
            writeln!(js, "\nexport {{ __wbg_get_imports }};")?;
            std::fs::write(self.wasm_bindgen_js_output_file(), js)?;
        }

        // Make sure to optimize the main wasm file if requested or if bundle splitting
        if should_bundle_split || self.release {
            ctx.status_optimizing_wasm();
            wasm_opt::optimize(&post_bindgen_wasm, &post_bindgen_wasm, &wasm_opt_options).await?;
        }

        if self.should_bundle_to_asset() {
            // Make sure to register the main wasm file with the asset system
            assets.register_asset(
                &post_bindgen_wasm,
                AssetOptions::builder().into_asset_options(),
            )?;
        }

        // Now that the wasm is registered as an asset, we can write the js glue shim
        self.write_js_glue_shim(assets)?;

        if self.should_bundle_to_asset() {
            // Register the main.js with the asset system so it bundles in the snippets and optimizes
            assets.register_asset(
                &self.wasm_bindgen_js_output_file(),
                AssetOptions::js()
                    .with_minify(true)
                    .with_preload(true)
                    .into_asset_options(),
            )?;
        }

        // Write the index.html file with the pre-configured contents we got from pre-rendering
        self.write_index_html(assets)?;

        Ok(())
    }

    fn write_js_glue_shim(&self, assets: &AppManifest) -> Result<()> {
        let wasm_path = self.bundled_wasm_path(assets);

        // Load and initialize wasm without requiring a separate javascript file.
        // This also allows using a strict Content-Security-Policy.
        let mut js = std::fs::OpenOptions::new()
            .append(true)
            .open(self.wasm_bindgen_js_output_file())?;
        let mut buf_writer = std::io::BufWriter::new(&mut js);
        writeln!(
            buf_writer,
            r#"
globalThis.__wasm_split_main_initSync = initSync;

// Actually perform the load
__wbg_init({{module_or_path: "/{}/{wasm_path}"}}).then((wasm) => {{
    // assign this module to be accessible globally
    globalThis.__dx_mainWasm = wasm;
    globalThis.__dx_mainInit = __wbg_init;
    globalThis.__dx_mainInitSync = initSync;
    globalThis.__dx___wbg_get_imports = __wbg_get_imports;

    if (wasm.__wbindgen_start == undefined) {{
        wasm.main();
    }}
}});
"#,
            self.base_path_or_default(),
        )?;

        Ok(())
    }

    /// Write the index.html file to the output directory. This must be called after the wasm and js
    /// assets are registered with the asset system if this is a release build.
    pub(crate) fn write_index_html(&self, assets: &AppManifest) -> Result<()> {
        let wasm_path = self.bundled_wasm_path(assets);
        let js_path = self.bundled_js_path(assets);

        // Write the index.html file with the pre-configured contents we got from pre-rendering
        std::fs::write(
            self.root_dir().join("index.html"),
            self.prepare_html(assets, &wasm_path, &js_path).unwrap(),
        )?;

        Ok(())
    }

    fn bundled_js_path(&self, assets: &AppManifest) -> String {
        let wasm_bindgen_js_out = self.wasm_bindgen_js_output_file();
        if self.should_bundle_to_asset() {
            let name = assets
                .get_first_asset_for_source(&wasm_bindgen_js_out)
                .expect("The js source must exist before creating index.html");
            format!("assets/{}", name.bundled_path())
        } else {
            format!(
                "wasm/{}",
                wasm_bindgen_js_out.file_name().unwrap().to_str().unwrap()
            )
        }
    }

    /// Get the path to the wasm-bindgen output files. Either the direct file or the optimized one depending on the build mode
    fn bundled_wasm_path(&self, assets: &AppManifest) -> String {
        let wasm_bindgen_wasm_out = self.wasm_bindgen_wasm_output_file();
        if self.should_bundle_to_asset() {
            let name = assets
                .get_first_asset_for_source(&wasm_bindgen_wasm_out)
                .expect("The wasm source must exist before creating index.html");
            format!("assets/{}", name.bundled_path())
        } else {
            format!(
                "wasm/{}",
                wasm_bindgen_wasm_out.file_name().unwrap().to_str().unwrap()
            )
        }
    }

    /// Users create an index.html for their SPA if they want it
    ///
    /// We always write our wasm as main.js and main_bg.wasm
    ///
    /// In prod we run the optimizer which bundles everything together properly
    ///
    /// So their index.html needs to include main.js in the scripts otherwise nothing happens?
    ///
    /// Seems like every platform has a weird file that declares a bunch of stuff
    /// - web: index.html
    /// - ios: info.plist
    /// - macos: info.plist
    /// - linux: appimage root thing?
    /// - android: androidmanifest.xml
    ///
    /// You also might different variants of these files (staging / prod) and different flavors (eu/us)
    ///
    /// web's index.html is weird since it's not just a bundle format but also a *content* format
    pub(crate) fn prepare_html(
        &self,
        assets: &AppManifest,
        wasm_path: &str,
        js_path: &str,
    ) -> Result<String> {
        let mut html = {
            const DEV_DEFAULT_HTML: &str = include_str!("../../assets/web/dev.index.html");
            const PROD_DEFAULT_HTML: &str = include_str!("../../assets/web/prod.index.html");

            let crate_root: &Path = &self.crate_dir();
            let custom_html_file = crate_root.join("index.html");
            let default_html = match self.release {
                true => PROD_DEFAULT_HTML,
                false => DEV_DEFAULT_HTML,
            };
            std::fs::read_to_string(custom_html_file).unwrap_or_else(|_| String::from(default_html))
        };

        // Inject any resources from the config into the html
        self.inject_resources(assets, &mut html)?;

        // Inject loading scripts if they are not already present
        self.inject_loading_scripts(assets, &mut html);

        // Replace any special placeholders in the HTML with resolved values
        self.replace_template_placeholders(&mut html, wasm_path, js_path);

        let title = self.config.web.app.title.clone();
        Self::replace_or_insert_before("{app_title}", "</title", &title, &mut html);

        Ok(html)
    }

    fn is_dev_build(&self) -> bool {
        !self.release
    }

    // Inject any resources from the config into the html
    fn inject_resources(&self, assets: &AppManifest, html: &mut String) -> Result<()> {
        use std::fmt::Write;

        // Collect all resources into a list of styles and scripts
        let resources = &self.config.web.resource;
        let mut style_list = resources.style.clone().unwrap_or_default();
        let mut script_list = resources.script.clone().unwrap_or_default();

        if self.is_dev_build() {
            style_list.extend(resources.dev.style.iter().cloned());
            script_list.extend(resources.dev.script.iter().cloned());
        }

        let mut head_resources = String::new();

        // Add all styles to the head
        for style in &style_list {
            writeln!(
                &mut head_resources,
                "<link rel=\"stylesheet\" href=\"{}\">",
                &style.to_str().unwrap(),
            )?;
        }

        // Add all scripts to the head
        for script in &script_list {
            writeln!(
                &mut head_resources,
                "<script src=\"{}\"></script>",
                &script.to_str().unwrap(),
            )?;
        }

        // Add the base path to the head if this is a debug build
        if self.is_dev_build() {
            if let Some(base_path) = &self.trimmed_base_path() {
                head_resources.push_str(&format_base_path_meta_element(base_path));
            }
        }

        // Inject any resources from manganis into the head
        for asset in assets.unique_assets() {
            let asset_path = asset.bundled_path();
            match asset.options().variant() {
                AssetVariant::Css(css_options) => {
                    if css_options.preloaded() {
                        _ = write!(
                            head_resources,
                            r#"<link rel="preload" as="style" href="/{{base_path}}/assets/{asset_path}" crossorigin>"#
                        );
                    }
                    if css_options.static_head() {
                        _ = write!(
                            head_resources,
                            r#"<link rel="stylesheet" href="/{{base_path}}/assets/{asset_path}" type="text/css">"#
                        );
                    }
                }
                AssetVariant::Image(image_options) if image_options.preloaded() => {
                    _ = write!(
                        head_resources,
                        r#"<link rel="preload" as="image" href="/{{base_path}}/assets/{asset_path}" crossorigin>"#
                    );
                }
                AssetVariant::Js(js_options) => {
                    if js_options.preloaded() {
                        _ = write!(
                            head_resources,
                            r#"<link rel="preload" as="script" href="/{{base_path}}/assets/{asset_path}" crossorigin>"#
                        );
                    }
                    if js_options.static_head() {
                        let source = std::path::Path::new(asset.absolute_source_path());
                        let module_attr = if js_is_module(js_options, source) {
                            r#" type="module""#
                        } else {
                            ""
                        };
                        _ = write!(
                            head_resources,
                            r#"<script{module_attr} src="/{{base_path}}/assets/{asset_path}"></script>"#
                        );
                    }
                }
                _ => {}
            }
        }

        // Do not preload the wasm file, because in Safari, preload as=fetch requires additional fetch() options to exactly match the network request
        // And if they do not match then Safari downloads the wasm file twice.
        // See https://github.com/wasm-bindgen/wasm-bindgen/blob/ac51055a4c39fa0affe02f7b63fb1d4c9b3ddfaf/crates/cli-support/src/js/mod.rs#L967
        Self::replace_or_insert_before("{style_include}", "</head", &head_resources, html);

        Ok(())
    }

    /// Inject loading scripts if they are not already present
    fn inject_loading_scripts(&self, assets: &AppManifest, html: &mut String) {
        // If the current build opted out of injecting loading scripts, don't inject anything
        if !self.inject_loading_scripts {
            return;
        }

        // If not, insert the script
        *html = html.replace(
            "</body",
            &format!(
                r#"<script type="module" async src="/{}/{}"></script>
            </body"#,
                self.base_path_or_default(),
                self.bundled_js_path(assets)
            ),
        );
    }

    /// Replace any special placeholders in the HTML with resolved values
    fn replace_template_placeholders(&self, html: &mut String, wasm_path: &str, js_path: &str) {
        let base_path = self.base_path_or_default();
        *html = html.replace("{base_path}", base_path);

        let app_name = &self.executable_name();

        // If the html contains the old `{app_name}` placeholder, replace {app_name}_bg.wasm and {app_name}.js
        // with the new paths
        *html = html.replace("wasm/{app_name}_bg.wasm", wasm_path);
        *html = html.replace("wasm/{app_name}.js", js_path);

        // Otherwise replace the new placeholders
        *html = html.replace("{wasm_path}", wasm_path);
        *html = html.replace("{js_path}", js_path);

        // Replace the app_name if we find it anywhere standalone
        *html = html.replace("{app_name}", app_name);
    }

    /// Replace a string or insert the new contents before a marker
    fn replace_or_insert_before(
        replace: &str,
        or_insert_before: &str,
        with: &str,
        content: &mut String,
    ) {
        if content.contains(replace) {
            *content = content.replace(replace, with);
        } else if let Some(pos) = content.find(or_insert_before) {
            content.insert_str(pos, with);
        }
    }

    /// Check if assets should be pre_compressed. This will only be true in release mode if the user
    /// has enabled pre_compress in the web config.
    pub fn should_pre_compress_web_assets(&self, release: bool) -> bool {
        self.config.web.pre_compress & release
    }

    /// Check if the wasm output should be bundled to an asset type app.
    pub(crate) fn should_bundle_to_asset(&self) -> bool {
        self.release && self.bundle == BundleFormat::Web
    }

    /// Get the path to the wasm bindgen temporary output folder
    pub fn wasm_bindgen_out_dir(&self) -> PathBuf {
        self.root_dir().join("wasm")
    }

    /// Get the path to the wasm bindgen javascript output file
    pub(crate) fn wasm_bindgen_js_output_file(&self) -> PathBuf {
        self.wasm_bindgen_out_dir()
            .join(self.executable_name())
            .with_extension("js")
    }

    /// Get the path to the wasm bindgen wasm output file
    pub(crate) fn wasm_bindgen_wasm_output_file(&self) -> PathBuf {
        self.wasm_bindgen_out_dir()
            .join(format!("{}_bg", self.executable_name()))
            .with_extension("wasm")
    }

    pub(crate) fn path_is_in_public_dir(&self, path: &Path) -> bool {
        let Some(static_dir) = self.user_public_dir() else {
            return false;
        };

        // Canonicalize when possible so we work with editors that use tmp files
        let canonical_static =
            dunce::canonicalize(&static_dir).unwrap_or_else(|_| static_dir.clone());
        let canonical_path = dunce::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());

        canonical_path.starts_with(&canonical_static)
    }
}
