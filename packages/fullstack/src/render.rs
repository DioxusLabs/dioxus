//! A shared pool of renderers for efficient server side rendering.

use std::sync::Arc;

use dioxus::prelude::VirtualDom;
use dioxus_ssr::{
    incremental::{IncrementalRendererConfig, RenderFreshness},
    Renderer,
};

use crate::prelude::*;
use dioxus::prelude::*;

enum SsrRendererPool {
    Renderer(object_pool::Pool<Renderer>),
    Incremental(
        object_pool::Pool<
            dioxus_ssr::incremental::IncrementalRenderer<
                crate::serve_config::EmptyIncrementalRenderTemplate,
            >,
        >,
    ),
}

impl SsrRendererPool {
    async fn render_to<P: Clone + 'static>(
        &self,
        cfg: &ServeConfig<P>,
        route: String,
        component: Component<P>,
        props: P,
        to: &mut String,
        modify_vdom: impl FnOnce(&mut VirtualDom),
    ) -> Result<RenderFreshness, dioxus_ssr::incremental::IncrementalRendererError> {
        match self {
            Self::Renderer(pool) => {
                let mut vdom = VirtualDom::new_with_props(component, props);
                modify_vdom(&mut vdom);

                let _ = vdom.rebuild();
                let mut renderer = pool.pull(pre_renderer);
                renderer.render_to(to, &vdom)?;

                Ok(RenderFreshness::now(None))
            }
            Self::Incremental(pool) => {
                let mut renderer = pool.pull(|| incremental_pre_renderer(cfg));
                Ok(renderer
                    .render_to_string(route, component, props, to, modify_vdom)
                    .await?)
            }
        }
    }
}

/// State used in server side rendering. This utilizes a pool of [`dioxus_ssr::Renderer`]s to cache static templates between renders.
#[derive(Clone)]
pub struct SSRState {
    // We keep a pool of renderers to avoid re-creating them on every request. They are boxed to make them very cheap to move
    renderers: Arc<SsrRendererPool>,
}

impl SSRState {
    pub(crate) fn new<P: Clone>(cfg: &ServeConfig<P>) -> Self {
        if cfg.incremental.is_some() {
            return Self {
                renderers: Arc::new(SsrRendererPool::Incremental(object_pool::Pool::new(
                    10,
                    || incremental_pre_renderer(cfg),
                ))),
            };
        }

        Self {
            renderers: Arc::new(SsrRendererPool::Renderer(object_pool::Pool::new(
                10,
                pre_renderer,
            ))),
        }
    }

    /// Render the application to HTML.
    pub fn render<'a, P: 'static + Clone + serde::Serialize + Send + Sync>(
        &'a self,
        route: String,
        cfg: &'a ServeConfig<P>,
        modify_vdom: impl FnOnce(&mut VirtualDom) + Send + 'a,
    ) -> impl std::future::Future<
        Output = Result<RenderResponse, dioxus_ssr::incremental::IncrementalRendererError>,
    > + Send
           + 'a {
        async move {
            let ServeConfig { app, props, .. } = cfg;

            let ServeConfig { index, .. } = cfg;

            let mut html = String::new();

            html += &index.pre_main;

            let freshness = self
                .renderers
                .render_to(cfg, route, *app, props.clone(), &mut html, modify_vdom)
                .await?;

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

            Ok(RenderResponse { html, freshness })
        }
    }
}

/// A rendered response from the server.
pub struct RenderResponse {
    pub(crate) html: String,
    pub(crate) freshness: RenderFreshness,
}

impl RenderResponse {
    /// Get the rendered HTML.
    pub fn html(&self) -> &str {
        &self.html
    }

    /// Get the freshness of the rendered HTML.
    pub fn freshness(&self) -> RenderFreshness {
        self.freshness
    }
}

fn pre_renderer() -> Renderer {
    let mut renderer = Renderer::default();
    renderer.pre_render = true;
    renderer.into()
}

fn incremental_pre_renderer<P: Clone>(
    cfg: &ServeConfig<P>,
) -> dioxus_ssr::incremental::IncrementalRenderer<crate::serve_config::EmptyIncrementalRenderTemplate>
{
    let builder: &IncrementalRendererConfig<_> = &*cfg.incremental.as_ref().unwrap();
    let mut renderer = builder.clone().build();
    renderer.renderer_mut().pre_render = true;
    renderer
}
