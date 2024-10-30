//! There are two common ways to render suspense:
//! 1. Stream the HTML in order - this will work even if javascript is disabled, but if there is something slow at the top of your page, and fast at the bottom, nothing will render until the slow part is done
//! 2. Render placeholders and stream the HTML out of order - this will only work if javascript is enabled. This lets you render any parts of your page that resolve quickly, and then render the rest of the page as it becomes available
//!
//! Dioxus currently uses a the second out of order streaming approach which requires javascript. The rendering structure is as follows:
//! ```html
//! // Initial content is sent down with placeholders
//! <div>
//!     Header
//!     <div class="flex flex-col">
//!         // If we reach a suspense placeholder that may be replaced later, we insert a template node with a unique id to replace later
//!         <div>Loading user info...</div>
//!     </div>
//!     Footer
//! </div>
//! // After the initial render is done, we insert divs that are hidden with new content.
//! // We use divs instead of templates for better SEO
//! <script>
//!     // Code to hook up hydration replacement
//! </script>
//! <div hidden id="ds-1-r">
//!     <div>Final HTML</div>
//! </div>
//! <script>
//!     window.dx_hydrate(2, "suspenseboundarydata");
//! </script>
//! ```

use crate::Result;
use futures_channel::mpsc::UnboundedSender;
use std::{
    fmt::{Display, Write},
    sync::{Arc, RwLock},
};

/// Sections are identified by a unique id based on the suspense path. We only track the path of suspense boundaries because the client may render different components than the server.
#[derive(Clone, Debug, Default)]
pub struct Mount {
    pub parent: Option<Arc<Mount>>,
    pub id: usize,
}

impl Mount {
    fn child(&self) -> Self {
        Self {
            parent: Some(Arc::new(self.clone())),
            id: 0,
        }
    }
}

impl Display for Mount {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(parent) = &self.parent {
            write!(f, "{},", parent)?;
        }
        write!(f, "{}", self.id)
    }
}

pub(crate) struct StreamingRenderer {
    channel: UnboundedSender<Result<String>>,
    current_path: RwLock<Mount>,
}

impl StreamingRenderer {
    /// Create a new streaming renderer with the given head that renders into a channel
    pub(crate) fn start(before_body: String, render_into: UnboundedSender<Result<String>>) -> Self {
        _ = render_into.unbounded_send(Ok(before_body));

        Self {
            channel: render_into.into(),
            current_path: Default::default(),
        }
    }

    /// Render a new chunk of html that will never change
    pub(crate) fn render(&self, html: String) {
        _ = self.channel.unbounded_send(Ok(html));
    }

    /// Render a new chunk of html that may change
    pub(crate) fn render_placeholder<W: Write + ?Sized>(
        &self,
        into: &mut W,
        html: impl FnOnce(&mut W) -> std::fmt::Result,
    ) -> Result<Mount> {
        let id = self.current_path.read().unwrap().clone();
        // Increment the id for the next placeholder
        self.current_path.write().unwrap().id += 1;
        // While we are inside the placeholder, set the suspense path to the suspense boundary that we are rendering
        let old_path = std::mem::replace(&mut *self.current_path.write().unwrap(), id.child());
        html(into)?;
        // Restore the old path
        *self.current_path.write().unwrap() = old_path;
        Ok(id)
    }

    /// Replace a placeholder that was rendered previously
    pub(crate) fn replace_placeholder<W: Write + ?Sized>(
        &self,
        id: Mount,
        html: impl FnOnce(&mut W) -> std::fmt::Result,
        data: impl Display,
        into: &mut W,
    ) -> std::fmt::Result {
        // Then replace the suspense placeholder with the new content
        write!(into, r#"<div id="ds-{id}-r" hidden>"#)?;
        // While we are inside the placeholder, set the suspense path to the suspense boundary that we are rendering
        let old_path = std::mem::replace(&mut *self.current_path.write().unwrap(), id.child());
        html(into)?;
        // Restore the old path
        *self.current_path.write().unwrap() = old_path;
        write!(
            into,
            r#"</div><script>window.dx_hydrate([{id}], "{data}")</script>"#
        )
    }

    /// Close the stream with an error
    pub(crate) fn close_with_error(&self, error: crate::Error) {
        _ = self.channel.unbounded_send(Err(error));
    }
}
