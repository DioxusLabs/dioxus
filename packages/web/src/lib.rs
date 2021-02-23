//! Dioxus WebSys
//! --------------
//! This crate implements a renderer of the Dioxus Virtual DOM for the web browser.
//!
//! While it is possible to render a single component directly, it is not possible to render component trees. For these,
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
use web_sys::{window, Document, Element, Event, Node};

use dioxus::prelude::VElement;
// use dioxus::{patch::Patch, prelude::VText};
// use dioxus::{patch::Patch, prelude::VText};
pub use dioxus_core as dioxus;
use dioxus_core::{
    events::EventTrigger,
    prelude::{bumpalo::Bump, html, DiffMachine, VNode, VirtualDom, FC},
};
use futures::{channel::mpsc, future, SinkExt, StreamExt};
use mpsc::UnboundedSender;
pub mod interpreter;
use interpreter::PatchMachine;
/// The `WebsysRenderer` provides a way of rendering a Dioxus Virtual DOM to the browser's DOM.
/// Under the hood, we leverage WebSys and interact directly with the DOM

///
pub struct WebsysRenderer {
    internal_dom: VirtualDom,
    // Map of handlers
}

impl WebsysRenderer {
    /// Create a new instance of the Dioxus Virtual Dom with no properties for the root component.
    ///
    /// This means that the root component must either consumes its own context, or statics are used to generate the page.
    /// The root component can access things like routing in its context.
    pub fn new(root: FC<()>) -> Self {
        Self::new_with_props(root, ())
    }
    /// Create a new text-renderer instance from a functional component root.
    /// Automatically progresses the creation of the VNode tree to completion.
    ///
    /// A VDom is automatically created. If you want more granular control of the VDom, use `from_vdom`
    pub fn new_with_props<T: 'static>(root: FC<T>, root_props: T) -> Self {
        Self::from_vdom(VirtualDom::new_with_props(root, root_props))
    }

    /// Create a new text renderer from an existing Virtual DOM.
    /// This will progress the existing VDom's events to completion.
    pub fn from_vdom(dom: VirtualDom) -> Self {
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
        // {
        //     let mut remote_sender = sender.clone();
        //     let listener = move || {
        //         let event = EventTrigger::new();
        //         wasm_bindgen_futures::spawn_local(async move {
        //             remote_sender
        //                 .send(event)
        //                 .await
        //                 .expect("Updating receiver failed");
        //         })
        //     };
        // }

        // Event loop waits for the receiver to finish up
        // TODO! Connect the sender to the virtual dom's suspense system
        // Suspense is basically an external event that can force renders to specific nodes
        while let Some(event) = receiver.next().await {
            // event is triggered
            // relevant listeners are ran
            // internal state is modified, components are tagged for changes

            match internal_dom.progress_with_event(event).await {
                Err(_) => {}
                Ok(_) => {} // Ok(_) => render_diffs(),
            }
            // waiting for next event to arrive from the external triggers
        }

        Ok(())
    }

    pub fn simple_render(tree: impl for<'a> Fn(&'a Bump) -> VNode<'a>) {
        let bump = Bump::new();

        // Choose the body to render the app into
        let window = web_sys::window().expect("should have access to the Window");
        let document = window
            .document()
            .expect("should have access to the Document");
        let body = document.body().unwrap();

        // Build a dummy div
        let container: &Element = body.as_ref();
        container.set_inner_html("");
        container
            .append_child(
                document
                    .create_element("div")
                    .expect("should create element OK")
                    .as_ref(),
            )
            .expect("should append child OK");

        // Create the old dom and the new dom
        // The old is just an empty div, like the one we made above
        let old = html! { <div> </div> }(&bump);
        let new = tree(&bump);

        // Build a machine that diffs doms
        let mut diff_machine = DiffMachine::new(&bump);
        diff_machine.diff_node(&old, &new);

        // Build a machine that patches doms
        // In practice, the diff machine might be on a different computer, sending us patches
        let mut patch_machine = PatchMachine::new(body.clone().into());

        // need to make sure we push the root node onto the stack before trying to run anything
        // this provides an entrance for the diffing machine to do its work
        // Here, we grab the div out of the container (the body) to connect with the dummy div we made above
        // This is because we don't support fragments (yet)
        let root_node = container.first_child().unwrap();
        patch_machine.stack.push(root_node);

        // Consume the diff machine, generating the patch list
        for patch in diff_machine.consume() {
            patch_machine.handle_edit(&patch);
            log::info!("Patch is {:?}", patch);
        }
    }

    pub fn complex_render(
        tree1: impl for<'a> Fn(&'a Bump) -> VNode<'a>,
        tree2: impl for<'a> Fn(&'a Bump) -> VNode<'a>,
    ) {
        let bump = Bump::new();

        let old = tree1(&bump);
        let new = tree2(&bump);

        let mut machine = DiffMachine::new(&bump);
        machine.diff_node(&old, &new);

        for patch in machine.consume() {
            println!("Patch is {:?}", patch);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::env;

    use super::*;
    use dioxus_core as dioxus;
    use dioxus_core::prelude::html;

    #[test]
    fn simple_patch() {
        env::set_var("RUST_LOG", "trace");
        pretty_env_logger::init();
        log::info!("Hello!");
        let renderer = WebsysRenderer::simple_render(html! {
            <div>
                "Hello world"
                <button onclick={move |_| log::info!("button1 clicked!")}> "click me" </button>
                <button onclick={move |_| log::info!("button2 clicked!")}> "click me" </button>
            </div>
        });
    }

    #[test]
    fn complex_patch() {
        env::set_var("RUST_LOG", "trace");
        pretty_env_logger::init();
        log::info!("Hello!");
        let renderer = WebsysRenderer::complex_render(
            html! {
                <div>
                    "Hello world"
                    <div>
                        <h1> "Heading" </h1>
                    </div>
                </div>
            },
            html! {
                <div>
                    "Hello world"
                    "Hello world"
                    "Hello world"
                    <div>
                        <h1> "Heading" </h1>
                    </div>
                </div>
            },
        );
    }
}
