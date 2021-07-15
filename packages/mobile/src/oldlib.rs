use dioxus::virtual_dom::VirtualDom;
pub use dioxus_core as dioxus;
use dioxus_core::{events::EventTrigger, prelude::FC};

pub struct IosRenderer {
    internal_dom: VirtualDom,
}

impl IosRenderer {
    pub async fn run(&mut self) -> dioxus_core::error::Result<()> {
        let (sender, mut receiver) = async_channel::unbounded::<EventTrigger>();

        // let body_element = prepare_websys_dom();

        let mut patch_machine = interpreter::PatchMachine::new(body_element.clone(), move |ev| {
            log::debug!("Event trigger! {:#?}", ev);
            let mut c = sender.clone();
            // wasm_bindgen_futures::spawn_local(async move {
            //     c.send(ev).await.unwrap();
            // });
        });
        let root_node = body_element.first_child().unwrap();
        patch_machine.stack.push(root_node.clone());

        // todo: initialize the event registry properly on the root

        let edits = self.internal_dom.rebuild()?;
        log::debug!("Received edits: {:#?}", edits);
        edits.iter().for_each(|edit| {
            log::debug!("patching with  {:?}", edit);
            patch_machine.handle_edit(edit);
        });

        patch_machine.reset();
        let root_node = body_element.first_child().unwrap();
        patch_machine.stack.push(root_node.clone());

        // log::debug!("patch stack size {:?}", patch_machine.stack);

        // Event loop waits for the receiver to finish up
        // TODO! Connect the sender to the virtual dom's suspense system
        // Suspense is basically an external event that can force renders to specific nodes
        while let Ok(event) = receiver.recv().await {
            log::debug!("Stack before entrance {:#?}", patch_machine.stack.top());
            // log::debug!("patch stack size before {:#?}", patch_machine.stack);
            // patch_machine.reset();
            // patch_machine.stack.push(root_node.clone());
            let edits = self.internal_dom.progress_with_event(event)?;
            log::debug!("Received edits: {:#?}", edits);

            for edit in &edits {
                log::debug!("edit stream {:?}", edit);
                // log::debug!("Stream stack {:#?}", patch_machine.stack.top());
                patch_machine.handle_edit(edit);
            }

            // log::debug!("patch stack size after {:#?}", patch_machine.stack);
            patch_machine.reset();
            // our root node reference gets invalidated
            // not sure why
            // for now, just select the first child again.
            // eventually, we'll just make our own root element instead of using body
            // or just use body directly IDEK
            let root_node = body_element.first_child().unwrap();
            patch_machine.stack.push(root_node.clone());
        }

        Ok(()) // should actually never return from this, should be an error, rustc just cant see it
    }
}
