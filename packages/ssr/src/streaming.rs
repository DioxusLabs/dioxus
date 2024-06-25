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
//!         <dx-hydration>
//!             <template id="ds-0" shadowrootmode="open">
//!                 <div>Loading user info...</div>
//!             </template>
//!         </dx-hydration>
//!     </div>
//!     Footer
//! </div>
//! // After the initial render is done, we insert divs that are hidden with new content.
//! // We use divs instead of templates for better SEO
//! <script>
//!     // Code to hook up hydration replacement
//! </script>
//! <div hidden id="ds-1">
//!     <div>Final HTML</div>
//! </div>
//! <script>
//!     window.dx_hydrate(2, "suspenseboundarydata");
//! </script>
//! ```

use dioxus_interpreter_js::STREAMING_JS;
use futures_channel::mpsc::Sender;

use std::fmt::{Display, Write};

pub struct StreamingRenderer<E = std::convert::Infallible> {
    channel: Sender<Result<String, E>>,
    last_mount_id: usize,
    has_script: bool,
}

impl<E> StreamingRenderer<E> {
    /// Create a new streaming renderer with the given head that renders into a channel
    pub fn new(before_body: impl Display, mut render_into: Sender<Result<String, E>>) -> Self {
        let start_html = before_body.to_string();
        _ = render_into.start_send(Ok(start_html));

        Self {
            channel: render_into,
            // We start on id 2 because the first id is reserved for the initial html chunk sent to the client
            last_mount_id: 2,
            has_script: false,
        }
    }

    /// Render a new chunk of html that will never change
    pub fn render(&mut self, html: impl Display) {
        _ = self.channel.start_send(Ok(html.to_string()));
    }

    /// Render a new chunk of html that may change
    pub fn render_placeholder<W: Write + ?Sized>(
        &mut self,
        html: impl FnOnce(&mut W) -> std::fmt::Result,
        into: &mut W,
    ) -> Result<Mount, std::fmt::Error> {
        let id = self.last_mount_id;
        // Increment the id by 2 so that we don't re use the same id again.
        // The next id is reserved for the id that will replace this node
        self.last_mount_id += 2;
        write!(
            into,
            r#"<dx-hydration id="ds-{id}"><template shadowrootmode="open">"#
        )?;
        html(into)?;
        write!(into, r#"</template></dx-hydration>"#)?;
        Ok(Mount { id })
    }

    /// Replace a placeholder that was rendered previously
    pub fn replace_placeholder<W: Write + ?Sized>(
        &mut self,
        id: Mount,
        html: impl Display,
        data: impl Display,
        into: &mut W,
    ) -> std::fmt::Result {
        // Make sure the client has the hydration function
        if !self.has_script {
            self.has_script = true;
            self.send_streaming_script();
        }

        // Then replace the suspense placeholder with the new content
        let resolved_id = id.id + 1;
        write!(
            into,
            r#"<div id="ds-{resolved_id}" hidden>{html}</div><script>window.dx_hydrate({id}, "{data}")</script>"#
        )
    }

    /// Sends the script that handles loading streaming chunks to the client
    fn send_streaming_script(&mut self) {
        let script = format!("<script>{STREAMING_JS}</script>");
        _ = self.channel.start_send(Ok(script));
    }

    /// Close the stream with an error
    pub fn close_with_error(&mut self, error: E) {
        _ = self.channel.start_send(Err(error));
    }
}

/// A mounted placeholder in the dom that may change in the future
#[derive(Clone, Copy, Debug, Default)]
pub struct Mount {
    id: usize,
}

impl Display for Mount {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.id)
    }
}
