//! A shared pool of renderers for efficient server side rendering.

use std::sync::Arc;

use dioxus_core::VirtualDom;
use dioxus_ssr::Renderer;

use crate::prelude::ServeConfig;

/// State used in server side rendering. This utilizes a pool of [`dioxus_ssr::Renderer`]s to cache static templates between renders.
#[derive(Clone)]
pub struct SSRState {
    // We keep a pool of renderers to avoid re-creating them on every request. They are boxed to make them very cheap to move
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
    /// Render the application to HTML.
    pub fn render<P: 'static + Clone>(&self, cfg: &ServeConfig<P>) -> String {
        let ServeConfig {
            app, props, index, ..
        } = cfg;

        let mut vdom = VirtualDom::new_with_props(*app, props.clone());

        let _ = vdom.rebuild();

        let mut renderer = self.renderers.pull(pre_renderer);

        let mut html = String::new();

        html += &index.pre_main;

        let _ = renderer.render_to(&mut html, &vdom);

        html += &index.post_main;

        html
    }
}

fn pre_renderer() -> Renderer {
    let mut renderer = Renderer::default();
    renderer.pre_render = true;
    renderer
}
