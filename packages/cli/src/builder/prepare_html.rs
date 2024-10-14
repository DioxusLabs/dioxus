//! Build the HTML file to load a web application. The index.html file may be created from scratch or modified from the `index.html` file in the crate root.

use super::{BuildRequest, UpdateBuildProgress};
use crate::Result;
use crate::TraceSrc;
use futures_channel::mpsc::UnboundedSender;
use manganis_cli_support::AssetManifest;
use std::fmt::Write;
use std::path::{Path, PathBuf};

const DEFAULT_HTML: &str = include_str!("../../assets/index.html");
const TOAST_HTML: &str = include_str!("../../assets/toast.html");

impl BuildRequest {
    pub(crate) fn prepare_html(
        &self,
        assets: Option<&AssetManifest>,
        _progress: &mut UnboundedSender<UpdateBuildProgress>,
    ) -> Result<String> {
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
        for style in &style_list {
            writeln!(
                &mut head_resources,
                "<link rel=\"stylesheet\" href=\"{}\">",
                &style.to_str().unwrap(),
            )?;
        }

        if !style_list.is_empty() {
            self.send_resource_deprecation_warning(style_list, ResourceType::Style);
        }

        // Add all scripts to the head
        for script in &script_list {
            writeln!(
                &mut head_resources,
                "<script src=\"{}\"></script>",
                &script.to_str().unwrap(),
            )?;
        }

        if !script_list.is_empty() {
            self.send_resource_deprecation_warning(script_list, ResourceType::Script);
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
            </script>
            {DX_TOAST_UTILITIES}
            </body"#,
        );

        *html = match self.serve && !self.build_arguments.release {
            true => html.replace("{DX_TOAST_UTILITIES}", TOAST_HTML),
            false => html.replace("{DX_TOAST_UTILITIES}", ""),
        };

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

    fn send_resource_deprecation_warning(&self, paths: Vec<PathBuf>, variant: ResourceType) {
        const RESOURCE_DEPRECATION_MESSAGE: &str = r#"The `web.resource` config has been deprecated in favor of head components and will be removed in a future release."#;

        let replacement_components = paths
            .iter()
            .map(|path| {
                let path = if path.exists() {
                    path.to_path_buf()
                } else {
                    // If the path is absolute, make it relative to the current directory before we join it
                    // The path is actually a web path which is relative to the root of the website
                    let path = path.strip_prefix("/").unwrap_or(path);
                    let asset_dir_path = self.dioxus_crate.asset_dir().join(path);
                    if let Ok(absolute_path) = asset_dir_path.canonicalize() {
                        let absolute_crate_root =
                            self.dioxus_crate.crate_dir().canonicalize().unwrap();
                        PathBuf::from("./")
                            .join(absolute_path.strip_prefix(absolute_crate_root).unwrap())
                    } else {
                        path.to_path_buf()
                    }
                };
                match variant {
                    ResourceType::Style => format!(
                        "    document::Link {{ rel: \"stylesheet\", href: asset!(css(\"{}\")) }}",
                        path.display()
                    ),
                    ResourceType::Script => {
                        format!("    Script {{ src: asset!(file(\"{}\")) }}", path.display())
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

        tracing::warn!(dx_src = ?TraceSrc::Build, "{}", message);
    }
}

enum ResourceType {
    Style,
    Script,
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
