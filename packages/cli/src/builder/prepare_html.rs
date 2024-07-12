//! Build the HTML file to load a web application. The index.html file may be created from scratch or modified from the `index.html` file in the crate root.

use super::BuildRequest;
use crate::Result;
use manganis_cli_support::AssetManifest;
use std::fmt::Write;
use std::path::Path;

const DEFAULT_HTML: &str = include_str!("../../assets/index.html");

impl BuildRequest {
    pub(crate) fn prepare_html(&self, assets: Option<&AssetManifest>) -> Result<String> {
        let mut html = html_or_default(&self.dioxus_crate.crate_dir());

        // Inject any resources from the config into the html
        self.inject_resources(&mut html, assets)?;

        // Inject loading scripts if they are not already present
        self.inject_loading_scripts(&mut html);

        // Replace any special placeholders in the HTML with resolved values
        self.replace_template_placeholders(&mut html);

        let title = self.dioxus_crate.dioxus_config.web.app.title.clone();

        replace_or_insert_before("{app_title}", "</title", &title, &mut html);

        Ok(html)
    }

    // Inject any resources from the config into the html
    fn inject_resources(&self, html: &mut String, assets: Option<&AssetManifest>) -> Result<()> {
        // Collect all resources into a list of styles and scripts
        let resources = &self.dioxus_crate.dioxus_config.web.resource;
        let mut style_list = resources.style.clone().unwrap_or_default();
        let mut script_list = resources.script.clone().unwrap_or_default();

        if self.serve {
            style_list.extend(resources.dev.style.iter().cloned());
            script_list.extend(resources.dev.script.iter().cloned());
        }

        let mut head_resources = String::new();
        // Add all styles to the head
        for style in style_list {
            writeln!(
                &mut head_resources,
                "<link rel=\"stylesheet\" href=\"{}\">",
                &style.to_str().unwrap(),
            )?;
        }

        // Add all scripts to the head
        for script in script_list {
            writeln!(
                &mut head_resources,
                "<script src=\"{}\"></script>",
                &script.to_str().unwrap(),
            )?;
        }

        // Inject any resources from manganis into the head
        if let Some(assets) = assets {
            head_resources.push_str(&assets.head());
        }

        replace_or_insert_before("{style_include}", "</head", &head_resources, html);

        Ok(())
    }

    /// Inject loading scripts if they are not already present
    fn inject_loading_scripts(&self, html: &mut String) {
        // If it looks like we are already loading wasm or the current build opted out of injecting loading scripts, don't inject anything
        if !self.build_arguments.inject_loading_scripts || html.contains("__wbindgen_start") {
            return;
        }

        // If not, insert the script
        *html = html.replace(
            "</body",
            r#"<script>
            // We can't use a module script here because we need to start the script immediately when streaming
            import("/{base_path}/assets/dioxus/{app_name}.js").then(
                ({ default: init }) => {
                init("/{base_path}/assets/dioxus/{app_name}_bg.wasm").then((wasm) => {
                    if (wasm.__wbindgen_start == undefined) {
                    wasm.main();
                    }
                });
                }
            );
            </script></body"#,
        );

        // And try to insert preload links for the wasm and js files
        *html = html.replace(
            "</head",
            r#"<link rel="preload" href="/{base_path}/assets/dioxus/{app_name}_bg.wasm" as="fetch" type="application/wasm" crossorigin="">
            <link rel="preload" href="/{base_path}/assets/dioxus/{app_name}.js" as="script">
            </head"#);
    }

    /// Replace any special placeholders in the HTML with resolved values
    fn replace_template_placeholders(&self, html: &mut String) {
        let base_path = self.dioxus_crate.dioxus_config.web.app.base_path();
        *html = html.replace("{base_path}", base_path);

        let app_name = &self.dioxus_crate.dioxus_config.application.name;
        *html = html.replace("{app_name}", app_name);
    }
}

/// Read the html file from the crate root or use the default html file
fn html_or_default(crate_root: &Path) -> String {
    let custom_html_file = crate_root.join("index.html");
    std::fs::read_to_string(custom_html_file).unwrap_or_else(|_| String::from(DEFAULT_HTML))
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
