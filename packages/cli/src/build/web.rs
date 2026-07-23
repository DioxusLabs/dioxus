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

enum WebResourceLocation<'a> {
    Browser(&'a str),
    Local(PathBuf),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WebResourceKind {
    Browser,
    Local,
}

fn web_resource_kind(resource: &Path) -> Result<WebResourceKind> {
    let resource_str = resource
        .to_str()
        .context("Web resource paths must be valid UTF-8")?;

    if resource_str.starts_with('/') {
        return Ok(WebResourceKind::Browser);
    }
    if resource.is_absolute() {
        return Ok(WebResourceKind::Local);
    }
    if url::Url::parse(resource_str).is_ok() {
        return Ok(WebResourceKind::Browser);
    }
    Ok(WebResourceKind::Local)
}

fn web_resource_location<'a>(
    crate_dir: &Path,
    resource: &'a Path,
) -> Result<WebResourceLocation<'a>> {
    let resource_str = resource
        .to_str()
        .context("Web resource paths must be valid UTF-8")?;

    // Preserve the original `[web.resource]` contract for root-relative and
    // protocol-relative browser URLs. On Windows, backslash UNC paths remain
    // local while `//server/share` retains its browser URL meaning.
    match web_resource_kind(resource)? {
        WebResourceKind::Browser => Ok(WebResourceLocation::Browser(resource_str)),
        WebResourceKind::Local => {
            resolve_web_resource(crate_dir, resource).map(WebResourceLocation::Local)
        }
    }
}

fn resolve_web_resource(crate_dir: &Path, resource: &Path) -> Result<PathBuf> {
    let source = if resource.is_absolute() {
        resource.to_path_buf()
    } else {
        crate_dir.join(resource)
    };

    let source = dunce::canonicalize(&source).with_context(|| {
        format!(
            "Failed to resolve web resource `{}` from `{}`",
            resource.display(),
            crate_dir.display()
        )
    })?;

    if !source.is_file() {
        anyhow::bail!(
            "Web resource `{}` does not resolve to a file",
            resource.display()
        );
    }

    Ok(source)
}

fn register_web_resource(
    assets: &mut AppManifest,
    crate_dir: &Path,
    resource: &Path,
    options: AssetOptions,
) -> Result<()> {
    if let WebResourceLocation::Local(source) = web_resource_location(crate_dir, resource)? {
        assets
            .register_asset(&source, options)
            .with_context(|| format!("Failed to register web resource `{}`", resource.display()))?;
    }
    Ok(())
}

fn append_url_path_segments<'a>(
    segments: &mut url::PathSegmentsMut<'a>,
    path: &Path,
) -> Result<()> {
    for component in path.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::Normal(segment) => {
                segments.push(
                    segment
                        .to_str()
                        .context("Bundled web resource paths must be valid UTF-8")?,
                );
            }
            _ => anyhow::bail!(
                "Bundled web resource path must be relative: `{}`",
                path.display()
            ),
        };
    }
    Ok(())
}

fn bundled_web_resource_url(base_path: Option<&str>, bundled_path: &Path) -> Result<String> {
    let mut url = url::Url::parse("https://dioxus.invalid/")
        .expect("the internal web resource URL base must be valid");
    {
        let mut segments = url
            .path_segments_mut()
            .expect("the internal web resource URL base supports path segments");
        segments.clear();
        if let Some(base_path) = base_path {
            append_url_path_segments(&mut segments, Path::new(base_path))?;
        }
        segments.push("assets");
        append_url_path_segments(&mut segments, bundled_path)?;
    }

    let path = url.path();
    Ok(if base_path.is_some() {
        path.to_owned()
    } else {
        path.trim_start_matches('/').to_owned()
    })
}

fn web_resource_url(
    is_dev: bool,
    crate_dir: &Path,
    base_path: Option<&str>,
    assets: &AppManifest,
    resource: &Path,
    options: AssetOptions,
) -> Result<String> {
    if is_dev {
        return resource
            .to_str()
            .map(ToOwned::to_owned)
            .context("Web resource paths must be valid UTF-8");
    }

    let source = match web_resource_location(crate_dir, resource)? {
        WebResourceLocation::Browser(url) => return Ok(url.to_owned()),
        WebResourceLocation::Local(source) => source,
    };
    let asset = assets
        .get_assets_for_source(&source)
        .and_then(|assets| assets.iter().find(|asset| asset.options() == &options))
        .with_context(|| {
            format!(
                "Web resource `{}` was not mapped to a bundled asset",
                resource.display()
            )
        })?;

    bundled_web_resource_url(base_path, Path::new(asset.bundled_path()))
}

fn register_configured_web_resources(
    assets: &mut AppManifest,
    crate_dir: &Path,
    resources: &crate::config::WebResourceConfig,
) -> Result<()> {
    for style in resources.style.iter().flatten() {
        register_web_resource(
            assets,
            crate_dir,
            style,
            AssetOptions::css().into_asset_options(),
        )?;
    }
    for script in resources.script.iter().flatten() {
        register_web_resource(
            assets,
            crate_dir,
            script,
            AssetOptions::js().into_asset_options(),
        )?;
    }
    Ok(())
}

fn configured_web_resource_tags(
    is_dev: bool,
    crate_dir: &Path,
    base_path: Option<&str>,
    assets: &AppManifest,
    resources: &crate::config::WebResourceConfig,
) -> Result<String> {
    use std::fmt::Write;

    let mut style_list = resources.style.clone().unwrap_or_default();
    let mut script_list = resources.script.clone().unwrap_or_default();
    if is_dev {
        style_list.extend(resources.dev.style.iter().cloned());
        script_list.extend(resources.dev.script.iter().cloned());
    }

    let mut tags = String::new();
    for style in &style_list {
        let url = web_resource_url(
            is_dev,
            crate_dir,
            base_path,
            assets,
            style,
            AssetOptions::css().into_asset_options(),
        )?;
        writeln!(&mut tags, "<link rel=\"stylesheet\" href=\"{url}\">")?;
    }
    for script in &script_list {
        let url = web_resource_url(
            is_dev,
            crate_dir,
            base_path,
            assets,
            script,
            AssetOptions::js().into_asset_options(),
        )?;
        writeln!(&mut tags, "<script src=\"{url}\"></script>")?;
    }
    Ok(tags)
}

fn write_index_html_file(path: &Path, prepare: impl FnOnce() -> Result<String>) -> Result<()> {
    let html = prepare().context("Failed to prepare web index.html")?;
    std::fs::write(path, html).context("Failed to write web index.html")
}

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

        self.register_web_resources(assets)?;

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
        write_index_html_file(&self.root_dir().join("index.html"), || {
            self.prepare_html(assets, &wasm_path, &js_path)
        })
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

    fn register_web_resources(&self, assets: &mut AppManifest) -> Result<()> {
        if !self.should_bundle_to_asset() {
            return Ok(());
        }

        let resources = &self.config.web.resource;
        register_configured_web_resources(assets, &self.crate_dir(), resources)
    }

    // Inject any resources from the config into the html
    fn inject_resources(&self, assets: &AppManifest, html: &mut String) -> Result<()> {
        use std::fmt::Write;

        let resources = &self.config.web.resource;
        let mut head_resources = configured_web_resource_tags(
            self.is_dev_build(),
            &self.crate_dir(),
            self.trimmed_base_path(),
            assets,
            resources,
        )?;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn release_resources_use_hashed_manifest_paths() {
        let temp = tempfile::tempdir().unwrap();
        let css = temp.path().join("assets/styling/main.css");
        let js = temp.path().join("assets/scripts/main.js");
        std::fs::create_dir_all(css.parent().unwrap()).unwrap();
        std::fs::create_dir_all(js.parent().unwrap()).unwrap();
        std::fs::write(&css, "body { color: red; }").unwrap();
        std::fs::write(&js, "console.log('release');").unwrap();

        let mut manifest = AppManifest::new();
        register_web_resource(
            &mut manifest,
            temp.path(),
            Path::new("assets/styling/main.css"),
            AssetOptions::css().into_asset_options(),
        )
        .unwrap();
        register_web_resource(
            &mut manifest,
            temp.path(),
            Path::new("assets/scripts/main.js"),
            AssetOptions::js().into_asset_options(),
        )
        .unwrap();

        let css_url = web_resource_url(
            false,
            temp.path(),
            None,
            &manifest,
            Path::new("assets/styling/main.css"),
            AssetOptions::css().into_asset_options(),
        )
        .unwrap();
        let js_url = web_resource_url(
            false,
            temp.path(),
            None,
            &manifest,
            Path::new("assets/scripts/main.js"),
            AssetOptions::js().into_asset_options(),
        )
        .unwrap();

        assert!(css_url.starts_with("assets/main-dxh"));
        assert!(css_url.ends_with(".css"));
        assert!(js_url.starts_with("assets/main-dxh"));
        assert!(js_url.ends_with(".js"));
        assert_ne!(css_url, "assets/styling/main.css");
        assert_ne!(js_url, "assets/scripts/main.js");

        let public = temp.path().join("public");
        for asset in manifest.unique_assets() {
            crate::opt::process_file_to(
                asset.options(),
                Path::new(asset.absolute_source_path()),
                &public.join("assets").join(asset.bundled_path()),
                None,
            )
            .unwrap();
        }
        assert!(public.join(css_url).is_file());
        assert!(public.join(js_url).is_file());
    }

    #[test]
    fn dev_resources_keep_configured_source_paths() {
        let manifest = AppManifest::new();
        let resource = Path::new("assets/styling/main.css");

        let url = web_resource_url(
            true,
            Path::new("/project"),
            Some("docs"),
            &manifest,
            resource,
            AssetOptions::css().into_asset_options(),
        )
        .unwrap();

        assert_eq!(url, "assets/styling/main.css");
    }

    #[test]
    fn release_resource_url_applies_base_path_once() {
        let temp = tempfile::tempdir().unwrap();
        let css = temp.path().join("main.css");
        std::fs::write(&css, "body { color: blue; }").unwrap();
        let mut manifest = AppManifest::new();
        register_web_resource(
            &mut manifest,
            temp.path(),
            Path::new("main.css"),
            AssetOptions::css().into_asset_options(),
        )
        .unwrap();

        let url = web_resource_url(
            false,
            temp.path(),
            Some("docs"),
            &manifest,
            Path::new("main.css"),
            AssetOptions::css().into_asset_options(),
        )
        .unwrap();

        assert!(url.starts_with("/docs/assets/main-dxh"));
        assert_eq!(url.matches("/docs").count(), 1);
    }

    #[test]
    fn missing_or_unregistered_release_resources_fail() {
        let temp = tempfile::tempdir().unwrap();
        let mut manifest = AppManifest::new();
        let missing = Path::new("missing.css");

        let missing_error = register_web_resource(
            &mut manifest,
            temp.path(),
            missing,
            AssetOptions::css().into_asset_options(),
        )
        .unwrap_err()
        .to_string();
        assert!(missing_error.contains("missing.css"));

        let existing = temp.path().join("existing.css");
        std::fs::write(&existing, "body {}").unwrap();
        let unmapped_error = web_resource_url(
            false,
            temp.path(),
            None,
            &manifest,
            Path::new("existing.css"),
            AssetOptions::css().into_asset_options(),
        )
        .unwrap_err()
        .to_string();
        assert!(unmapped_error.contains("existing.css"));
    }

    #[test]
    fn release_external_resources_remain_external() {
        let manifest = AppManifest::new();
        let resource = Path::new("https://cdn.example.com/main.css");

        let url = web_resource_url(
            false,
            Path::new("/project"),
            Some("docs"),
            &manifest,
            resource,
            AssetOptions::css().into_asset_options(),
        )
        .unwrap();

        assert_eq!(url, "https://cdn.example.com/main.css");
    }

    #[test]
    fn release_hashed_urls_percent_encode_each_path_segment() {
        let temp = tempfile::tempdir().unwrap();
        let css = temp.path().join("theme # %20 你好.css");
        std::fs::write(&css, "body { color: green; }").unwrap();
        let mut manifest = AppManifest::new();
        register_web_resource(
            &mut manifest,
            temp.path(),
            Path::new("theme # %20 你好.css"),
            AssetOptions::css().into_asset_options(),
        )
        .unwrap();

        let url = web_resource_url(
            false,
            temp.path(),
            Some("docs space/版本"),
            &manifest,
            Path::new("theme # %20 你好.css"),
            AssetOptions::css().into_asset_options(),
        )
        .unwrap();

        assert!(url.starts_with("/docs%20space/%E7%89%88%E6%9C%AC/assets/"));
        assert!(url.contains("theme%20%23%20%2520%20%E4%BD%A0%E5%A5%BD-dxh"));
        assert!(!url.contains("%2F"));
        assert!(!url.contains('你'));
        assert!(!url.contains('#'));
    }

    #[test]
    fn browser_urls_keep_the_legacy_verbatim_contract() {
        let manifest = AppManifest::new();
        for resource in [
            "/styles.css",
            "//cdn.example.com/styles.css",
            "data:text/css,body%7Bcolor:red%7D",
            "blob:https://example.com/3f4e",
            "https://cdn.example.com/styles.css",
        ] {
            assert_eq!(
                web_resource_kind(Path::new(resource)).unwrap(),
                WebResourceKind::Browser
            );
            assert_eq!(
                web_resource_url(
                    false,
                    Path::new("/project"),
                    Some("docs"),
                    &manifest,
                    Path::new(resource),
                    AssetOptions::css().into_asset_options(),
                )
                .unwrap(),
                resource
            );
        }
    }

    #[test]
    fn slash_unc_spelling_keeps_protocol_relative_browser_semantics() {
        assert_eq!(
            web_resource_kind(Path::new("//server/share/styles.css")).unwrap(),
            WebResourceKind::Browser
        );
    }

    #[cfg(windows)]
    #[test]
    fn backslash_unc_spelling_is_a_local_windows_path() {
        assert_eq!(
            web_resource_kind(Path::new(r"\\server\share\styles.css")).unwrap(),
            WebResourceKind::Local
        );
    }

    #[test]
    fn parsed_config_produces_encoded_html_and_matching_disk_assets() {
        let temp = tempfile::tempdir().unwrap();
        let css_name = "assets/theme # %20 你好.css";
        let js_name = "assets/main # %2F app.js";
        std::fs::create_dir_all(temp.path().join("assets")).unwrap();
        std::fs::write(temp.path().join(css_name), "body { color: navy; }").unwrap();
        std::fs::write(temp.path().join(js_name), "console.log('encoded');").unwrap();

        let config: crate::config::DioxusConfig = toml::from_str(&format!(
            r#"
                [web.resource]
                style = ["{css_name}"]
                script = ["{js_name}"]

                [web.resource.dev]
            "#
        ))
        .unwrap();
        let mut manifest = AppManifest::new();
        register_configured_web_resources(&mut manifest, temp.path(), &config.web.resource)
            .unwrap();
        let tags = configured_web_resource_tags(
            false,
            temp.path(),
            Some("docs"),
            &manifest,
            &config.web.resource,
        )
        .unwrap();
        let html = format!("<html><head>{tags}</head><body></body></html>");
        let public_assets = temp.path().join("public/assets");

        for asset in manifest.unique_assets() {
            let output = public_assets.join(asset.bundled_path());
            crate::opt::process_file_to(
                asset.options(),
                Path::new(asset.absolute_source_path()),
                &output,
                None,
            )
            .unwrap();
            assert!(output.is_file());

            let url =
                bundled_web_resource_url(Some("docs"), Path::new(asset.bundled_path())).unwrap();
            assert!(html.contains(&url));
        }

        assert!(html.contains("%23"));
        assert!(html.contains("%2520"));
        assert!(html.contains("%252F"));
        assert!(html.contains("%E4%BD%A0%E5%A5%BD"));
        assert!(!html.contains(css_name));
        assert!(!html.contains(js_name));
    }

    #[test]
    fn index_resource_errors_propagate_before_writing_html() {
        let temp = tempfile::tempdir().unwrap();
        let resource = temp.path().join("existing.css");
        std::fs::write(&resource, "body {}").unwrap();
        let config: crate::config::DioxusConfig = toml::from_str(
            r#"
                [web.resource]
                style = ["existing.css"]

                [web.resource.dev]
            "#,
        )
        .unwrap();

        let error = configured_web_resource_tags(
            false,
            temp.path(),
            None,
            &AppManifest::new(),
            &config.web.resource,
        )
        .unwrap_err();

        assert!(
            error
                .to_string()
                .contains("was not mapped to a bundled asset")
        );
    }

    #[test]
    fn write_index_html_file_returns_prepare_errors_without_writing() {
        let temp = tempfile::tempdir().unwrap();
        let index = temp.path().join("index.html");
        let error = write_index_html_file(&index, || {
            anyhow::bail!("configured resource was not mapped")
        })
        .unwrap_err();
        let error = format!("{error:#}");
        assert!(error.contains("Failed to prepare web index.html"));
        assert!(error.contains("configured resource was not mapped"));
        assert!(!index.exists());
    }
}
