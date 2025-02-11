use dioxus_cli_config::format_base_path_meta_element;
use manganis::AssetOptions;

use crate::error::Result;
use std::fmt::Write;
use std::path::{Path, PathBuf};

use super::AppBundle;

const DEFAULT_HTML: &str = include_str!("../../assets/web/index.html");
const TOAST_HTML: &str = include_str!("../../assets/web/toast.html");

impl AppBundle {
    pub(crate) fn prepare_html(&self) -> Result<String> {
        let mut html = {
            let crate_root: &Path = &self.build.krate.crate_dir();
            let custom_html_file = crate_root.join("index.html");
            std::fs::read_to_string(custom_html_file).unwrap_or_else(|_| String::from(DEFAULT_HTML))
        };

        // Inject any resources from the config into the html
        self.inject_resources(&mut html)?;

        // Inject loading scripts if they are not already present
        self.inject_loading_scripts(&mut html);

        // Replace any special placeholders in the HTML with resolved values
        self.replace_template_placeholders(&mut html);

        let title = self.build.krate.config.web.app.title.clone();

        replace_or_insert_before("{app_title}", "</title", &title, &mut html);

        Ok(html)
    }

    fn is_dev_build(&self) -> bool {
        !self.build.build.release
    }

    // Inject any resources from the config into the html
    fn inject_resources(&self, html: &mut String) -> Result<()> {
        // Collect all resources into a list of styles and scripts
        let resources = &self.build.krate.config.web.resource;
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
            if let Some(base_path) = &self.build.krate.config.web.app.base_path {
                head_resources.push_str(&format_base_path_meta_element(base_path));
            }
        }

        if !style_list.is_empty() {
            self.send_resource_deprecation_warning(style_list, ResourceType::Style);
        }
        if !script_list.is_empty() {
            self.send_resource_deprecation_warning(script_list, ResourceType::Script);
        }

        // Inject any resources from manganis into the head
        for asset in self.app.assets.assets.values() {
            let asset_path = asset.bundled_path();
            match asset.options() {
                AssetOptions::Css(css_options) => {
                    if css_options.preloaded() {
                        head_resources.push_str(&format!(
                            "<link rel=\"preload\" as=\"style\" href=\"/{{base_path}}/assets/{asset_path}\" crossorigin>"
                        ))
                    }
                }
                AssetOptions::Image(image_options) => {
                    if image_options.preloaded() {
                        head_resources.push_str(&format!(
                            "<link rel=\"preload\" as=\"image\" href=\"/{{base_path}}/assets/{asset_path}\" crossorigin>"
                        ))
                    }
                }
                AssetOptions::Js(js_options) => {
                    if js_options.preloaded() {
                        head_resources.push_str(&format!(
                            "<link rel=\"preload\" as=\"script\" href=\"/{{base_path}}/assets/{asset_path}\" crossorigin>"
                        ))
                    }
                }
                _ => {}
            }
        }
        // Manually inject the wasm file for preloading. WASM currently doesn't support preloading in the manganis asset system
        let wasm_source_path = self.build.wasm_bindgen_wasm_output_file();
        let wasm_path = self
            .app
            .assets
            .assets
            .get(&wasm_source_path)
            .expect("WASM asset should exist in web bundles")
            .bundled_path();
        head_resources.push_str(&format!(
            "<link rel=\"preload\" as=\"fetch\" type=\"application/wasm\" href=\"/{{base_path}}/assets/{wasm_path}\" crossorigin>"
        ));

        replace_or_insert_before("{style_include}", "</head", &head_resources, html);

        Ok(())
    }

    /// Inject loading scripts if they are not already present
    fn inject_loading_scripts(&self, html: &mut String) {
        // If it looks like we are already loading wasm or the current build opted out of injecting loading scripts, don't inject anything
        if !self.build.build.inject_loading_scripts || html.contains("__wbindgen_start") {
            return;
        }

        // If not, insert the script
        *html = html.replace(
            "</body",
            r#"<script>
            // We can't use a module script here because we need to start the script immediately when streaming
            import("/{base_path}/{js_path}").then(
                ({ default: init }) => {
                init("/{base_path}/{wasm_path}").then((wasm) => {
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
    }

    /// Replace any special placeholders in the HTML with resolved values
    fn replace_template_placeholders(&self, html: &mut String) {
        let base_path = self.build.krate.config.web.app.base_path();
        *html = html.replace("{base_path}", base_path);

        let app_name = &self.build.krate.executable_name();
        let wasm_source_path = self.build.wasm_bindgen_wasm_output_file();
        let wasm_path = self
            .app
            .assets
            .assets
            .get(&wasm_source_path)
            .expect("WASM asset should exist in web bundles")
            .bundled_path();
        let wasm_path = format!("assets/{wasm_path}");
        let js_source_path = self.build.wasm_bindgen_js_output_file();
        let js_path = self
            .app
            .assets
            .assets
            .get(&js_source_path)
            .expect("JS asset should exist in web bundles")
            .bundled_path();
        let js_path = format!("assets/{js_path}");

        // If the html contains the old `{app_name}` placeholder, replace {app_name}_bg.wasm and {app_name}.js
        // with the new paths
        *html = html.replace("wasm/{app_name}_bg.wasm", &wasm_path);
        *html = html.replace("wasm/{app_name}.js", &js_path);
        // Otherwise replace the new placeholders
        *html = html.replace("{wasm_path}", &wasm_path);
        *html = html.replace("{js_path}", &js_path);
        // Replace the app_name if we find it anywhere standalone
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
                    let asset_dir_path = self
                        .build
                        .krate
                        .legacy_asset_dir()
                        .map(|dir| dir.join(path).canonicalize());

                    if let Some(Ok(absolute_path)) = asset_dir_path {
                        let absolute_crate_root =
                            self.build.krate.crate_dir().canonicalize().unwrap();
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

        tracing::warn!(
            "{RESOURCE_DEPRECATION_MESSAGE}\nTo migrate to head components, remove `{section_name}` and include the following rsx in your root component:\n```rust\n{replacement_components}\n```"
        );
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
