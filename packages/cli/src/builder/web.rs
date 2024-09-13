use super::{BuildRequest, Platform};
use crate::error::{Error, Result};
use crate::{
    builder::progress::{
        BuildMessage, BuildUpdateProgress, MessageSource, MessageType, Stage, UpdateStage,
    },
    TraceSrc,
};
use anyhow::Context;
use std::fmt::Write;
use std::path::{Path, PathBuf};
use tokio::process::Command;
use tracing::Level;
use wasm_bindgen_cli_support::Bindgen;

const DEFAULT_HTML: &str = include_str!("../../assets/index.html");
const TOAST_HTML: &str = include_str!("../../assets/toast.html");

impl BuildRequest {
    pub(crate) async fn run_wasm_bindgen(
        &self,
        input_path: &Path,
        bindgen_outdir: &Path,
    ) -> Result<()> {
        tracing::info!("Running wasm-bindgen");

        let input_path = input_path.to_path_buf();
        let bindgen_outdir = bindgen_outdir.to_path_buf();
        let name = self.krate.dioxus_config.application.name.clone();
        let keep_debug = self.krate.dioxus_config.web.wasm_opt.debug || (!self.build.release);

        let start = std::time::Instant::now();
        tokio::task::spawn_blocking(move || {
            Bindgen::new()
                .input_path(&input_path)
                .web(true)
                .unwrap()
                .debug(keep_debug)
                .demangle(keep_debug)
                .keep_debug(keep_debug)
                .reference_types(true)
                .remove_name_section(!keep_debug)
                .remove_producers_section(!keep_debug)
                .out_name(&name)
                .generate(&bindgen_outdir)
        })
        .await
        .context("Wasm-bindgen crashed while optimizing the wasm binary")?
        .context("Failed to generate wasm-bindgen bindings")?;

        tracing::info!(dx_src = ?TraceSrc::Build, "wasm-bindgen complete in {:?}", start.elapsed());

        Ok(())
    }

    #[allow(unused)]
    pub(crate) fn run_wasm_opt(&self, bindgen_outdir: &std::path::PathBuf) -> Result<(), Error> {
        if !self.build.release {
            return Ok(());
        };

        #[cfg(feature = "wasm-opt")]
        {
            use crate::config::WasmOptLevel;

            tracing::info!(dx_src = ?TraceSrc::Build, "Running optimization with wasm-opt...");

            let mut options = match self.dioxus_crate.dioxus_config.web.wasm_opt.level {
                WasmOptLevel::Z => {
                    wasm_opt::OptimizationOptions::new_optimize_for_size_aggressively()
                }
                WasmOptLevel::S => wasm_opt::OptimizationOptions::new_optimize_for_size(),
                WasmOptLevel::Zero => wasm_opt::OptimizationOptions::new_opt_level_0(),
                WasmOptLevel::One => wasm_opt::OptimizationOptions::new_opt_level_1(),
                WasmOptLevel::Two => wasm_opt::OptimizationOptions::new_opt_level_2(),
                WasmOptLevel::Three => wasm_opt::OptimizationOptions::new_opt_level_3(),
                WasmOptLevel::Four => wasm_opt::OptimizationOptions::new_opt_level_4(),
            };
            let wasm_file = bindgen_outdir.join(format!(
                "{}_bg.wasm",
                self.dioxus_crate.dioxus_config.application.name
            ));
            let old_size = wasm_file.metadata()?.len();
            options
                // WASM bindgen relies on reference types
                .enable_feature(wasm_opt::Feature::ReferenceTypes)
                .debug_info(self.dioxus_crate.dioxus_config.web.wasm_opt.debug)
                .run(&wasm_file, &wasm_file)
                .map_err(|err| Error::Other(anyhow::anyhow!(err)))?;

            let new_size = wasm_file.metadata()?.len();
            tracing::info!(
                dx_src = ?TraceSrc::Build,
                "wasm-opt reduced WASM size from {} to {} ({:2}%)",
                old_size,
                new_size,
                (new_size as f64 - old_size as f64) / old_size as f64 * 100.0
            );
        }

        Ok(())
    }

    pub(crate) fn prepare_html(&self) -> Result<String> {
        let mut html = {
            let crate_root: &Path = &self.krate.crate_dir();
            let custom_html_file = crate_root.join("index.html");
            std::fs::read_to_string(custom_html_file).unwrap_or_else(|_| String::from(DEFAULT_HTML))
        };

        // Inject any resources from the config into the html
        self.inject_resources(&mut html)?;

        // Inject loading scripts if they are not already present
        self.inject_loading_scripts(&mut html);

        // Replace any special placeholders in the HTML with resolved values
        self.replace_template_placeholders(&mut html);

        let title = self.krate.dioxus_config.web.app.title.clone();

        replace_or_insert_before("{app_title}", "</title", &title, &mut html);

        Ok(html)
    }

    fn is_dev_build(&self) -> bool {
        !self.build.release
    }

    // Inject any resources from the config into the html
    fn inject_resources(&self, html: &mut String) -> Result<()> {
        // Collect all resources into a list of styles and scripts
        let resources = &self.krate.dioxus_config.web.resource;
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

        if !style_list.is_empty() {
            self.send_resource_deprecation_warning(style_list, ResourceType::Style);
        }
        if !script_list.is_empty() {
            self.send_resource_deprecation_warning(script_list, ResourceType::Script);
        }

        // Inject any resources from manganis into the head
        // if let Some(assets) = assets {
        //     head_resources.push_str(&assets.head());
        // }

        replace_or_insert_before("{style_include}", "</head", &head_resources, html);

        Ok(())
    }

    /// Inject loading scripts if they are not already present
    fn inject_loading_scripts(&self, html: &mut String) {
        // If it looks like we are already loading wasm or the current build opted out of injecting loading scripts, don't inject anything
        if !self.build.inject_loading_scripts || html.contains("__wbindgen_start") {
            return;
        }

        // If not, insert the script
        *html = html.replace(
            "</body",
            r#"<script>
            // We can't use a module script here because we need to start the script immediately when streaming
            import("/{base_path}/wasm/{app_name}.js").then(
                ({ default: init }) => {
                init("/{base_path}/wasm/{app_name}_bg.wasm").then((wasm) => {
                    if (wasm.__wbindgen_start == undefined) {
                    wasm.main();
                    }
                });
                }
            );
            </script>
            {DX_TOAST_UTILITIES}
            </body"#,
        );

        // Trim out the toasts if we're in release, or add them if we're serving
        *html = match self.is_dev_build() {
            true => html.replace("{DX_TOAST_UTILITIES}", TOAST_HTML),
            false => html.replace("{DX_TOAST_UTILITIES}", ""),
        };

        // And try to insert preload links for the wasm and js files
        *html = html.replace(
            "</head",
            r#"<link rel="preload" href="/{base_path}/wasm/{app_name}_bg.wasm" as="fetch" type="application/wasm" crossorigin="">
            <link rel="preload" href="/{base_path}/wasm/{app_name}.js" as="script">
            </head"#
        );
    }

    /// Replace any special placeholders in the HTML with resolved values
    fn replace_template_placeholders(&self, html: &mut String) {
        let base_path = self.krate.dioxus_config.web.app.base_path();
        *html = html.replace("{base_path}", base_path);

        let app_name = &self.krate.dioxus_config.application.name;
        *html = html.replace("{app_name}", app_name);
    }

    fn send_resource_deprecation_warning(&self, paths: Vec<PathBuf>, variant: ResourceType) {
        const RESOURCE_DEPRECATION_MESSAGE: &str = r#"The `web.resource` config has been deprecated in favor of head components and will be removed in a future release. Instead of including assets in the config, you can include assets with the `asset!` macro and add them to the head with `document::Link` and `Script` components."#;

        let replacement_components = paths
            .iter()
            .map(|path| {
                let path = if path.exists() {
                    path.to_path_buf()
                } else {
                    // If the path is absolute, make it relative to the current directory before we join it
                    // The path is actually a web path which is relative to the root of the website
                    let path = path.strip_prefix("/").unwrap_or(path);
                    let asset_dir_path = self.krate.legacy_asset_dir().join(path);
                    if let Ok(absolute_path) = asset_dir_path.canonicalize() {
                        let absolute_crate_root = self.krate.crate_dir().canonicalize().unwrap();
                        PathBuf::from("./")
                            .join(absolute_path.strip_prefix(absolute_crate_root).unwrap())
                    } else {
                        path.to_path_buf()
                    }
                };
                match variant {
                    ResourceType::Style => {
                        format!("    Stylesheet {{ href: asset!(\"{}\") }}", path.display())
                    }
                    ResourceType::Script => {
                        format!("    Script {{ src: asset!(\"{}\") }}", path.display())
                    }
                }
            })
            .collect::<Vec<_>>();
        let replacement_components = format!("rsx! {{\n{}\n}}", replacement_components.join("\n"));
        let section_name = match variant {
            ResourceType::Style => "web.resource.style",
            ResourceType::Script => "web.resource.script",
        };

        let message = format!(
            "{RESOURCE_DEPRECATION_MESSAGE}\nTo migrate to head components, remove `{section_name}` and include the following rsx in your root component:\n```rust\n{replacement_components}\n```"
        );

        // _ = self.progress.unbounded_send(BuildUpdateProgress {
        //     platform: self.platform(),
        //     stage: Stage::OptimizingWasm,
        //     update: UpdateStage::AddMessage(BuildMessage {
        //         level: Level::WARN,
        //         message: MessageType::Text(message),
        //         source: MessageSource::Build,
        //     }),
        // });
    }

    /// Check if the build is targeting the web platform
    pub(crate) fn targeting_web(&self) -> bool {
        self.platform() == Platform::Web
    }
}

enum ResourceType {
    Style,
    Script,
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
