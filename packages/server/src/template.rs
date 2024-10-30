//! A shared pool of renderers for efficient server side rendering.
use crate::prelude::*;
use crate::Result;
use crate::{document::ServerDocument, ServeConfig};
use crate::{
    streaming::{Mount, StreamingRenderer},
    IncrementalRendererError,
};
use crate::{CachedRender, IncrementalRenderer, RenderFreshness};
use dioxus_lib::document::Document;
use dioxus_lib::prelude::*;
use dioxus_ssr::Renderer;
use futures_channel::mpsc::Sender;
use futures_util::{Stream, StreamExt};
use std::{collections::HashMap, future::Future};
use std::{fmt::Write, sync::RwLock};
use std::{rc::Rc, sync::Arc};
use tokio::task::JoinHandle;

/// The template that wraps the body of the HTML for a fullstack page. This template contains the data needed to hydrate server functions that were run on the server.
pub struct FullstackHTMLTemplate {
    pub cfg: ServeConfig,
}

impl FullstackHTMLTemplate {
    /// Render any content before the head of the page.
    pub fn render_head<R: Write>(&self, to: &mut R, virtual_dom: &VirtualDom) -> Result<()> {
        let ServeConfig { index, .. } = &self.cfg;

        let title = {
            let document: Option<Rc<ServerDocument>> =
                virtual_dom.in_runtime(|| ScopeId::ROOT.consume_context());
            // Collect any head content from the document provider and inject that into the head
            document.and_then(|document| document.title())
        };

        to.write_str(&index.head_before_title)?;
        if let Some(title) = title {
            to.write_str(&title)?;
        } else {
            to.write_str(&index.title)?;
        }
        to.write_str(&index.head_after_title)?;

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
    fn render_before_body<R: Write>(&self, to: &mut R) -> Result<()> {
        let ServeConfig { index, .. } = &self.cfg;

        to.write_str(&index.close_head)?;

        use dioxus_interpreter_js::INITIALIZE_STREAMING_JS;
        write!(to, "<script>{INITIALIZE_STREAMING_JS}</script>")?;

        Ok(())
    }

    /// Render all content after the main element of the page.
    pub fn render_after_main<R: Write>(&self, to: &mut R, virtual_dom: &VirtualDom) -> Result<()> {
        let ServeConfig { index, .. } = &self.cfg;

        // Collect the initial server data from the root node. For most apps, no use_server_futures will be resolved initially, so this will be full on `None`s.
        // Sending down those Nones are still important to tell the client not to run the use_server_futures that are already running on the backend
        let resolved_data = serialize_server_data(virtual_dom, ScopeId::ROOT);
        write!(
            to,
            r#"<script>window.initial_dioxus_hydration_data="{resolved_data}";</script>"#,
        )?;
        to.write_str(&index.post_main)?;

        Ok(())
    }

    /// Render all content after the body of the page.
    pub fn render_after_body<R: Write>(&self, to: &mut R) -> Result<()> {
        let ServeConfig { index, .. } = &self.cfg;

        to.write_str(&index.after_closing_body_tag)?;

        Ok(())
    }

    /// Wrap a body in the template
    pub fn wrap_body<R: Write>(
        &self,
        to: &mut R,
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
