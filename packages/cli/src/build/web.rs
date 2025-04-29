use crate::error::Result;
use dioxus_cli_config::format_base_path_meta_element;
use dioxus_cli_opt::AssetManifest;
use manganis::AssetOptions;
use std::fmt::Write;
use std::path::Path;

use super::BuildRequest;

const DEFAULT_HTML: &str = include_str!("../../assets/web/dev.index.html");

impl BuildRequest {
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
        assets: &AssetManifest,
        wasm_path: &str,
        js_path: &str,
    ) -> Result<String> {
        let mut html = {
            let crate_root: &Path = &self.crate_dir();
            let custom_html_file = crate_root.join("index.html");
            std::fs::read_to_string(custom_html_file).unwrap_or_else(|_| String::from(DEFAULT_HTML))
        };

        // Inject any resources from the config into the html
        self.inject_resources(assets, &mut html)?;

        // Inject loading scripts if they are not already present
        self.inject_loading_scripts(&mut html);

        // Replace any special placeholders in the HTML with resolved values
        self.replace_template_placeholders(&mut html, wasm_path, js_path);

        let title = self.config.web.app.title.clone();
        replace_or_insert_before("{app_title}", "</title", &title, &mut html);

        Ok(html)
    }

    fn is_dev_build(&self) -> bool {
        !self.release
    }

    // Inject any resources from the config into the html
    fn inject_resources(&self, assets: &AssetManifest, html: &mut String) -> Result<()> {
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
            if let Some(base_path) = &self.config.web.app.base_path {
                head_resources.push_str(&format_base_path_meta_element(base_path));
            }
        }

        // Inject any resources from manganis into the head
        for asset in assets.assets.values() {
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
        let wasm_source_path = self.wasm_bindgen_wasm_output_file();
        if let Some(wasm_path) = assets.assets.get(&wasm_source_path) {
            let wasm_path = wasm_path.bundled_path();
            head_resources.push_str(&format!(
                    "<link rel=\"preload\" as=\"fetch\" type=\"application/wasm\" href=\"/{{base_path}}/assets/{wasm_path}\" crossorigin>"
                ));

            replace_or_insert_before("{style_include}", "</head", &head_resources, html);
        }

        Ok(())
    }

    /// Inject loading scripts if they are not already present
    fn inject_loading_scripts(&self, html: &mut String) {
        // If it looks like we are already loading wasm or the current build opted out of injecting loading scripts, don't inject anything
        if !self.inject_loading_scripts || html.contains("__wbindgen_start") {
            return;
        }

        // If not, insert the script
        *html = html.replace(
            "</body",
r#" <script>
  // We can't use a module script here because we need to start the script immediately when streaming
  import("/{base_path}/{js_path}").then(
    ({ default: init, initSync, __wbg_get_imports }) => {
      // export initSync in case a split module needs to initialize
      window.__wasm_split_main_initSync = initSync;

      // Actually perform the load
      init("/{base_path}/{wasm_path}").then((wasm) => {
        if (wasm.__wbindgen_start == undefined) {
            wasm.main();
        }

        // assign this module to be accessible globally
        window.mainWasm = wasm;
        window.mainInit = init;
        window.mainInitSync = initSync;
        window.__wbg_get_imports = __wbg_get_imports;
      });
    }
  );
  </script>
            </body"#,
        );
    }

    /// Replace any special placeholders in the HTML with resolved values
    fn replace_template_placeholders(&self, html: &mut String, wasm_path: &str, js_path: &str) {
        let base_path = self.config.web.app.base_path();
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
