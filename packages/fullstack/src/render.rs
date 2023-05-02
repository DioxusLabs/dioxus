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
    pub fn render<P: 'static + Clone + serde::Serialize>(&self, cfg: &ServeConfig<P>) -> String {
        let ServeConfig { app, props, .. } = cfg;

        let mut vdom = VirtualDom::new_with_props(*app, props.clone());

        let _ = vdom.rebuild();

        self.render_vdom(&vdom, cfg)
    }

    /// Render a VirtualDom to HTML.
    pub fn render_vdom<P: 'static + Clone + serde::Serialize>(
        &self,
        vdom: &VirtualDom,
        cfg: &ServeConfig<P>,
    ) -> String {
        let ServeConfig { index, .. } = cfg;

        let mut renderer = self.renderers.pull(pre_renderer);

        let mut html = String::new();

        html += &index.pre_main;

        let _ = renderer.render_to(&mut html, vdom);

        // serialize the props
        let _ = crate::props_html::serialize_props::encode_in_element(&cfg.props, &mut html);

        #[cfg(all(debug_assertions, feature = "hot-reload"))]
        {
            // In debug mode, we need to add a script to the page that will reload the page if the websocket disconnects to make full recompile hot reloads work
            let disconnect_js = r#"(function () {
    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    const url = protocol + '//' + window.location.host + '/_dioxus/disconnect';
    const poll_interval = 1000;
    const reload_upon_connect = () => {
        console.log('Disconnected from server. Attempting to reconnect...');
        window.setTimeout(
            () => {
                // Try to reconnect to the websocket
                const ws = new WebSocket(url);
                ws.onopen = () => {
                    // If we reconnect, reload the page
                    window.location.reload();
                }
                // Otherwise, try again in a second
                reload_upon_connect();
            },
            poll_interval);
    };

    // on initial page load connect to the disconnect ws
    const ws = new WebSocket(url);
    // if we disconnect, start polling
    ws.onclose = reload_upon_connect;
})()"#;

            html += r#"<script>"#;
            html += disconnect_js;
            html += r#"</script>"#;
        }

        html += &index.post_main;

        html
    }
}

fn pre_renderer() -> Renderer {
    let mut renderer = Renderer::default();
    renderer.pre_render = true;
    renderer
}
