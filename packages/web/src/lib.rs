//! Dioxus WebSys
//!
//! This crate implements a renderer of the Dioxus Virtual DOM for the web browser.
//!
//! While `VNode` supports "to_string" directly, it renders child components as the RSX! macro tokens. For custom components,
//! an external renderer is needed to progress the component lifecycles. The `WebsysRenderer` shows how to use the Virtual DOM
//! API to progress these lifecycle events to generate a fully-mounted Virtual DOM instance which can be renderer in the
//! `render` method.
//!
//! ```ignore
//! fn main() {
//!     let renderer = WebsysRenderer::<()>::new(|_| html! {<div> "Hello world" </div>});
//!     let output = renderer.render();
//!     assert_eq!(output, "<div>Hello World</div>");
//! }
//! ```
//!
//! The `WebsysRenderer` is particularly useful when needing to cache a Virtual DOM in between requests
//!

use dioxus_core::{
    events::EventTrigger,
    prelude::{Properties, VNode, VirtualDom, FC},
};
use futures::{channel::mpsc, future, SinkExt, StreamExt};
use mpsc::UnboundedSender;
pub mod interpreter;
/// The `WebsysRenderer` provides a way of rendering a Dioxus Virtual DOM to the browser's DOM.
/// Under the hood, we leverage WebSys and interact directly with the DOM

///
pub struct WebsysRenderer<T: Properties> {
    internal_dom: VirtualDom<T>,
}

/// Implement VirtualDom with no props for components that initialize their state internal to the VDom rather than externally.
impl WebsysRenderer<()> {
    /// Create a new instance of the Dioxus Virtual Dom with no properties for the root component.
    ///
    /// This means that the root component must either consumes its own context, or statics are used to generate the page.
    /// The root component can access things like routing in its context.
    pub fn new(root: FC<()>) -> Self {
        Self::new_with_props(root, ())
    }
}

impl<T: Properties + 'static> WebsysRenderer<T> {
    /// Create a new text-renderer instance from a functional component root.
    /// Automatically progresses the creation of the VNode tree to completion.
    ///
    /// A VDom is automatically created. If you want more granular control of the VDom, use `from_vdom`
    pub fn new_with_props(root: FC<T>, root_props: T) -> Self {
        Self::from_vdom(VirtualDom::new_with_props(root, root_props))
    }

    /// Create a new text renderer from an existing Virtual DOM.
    /// This will progress the existing VDom's events to completion.
    pub fn from_vdom(dom: VirtualDom<T>) -> Self {
        Self { internal_dom: dom }
    }

    /// Run the renderer, progressing any events that crop up
    /// Yield on event handlers
    /// If the dom errors out, self is consumed and the dom is torn down
    pub async fn run(self) -> dioxus_core::error::Result<()> {
        let WebsysRenderer { mut internal_dom } = self;

        // Progress the mount of the root component
        internal_dom
            .progress()
            .expect("Progressing the root failed :(");

        // set up the channels to connect listeners to the event loop
        let (sender, mut receiver) = mpsc::unbounded::<EventTrigger>();

        // Iterate through the nodes, attaching the closure and sender to the listener
        {
            let mut remote_sender = sender.clone();
            let f = move || {
                let event = EventTrigger::new();
                wasm_bindgen_futures::spawn_local(async move {
                    remote_sender
                        .send(event)
                        .await
                        .expect("Updating receiver failed");
                })
            };
        }

        // Event loop waits for the receiver to finish up
        // TODO! Connect the sender to the virtual dom's suspense system
        // Suspense is basically an external event that can force renders to specific nodes
        while let Some(event) = receiver.next().await {
            // event is triggered
            // relevant listeners are ran
            // internal state is modified, components are tagged for changes

            match internal_dom.progress_with_event(event).await {
                Err(_) => {}
                Ok(_) => render_diffs(),
            }
            // waiting for next event to arrive from the external triggers
        }

        Ok(())
    }
}

/// For any listeners in the tree, attach the sender closure.
/// When a event is triggered, we convert it into the synthetic event type and dump it back in the Virtual Dom's queu
fn attach_listeners<P: Properties>(sender: &UnboundedSender<EventTrigger>, dom: &VirtualDom<P>) {}

fn render_diffs() {}
