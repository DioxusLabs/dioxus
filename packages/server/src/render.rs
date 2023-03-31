use std::fmt::Write;
use std::sync::Arc;

use dioxus_core::VirtualDom;
use dioxus_ssr::Renderer;

use crate::prelude::ServeConfig;

fn dioxus_ssr_html<P: 'static + Clone>(cfg: &ServeConfig<P>, renderer: &mut Renderer) -> String {
    let ServeConfig {
        app,
        application_name,
        base_path,
        head,
        props,
        ..
    } = cfg;

    let application_name = application_name.unwrap_or("dioxus");
    let mut vdom = VirtualDom::new_with_props(*app, props.clone());
    let _ = vdom.rebuild();
    let base_path = base_path.unwrap_or(".");

    let mut html = String::new();

    let result = match head {
        Some(head) => {
            write!(
                &mut html,
                r#"
        <!DOCTYPE html>
        <html>
        <head>{head}
        </head>
        <body>
        <div id="main">"#
            )
        }
        None => {
            write!(
                &mut html,
                r#"<title>Dioxus Application</title>
        <link rel="preload" href="/{base_path}/assets/dioxus/{application_name}_bg.wasm" as="fetch" type="application/wasm" crossorigin="" />
        <link rel="modulepreload" href="/{base_path}/assets/dioxus/{application_name}.js" />
        <meta content="text/html;charset=utf-8" http-equiv="Content-Type" />
        <meta name="viewport" content="width=device-width, initial-scale=1" />
        <meta charset="UTF-8" />"#
            )
        }
    };

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

#[derive(Clone)]
pub struct SSRState {
    // We keep a cache of renderers to avoid re-creating them on every request. They are boxed to make them very cheap to move
    renderers: Arc<object_pool::Pool<Renderer>>,
}

impl Default for SSRState {
    fn default() -> Self {
        Self {
            renderers: Arc::new(object_pool::Pool::new(10, pre_renderer)),
        }
    }
}

impl SSRState {
    pub fn render<P: 'static + Clone>(&self, cfg: &ServeConfig<P>) -> String {
        let mut renderer = self.renderers.pull(pre_renderer);
        dioxus_ssr_html(cfg, &mut renderer)
    }
}

fn pre_renderer() -> Renderer {
    let mut renderer = Renderer::default();
    renderer.pre_render = true;
    renderer
}
