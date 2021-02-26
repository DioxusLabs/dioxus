//! Dioxus WebSys
//! --------------
//! This crate implements a renderer of the Dioxus Virtual DOM for the web browser using Websys.

use fxhash::FxHashMap;
use web_sys::{window, Document, Element, Event, Node};

pub use dioxus_core as dioxus;
use dioxus_core::{
    events::EventTrigger,
    prelude::{VirtualDom, FC},
};
use futures::{channel::mpsc, SinkExt, StreamExt};

pub mod interpreter;

/// The `WebsysRenderer` provides a way of rendering a Dioxus Virtual DOM to the browser's DOM.
/// Under the hood, we leverage WebSys and interact directly with the DOM
///
pub struct WebsysRenderer {
    internal_dom: VirtualDom,
}

impl WebsysRenderer {
    /// This method is the primary entrypoint for Websys Dioxus apps. Will panic if an error occurs while rendering.
    /// See DioxusErrors for more information on how these errors could occour.
    ///
    /// ```ignore
    /// fn main() {
    ///     wasm_bindgen_futures::spawn_local(WebsysRenderer::start(Example));
    /// }
    /// ```
    ///
    /// Run the app to completion, panicing if any error occurs while rendering.
    /// Pairs well with the wasm_bindgen async handler
    pub async fn start(root: FC<()>) {
        Self::new(root).run().await.expect("Virtual DOM failed");
    }

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
    pub fn from_vdom(dom: VirtualDom) -> Self {
        // todo: initialize the event registry properly
        Self { internal_dom: dom }
    }

    pub async fn run(&mut self) -> dioxus_core::error::Result<()> {
        let (sender, mut receiver) = mpsc::unbounded::<EventTrigger>();

        let body_element = prepare_websys_dom();
        let mut patch_machine = interpreter::PatchMachine::new(body_element.clone(), |_| {});
        let root_node = body_element.first_child().unwrap();
        patch_machine.stack.push(root_node);

        // todo: initialize the event registry properly on the root

        self.internal_dom.rebuild()?.iter().for_each(|edit| {
            log::debug!("patching with  {:?}", edit);
            patch_machine.handle_edit(edit);
        });

        // Event loop waits for the receiver to finish up
        // TODO! Connect the sender to the virtual dom's suspense system
        // Suspense is basically an external event that can force renders to specific nodes
        while let Some(event) = receiver.next().await {
            self.internal_dom
                .progress_with_event(event)?
                .iter()
                .for_each(|edit| {
                    patch_machine.handle_edit(edit);
                });
        }

        Ok(()) // should actually never return from this, should be an error, rustc just cant see it
    }
}

fn prepare_websys_dom() -> Element {
    // Initialize the container on the dom
    // Hook up the body as the root component to render tinto
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

    container.clone()
}

// Progress the mount of the root component

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

        wasm_bindgen_futures::spawn_local(WebsysRenderer::start(|ctx, _| {
            ctx.view(html! {
                <div>
                    "Hello world"
                    <button onclick={move |_| log::info!("button1 clicked!")}> "click me" </button>
                    <button onclick={move |_| log::info!("button2 clicked!")}> "click me" </button>
                </div>
            })
        }))
    }
}
