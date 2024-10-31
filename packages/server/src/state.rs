use crate::prelude::*;
use crate::{document::ServerDocument, CachedPages};
use crate::{
    streaming::{Mount, StreamingRenderer},
    IncrementalRendererError,
};
// use crate::{CachedRender, IncrementalRenderer, RenderFreshness};
use crate::{DioxusServerContext, Error, IndexHtml, RenderChunk, StreamingResponse};
use crate::{Result, ServeConfig};
use axum::{
    body::Body,
    response::{Html, IntoResponse, Response},
};
use dioxus_lib::document::Document;
use dioxus_lib::prelude::VirtualDom;
use dioxus_lib::prelude::*;
use dioxus_ssr::Renderer;
use futures_channel::mpsc::Sender;
use futures_channel::{mpsc::UnboundedSender, oneshot};
use futures_util::Stream;
use futures_util::{stream, StreamExt, TryFutureExt};
use http::Request;
use http::{uri::PathAndQuery, StatusCode};
use std::rc::Rc;
use std::sync::Arc;
use std::{collections::HashMap, future::Future};
use std::{fmt::Write, sync::RwLock};
use tokio::task::JoinHandle;

pub struct ServerState {
    cfg: ServeConfig,
    index: IndexHtml,
    vdom_factory: Arc<dyn Fn() -> VirtualDom + Send + Sync>,
    cache: CachedPages,
}

pub type SharedServerState = Arc<ServerState>;
pub type ChunkTx = UnboundedSender<Result<RenderChunk>>;

impl ServerState {
    pub fn new(cfg: ServeConfig) -> SharedServerState {
        // // The CLI always bundles static assets into the exe/public directory
        // let public_path = public_path();

        // let index_path = self
        //     .index_path
        //     .map(PathBuf::from)
        //     .unwrap_or_else(|| public_path.join("index.html"));

        // let root_id = self.root_id.unwrap_or("main");

        // let index_html = match self.index_html {
        //     Some(index) => index,
        //     None => load_index_path(index_path).unwrap_or_default(),
        // };
        // let index = load_index_html(index_html, root_id);

        todo!()
    }

    pub async fn respond(self: Arc<Self>, request: Request<Body>) -> Result<Response<Body>> {
        let mut stream = StreamingResponse::new();

        // Run the virtualdom rendering on a local task so we can run the !Send futures
        // If the task fails while rendering it'll bring down the response
        let tx = stream.tx();
        tokio::task::spawn_local(async move {
            self.stream(request, tx.clone())
                .map_err(|err| tx.unbounded_send(Err(err)))
                .await
        });

        // Wait for the first chunk to be rendered
        // This will give us the "shell" as well as the status code in case of an error
        // Take out any headers (like 404s or custom error codes) to be attached to the response
        let mut first_chunk = stream.next().await.ok_or_else(|| crate::Error::Crash())??;
        let first_chunk_headers = std::mem::take(&mut first_chunk.headers);

        // Now assemble the body with the immediately ready chunk and the following chunks if need be
        let chunks = stream::once(async move { Ok(first_chunk) }).chain(stream);
        let mut response = Html::from(Body::from_stream(chunks)).into_response();

        // Attach any headers from the first chunk
        response.headers_mut().extend(first_chunk_headers);

        Ok(response)
    }

    async fn stream(&self, request: Request<Body>, tx: ChunkTx) -> Result<()> {
        let (parts, _body) = request.into_parts();
        let url = parts
            .uri
            .path_and_query()
            .ok_or_else(|| Error::Http(StatusCode::BAD_REQUEST))?;

        // Retrieve the cached page if it exists
        // todo(jon): probably want a much more sophisticated caching strategy here
        let mut should_cache_page = false;
        if let Some(page) = self.cache.get(url.as_str()) {
            if page.is_fresh() {
                tx.unbounded_send(Ok(page.to_chunk()));
                return Ok(());
            }

            should_cache_page = true;
        }

        let ctx = DioxusServerContext::new(parts);
        let mut virtualdom = self.new_vdom();
        ctx.run_with(|| virtualdom.rebuild_in_place());

        // Wait for suspense boundaries to resolve until we receive a router

        Ok(())
    }

    pub fn new_vdom(&self) -> VirtualDom {
        let mut vdom = (self.vdom_factory)();

        // vdom.provide_root_context(context);
        // vdom.provide_root_context(context);

        // vdom.rebuild_in_place();

        vdom
    }

    /// Render any content before the head of the page.
    pub fn render_head(&self, to: &mut String, virtual_dom: &VirtualDom) -> Result<()> {
        let title = {
            let document: Option<Rc<ServerDocument>> =
                virtual_dom.in_runtime(|| ScopeId::ROOT.consume_context());
            // Collect any head content from the document provider and inject that into the head
            document.and_then(|document| document.title())
        };

        to.write_str(&self.index.head_before_title)?;
        if let Some(title) = title {
            to.write_str(&title)?;
        } else {
            to.write_str(&self.index.title)?;
        }
        to.write_str(&self.index.head_after_title)?;

        let document: Option<Rc<ServerDocument>> =
            virtual_dom.in_runtime(|| ScopeId::ROOT.consume_context());

        if let Some(document) = document {
            // Collect any head content from the document provider and inject that into the head
            document.render(to)?;

            // Enable a warning when inserting contents into the head during streaming
            document.start_streaming();
        }

        self.render_before_body(to)?;

        Ok(())
    }

    /// Render any content before the body of the page.
    fn render_before_body(&self, to: &mut String) -> Result<()> {
        to.write_str(&self.index.close_head)?;

        use dioxus_interpreter_js::INITIALIZE_STREAMING_JS;
        write!(to, "<script>{INITIALIZE_STREAMING_JS}</script>")?;

        Ok(())
    }

    /// Render all content after the main element of the page.
    pub fn render_after_main(&self, to: &mut String, virtual_dom: &VirtualDom) -> Result<()> {
        // Collect the initial server data from the root node. For most apps, no use_server_futures will be resolved initially, so this will be full on `None`s.
        // Sending down those Nones are still important to tell the client not to run the use_server_futures that are already running on the backend
        let resolved_data = serialize_server_data(virtual_dom, ScopeId::ROOT);
        write!(
            to,
            r#"<script>window.initial_dioxus_hydration_data="{resolved_data}";</script>"#,
        )?;
        to.write_str(&self.index.post_main)?;

        Ok(())
    }

    /// Render all content after the body of the page.
    pub fn render_after_body(&self, to: &mut String) -> Result<()> {
        to.write_str(&self.index.after_closing_body_tag)?;

        Ok(())
    }

    /// Wrap a body in the template
    pub fn wrap_body(
        &self,
        to: &mut String,
        virtual_dom: &VirtualDom,
        body: impl std::fmt::Display,
    ) -> Result<()> {
        self.render_head(to, virtual_dom)?;
        write!(to, "{body}")?;
        self.render_after_main(to, virtual_dom)?;
        self.render_after_body(to)?;

        Ok(())
    }
}

pub fn serialize_server_data(virtual_dom: &VirtualDom, scope: ScopeId) -> String {
    "extract shared serialize out".to_string()
    // todo!("extract shared serialize out")
    // // After we replace the placeholder in the dom with javascript, we need to send down the resolved data so that the client can hydrate the node
    // // Extract any data we serialized for hydration (from server futures)
    // let html_data =
    //     crate::html_storage::HTMLData::extract_from_suspense_boundary(virtual_dom, scope);

    // // serialize the server state into a base64 string
    // html_data.serialized()
}
