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
//!     hydrate(2);
//! </script>
//! ```

use dioxus_interpreter_js::HYDRATE_JS;
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
            last_mount_id: 0,
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
    pub fn replace_placeholder(&mut self, id: Mount, html: impl Display) {
        // Make sure the client has the hydration function
        if !self.has_script {
            self.has_script = true;
            self.send_streaming_script();
        }

        // Then replace the suspense placeholder with the new content
        let resolved_id = id.id + 1;
        _ = self.channel.start_send(Ok(format!(
            r#"<div id="ds-{resolved_id}" hidden>{html}</div><script>hydrate({id})</script>"#
        )));
    }

    /// Sends the script that handles loading streaming chunks to the client
    fn send_streaming_script(&mut self) {
        let script = format!("<script type=\"module\">{HYDRATE_JS}</script>");
        _ = self.channel.start_send(Ok(script));
    }

    /// Close the stream with an error
    pub fn close_with_error(mut self, error: E) {
        _ = self.channel.start_send(Err(error));
    }
}

/// A mounted placeholder in the dom that may change in the future
#[derive(Clone, Copy)]
pub struct Mount {
    id: usize,
}

impl Display for Mount {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.id)
    }
}

#[test]
fn render_streaming() {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            use dioxus::prelude::*;
            use futures_util::StreamExt;
            use std::collections::HashMap;
            use std::sync::Arc;
            use std::sync::RwLock;

            let (tx, mut rx) = futures_channel::mpsc::channel(1000);
            {
                let stream = Arc::new(RwLock::new(
                    StreamingRenderer::<std::convert::Infallible>::new(
                        r#"<!DOCTYPE html><html><head><title>Dioxus</title></head><body>"#,
                        tx,
                    ),
                ));
                let scope_to_mount_mapping = Arc::new(RwLock::new(HashMap::new()));

                let mut ssr_renderer = crate::Renderer::new();
                {
                    let scope_to_mount_mapping = scope_to_mount_mapping.clone();
                    let stream = stream.clone();
                    ssr_renderer.set_render_components(move |renderer, to, vdom, scope| {
                        let is_suspense_boundary = vdom
                            .get_scope(scope)
                            .and_then(|s| SuspenseBoundaryProps::downcast_from_scope(s))
                            .filter(|s| s.suspended())
                            .is_some();
                        if is_suspense_boundary {
                            let mount = stream.write().unwrap().render_placeholder(
                                |to| renderer.render_scope(to, vdom, scope),
                                &mut *to,
                            )?;
                            scope_to_mount_mapping.write().unwrap().insert(scope, mount);
                        } else {
                            renderer.render_scope(to, vdom, scope)?
                        }
                        Ok(())
                    });
                }

                fn app() -> Element {
                    rsx! {
                        div {
                            "Hello world"
                        }
                        SuspenseBoundary {
                            fallback: |_| rsx! {
                                "Loading..."
                            },
                            SuspendedComponent {}
                        }
                        div {}
                    }
                }

                #[component]
                fn SuspendedComponent() -> Element {
                    use_resource(move || async move {
                        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                    })
                    .suspend()?;
                    rsx! {
                        "Suspended"
                    }
                }

                let mut dom = VirtualDom::new(app);
                dom.rebuild(&mut dioxus_core::NoOpMutations);

                let initial_frame = ssr_renderer.render(&dom);
                stream.write().unwrap().render(initial_frame);

                // Actually resolve suspense
                while dom.suspended_tasks_remaining() {
                    println!("waiting for suspense");
                    dom.wait_for_suspense_work().await;
                    let resolved_suspense_nodes = dom.render_suspense_immediate();

                    // Just rerender the resolved nodes
                    for scope in resolved_suspense_nodes {
                        let mount = {
                            let mut lock = scope_to_mount_mapping.write().unwrap();
                            lock.remove(&scope).unwrap()
                        };
                        let mut new_html = String::new();
                        ssr_renderer.render_scope(&mut new_html, &dom, scope)?;
                        stream.write().unwrap().replace_placeholder(mount, new_html);
                    }
                }

                stream.write().unwrap().render("</body></html>");
            }

            while let Some(Ok(chunk)) = rx.next().await {
                println!("{}", chunk);
            }

            Ok::<(), anyhow::Error>(())
        })
        .unwrap();
}
