//! A shared pool of renderers for efficient server side rendering.

use std::sync::Arc;

use dioxus::prelude::VirtualDom;
use dioxus_ssr::{
    incremental::{IncrementalRendererConfig, RenderFreshness, WrapBody},
    Renderer,
};
use serde::Serialize;

use crate::{prelude::*, server_context::with_server_context};
use dioxus::prelude::*;

enum SsrRendererPool {
    Renderer(object_pool::Pool<Renderer>),
    Incremental(object_pool::Pool<dioxus_ssr::incremental::IncrementalRenderer>),
}

impl SsrRendererPool {
    async fn render_to<P: Clone + Serialize + Send + Sync + 'static>(
        &self,
        cfg: &ServeConfig<P>,
        route: String,
        component: Component<P>,
        props: P,
        to: &mut WriteBuffer,
        server_context: &DioxusServerContext,
    ) -> Result<RenderFreshness, dioxus_ssr::incremental::IncrementalRendererError> {
        let wrapper = FullstackRenderer {
            cfg,
            server_context: server_context.clone(),
        };
        match self {
            Self::Renderer(pool) => {
                let server_context = Box::new(server_context.clone());
                let mut vdom = VirtualDom::new_with_props(component, props);

                with_server_context(server_context, || {
                    let _ = vdom.rebuild();
                });

                let mut renderer = pool.pull(pre_renderer);

                // SAFETY: The fullstack renderer will only write UTF-8 to the buffer.
                wrapper.render_before_body(&mut **to)?;
                renderer.render_to(to, &vdom)?;
                wrapper.render_after_body(&mut **to)?;

                Ok(RenderFreshness::now(None))
            }
            Self::Incremental(pool) => {
                let mut renderer =
                    pool.pull(|| incremental_pre_renderer(cfg.incremental.as_ref().unwrap()));
                Ok(renderer
                    .render(
                        route,
                        component,
                        props,
                        &mut **to,
                        |vdom| {
                            let server_context = Box::new(server_context.clone());
                            with_server_context(server_context, || {
                                let _ = vdom.rebuild();
                            });
                        },
                        &wrapper,
                    )
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
                    || incremental_pre_renderer(cfg.incremental.as_ref().unwrap()),
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
        server_context: &'a DioxusServerContext,
    ) -> impl std::future::Future<
        Output = Result<RenderResponse, dioxus_ssr::incremental::IncrementalRendererError>,
    > + Send
           + 'a {
        async move {
            let mut html = WriteBuffer { buffer: Vec::new() };
            let ServeConfig { app, props, .. } = cfg;

            let freshness = self
                .renderers
                .render_to(cfg, route, *app, props.clone(), &mut html, server_context)
                .await?;

            Ok(RenderResponse {
                html: String::from_utf8(html.buffer).map_err(|err| {
                    dioxus_ssr::incremental::IncrementalRendererError::Other(Box::new(err))
                })?,
                freshness,
            })
        }
    }
}

struct FullstackRenderer<'a, P: Clone + Send + Sync + 'static> {
    cfg: &'a ServeConfig<P>,
    server_context: DioxusServerContext,
}

impl<'a, P: Clone + Serialize + Send + Sync + 'static> dioxus_ssr::incremental::WrapBody
    for FullstackRenderer<'a, P>
{
    fn render_before_body<R: std::io::Write>(
        &self,
        to: &mut R,
    ) -> Result<(), dioxus_ssr::incremental::IncrementalRendererError> {
        let ServeConfig { index, .. } = &self.cfg;

        to.write_all(index.pre_main.as_bytes())?;

        Ok(())
    }

    fn render_after_body<R: std::io::Write>(
        &self,
        to: &mut R,
    ) -> Result<(), dioxus_ssr::incremental::IncrementalRendererError> {
        // serialize the props
        crate::html_storage::serialize::encode_props_in_element(&self.cfg.props, to)?;
        // serialize the server state
        crate::html_storage::serialize::encode_in_element(
            &*self.server_context.html_data().map_err(|err| {
                dioxus_ssr::incremental::IncrementalRendererError::Other(Box::new({
                    #[derive(Debug)]
                    struct HTMLDataReadError;

                    impl std::fmt::Display for HTMLDataReadError {
                        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                            f.write_str(
                                "Failed to read the server data to serialize it into the HTML",
                            )
                        }
                    }

                    impl std::error::Error for HTMLDataReadError {}

                    HTMLDataReadError
                }))
            })?,
            to,
        )?;

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

            to.write_all(r#"<script>"#.as_bytes())?;
            to.write_all(disconnect_js.as_bytes())?;
            to.write_all(r#"</script>"#.as_bytes())?;
        }

        let ServeConfig { index, .. } = &self.cfg;

        to.write_all(index.post_main.as_bytes())?;

        Ok(())
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

fn incremental_pre_renderer(
    cfg: &IncrementalRendererConfig,
) -> dioxus_ssr::incremental::IncrementalRenderer {
    let mut renderer = cfg.clone().build();
    renderer.renderer_mut().pre_render = true;
    renderer
}

#[cfg(all(feature = "ssr", feature = "router"))]
/// Pre-caches all static routes
pub async fn pre_cache_static_routes_with_props<Rt>(
    cfg: &crate::prelude::ServeConfig<crate::router::FullstackRouterConfig<Rt>>,
) -> Result<(), dioxus_ssr::incremental::IncrementalRendererError>
where
    Rt: dioxus_router::prelude::Routable + Send + Sync + Serialize,
    <Rt as std::str::FromStr>::Err: std::fmt::Display,
{
    let wrapper = FullstackRenderer { cfg };
    let mut renderer = incremental_pre_renderer(
        cfg.incremental
            .as_ref()
            .expect("incremental renderer config must be set to pre-cache static routes"),
    );

    dioxus_router::incremental::pre_cache_static_routes::<Rt, _>(&mut renderer, &wrapper).await
}

struct WriteBuffer {
    buffer: Vec<u8>,
}

impl std::fmt::Write for WriteBuffer {
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        self.buffer.extend_from_slice(s.as_bytes());
        Ok(())
    }
}

impl std::ops::Deref for WriteBuffer {
    type Target = Vec<u8>;

    fn deref(&self) -> &Self::Target {
        &self.buffer
    }
}

impl std::ops::DerefMut for WriteBuffer {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.buffer
    }
}
