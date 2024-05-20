/// There are two common ways to render suspense:
/// 1. Stream the HTML in order - this will work even if javascript is disabled, but if there is something slow at the top of your page, and fast at the bottom, nothing will render until the slow part is done
/// 2. Render placeholders and stream the HTML out of order - this will only work if javascript is enabled. This lets you render any parts of your page that resolve quickly, and then render the rest of the page as it becomes available
///
/// Dioxus uses a hybrid approach. It does out of order rendering, but it doesn't require javascript/wasm to be enabled.
/// We use the declarative shadow dom to render the placeholders, and replace those placeholders with the real content as soon as the data is available.
///
/// The rendering structure is as follows:
/// ```html
/// // Streaming content is all placed inside a div with the id "streaming-dioxus-root"
/// <div id="streaming-dioxus-root">
///     // Each template needs to be placed inside a different div
///     <div>
///         // Each time we render, we place it inside a different template that may be replaced later
///         <template shadowrootmode="open">
///             // The slot id will be used later to replace the template
///             <slot name="dioxus-0">
///                 // Render the current content here
///                 <div>Loading...</div>
///             </slot>
///         </template>
///         // Next time new content is available, we replace the template by rendering a div with the slot id of the old template
///         <div slot="dioxus-0">
///             <template shadowrootmode="open">
///                 // We use a new id for the slot
///                 <slot name="dioxus-1">
///                     <div>Loading...more?</div>
///                 </slot>
///             </template>
///             // If we get too deeply nested, we clear the slot and start a new one
///             <div slot="dioxus-1">
///                 <div style="display: none"></div>
///             </div>
///         </div>
///     </div>
///     // start a new slot here
/// </div>
/// // After suspense is done, we inject a tiny bit of javascript to clear out the useless html
/// <script>...</script>
/// // And then render the final html with all the hydration ids we need to render
/// <div data-dioxus-hydration-id="0">
///     <div>Final HTML</div>
/// </div>
/// ```
use std::fmt::Display;

use futures_channel::mpsc::{channel, Receiver, Sender};

// Browsers tend not to like it if you nest your HTML forever. (chrome crashes at ~600 nested elements)
// But there is less flickering if you nest html instead of clearing and re-rendering.
// This value controls how many nested elements we can have before we clear and re-render.
const MAX_REPLACE_DEPTH: usize = 50;

pub struct StreamingRenderer {
    depth: usize,
    last_mount: usize,
    channel: Sender<String>,
    root: Mount,
}

impl StreamingRenderer {
    /// Create a new streaming renderer with the given head
    pub fn create(head: impl Display) -> (Self, Receiver<String>) {
        let (tx, rx) = channel(100);
        (Self::new(head, tx), rx)
    }

    /// Create a new streaming renderer with the given head that renders into a channel
    pub fn new(before_body: impl Display, mut render_into: Sender<String>) -> Self {
        let start_html = format!(r#"{before_body}<div id="streaming-dioxus-root">"#);
        _ = render_into.start_send(start_html);

        Self {
            depth: 0,
            last_mount: 0,
            channel: render_into,
            root: Mount { id: 0 },
        }
    }

    /// Render new content in the body. This will clear out the old content and replace it with the new content (without JS/WASM)
    pub fn render(&mut self, html: String) {
        // If we are too deeply nested, clear the slot and start a new one
        if self.depth > MAX_REPLACE_DEPTH {
            self.clear();
            // Create a new root template (each template needs to be in a unique element so we create a wrapper div)
            let _ = self.channel.start_send(r#"<div>"#.to_string());
            self.depth += 1;
            self.root = self.create_template(html);
        } else {
            self.root = self.replace(self.root, html);
        }
    }

    // Clear the current slot and reset the depth
    fn clear(&mut self) {
        self.root = self.replace(self.root, String::new());
        self.close();
    }

    // Reset the depth
    fn close(&mut self) {
        let close = "</div>".repeat(self.depth);
        _ = self.channel.start_send(close);
        self.depth = 0;
    }

    fn mount(&mut self) -> Mount {
        let mount = self.last_mount;
        self.last_mount += 1;
        Mount { id: mount }
    }

    fn start_slot(&mut self) -> Mount {
        let mount = self.mount();
        let id = mount.id;
        _ = self
            .channel
            .start_send(format!(r#"<slot name="dioxus-{id}">"#));
        mount
    }

    fn end_slot(&mut self) {
        _ = self.channel.start_send("</slot>".to_string());
    }

    // Replace a slot with new content. The new content should be sent after this method is called
    fn replace_slot(&mut self, mount: Mount) {
        let mounted_id = mount.id;
        _ = self
            .channel
            .start_send(format!(r#"<div slot="dioxus-{mounted_id}">"#));

        // The div is still open, keep track of the depth
        self.depth += 1;
    }

    fn replace(&mut self, mount: Mount, html: String) -> Mount {
        // replace the old slot
        self.replace_slot(mount);
        // and create a new template inside the slot that may be replaced later
        self.create_template(html)
    }

    fn create_template(&mut self, html: String) -> Mount {
        // Start a div and a template. Each div can only have one template.
        _ = self
            .channel
            .start_send(r#"<template shadowrootmode="open">"#.to_string());

        // Fill the template with some default content that will be replaced when we fill in the slot (when you call replace on mount)
        let mount = self.start_slot();
        _ = self.channel.start_send(html);
        self.end_slot();

        // Only end the template with the default content
        _ = self.channel.start_send("</template>".to_string());
        mount
    }

    /// Finish streaming the content and set the final HTML that will remain in the body
    pub fn finish_streaming(mut self, final_html: String) {
        // Clear the current slot and reset the depth
        self.clear();
        // Close the root div
        _ = self.channel.start_send("</div>".to_string());
        // After suspense is done, we inject a tiny bit of javascript to clear out the useless html
        let _ = self.channel.start_send(
            "<script>document.getElementById('streaming-dioxus-root').remove();</script>"
                .to_string(),
        );
        // And then render the final html with all the hydration ids we need to render
        let _ = self.channel.start_send(final_html);
    }
}

#[derive(Clone, Copy)]
struct Mount {
    id: usize,
}
