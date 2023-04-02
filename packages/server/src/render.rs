use std::fmt::Write;
use std::sync::Arc;

use dioxus_core::VirtualDom;
use dioxus_ssr::Renderer;

use crate::prelude::ServeConfig;

#[derive(Clone)]
pub struct SSRState {
    // We keep a cache of renderers to avoid re-creating them on every request. They are boxed to make them very cheap to move
    renderers: Arc<object_pool::Pool<Renderer>>,
    #[cfg(all(debug_assertions, feature = "hot-reload"))]
    // The cache of all templates that have been modified since the last time we checked
    templates: Arc<std::sync::RwLock<std::collections::HashSet<dioxus_core::Template<'static>>>>,
}

impl Default for SSRState {
    fn default() -> Self {
        #[cfg(all(debug_assertions, feature = "hot-reload"))]
        let templates = {
            let templates = Arc::new(std::sync::RwLock::new(std::collections::HashSet::new()));
            dioxus_hot_reload::connect({
                let templates = templates.clone();
                move |msg| match msg {
                    dioxus_hot_reload::HotReloadMsg::UpdateTemplate(template) => {
                        if let Ok(mut templates) = templates.write() {
                            templates.insert(template);
                        }
                    }
                    dioxus_hot_reload::HotReloadMsg::Shutdown => {
                        std::process::exit(0);
                    }
                }
            });
            templates
        };

        Self {
            renderers: Arc::new(object_pool::Pool::new(10, pre_renderer)),
            #[cfg(all(debug_assertions, feature = "hot-reload"))]
            templates,
        }
    }
}

impl SSRState {
    pub fn render<P: 'static + Clone>(&self, cfg: &ServeConfig<P>) -> String {
        let ServeConfig {
            app,
            application_name,
            base_path,
            head,
            props,
            ..
        } = cfg;

        let mut vdom = VirtualDom::new_with_props(*app, props.clone());

        let _ = vdom.rebuild();

        let mut renderer = self.renderers.pull(pre_renderer);

        let mut html = String::new();

        let result = write!(
            &mut html,
            r#"
        <!DOCTYPE html>
        <html>
        <head>{head}
        </head><body>
        <div id="main">"#
        );

        if let Err(err) = result {
            eprintln!("Failed to write to html: {}", err);
        }

        let _ = renderer.render_to(&mut html, &vdom);

        if let Err(err) = write!(
            &mut html,
            r#"</div>
        <script type="module">
        import init from "/{base_path}/assets/dioxus/{application_name}.js";
    init("/{base_path}/assets/dioxus/{application_name}_bg.wasm").then(wasm => {{
      if (wasm.__wbindgen_start == undefined) {{
          wasm.main();
        }}
    }});
    </script>
    </body>
    </html>"#
        ) {
            eprintln!("Failed to write to html: {}", err);
        }

        html
    }
}

fn pre_renderer() -> Renderer {
    let mut renderer = Renderer::default();
    renderer.pre_render = true;
    renderer
}
