//! VirtualDom thread management for desktop.
//!
//! This module handles running the VirtualDom on a dedicated thread to improve
//! responsiveness. The main thread continues to run the tao event loop and manage
//! windows, while VirtualDom polling and rendering happens on separate threads.

use crate::document::DesktopDocument;
use dioxus_core::{provide_context, ScopeId, VirtualDom};
use dioxus_document::Document;
use dioxus_history::{History, MemoryHistory};
use dioxus_interpreter_js::MutationState;
use futures_channel::mpsc as futures_mpsc;
use std::rc::Rc;
use tokio::sync::mpsc as tokio_mpsc;

/// Events sent from the main thread to the VirtualDom thread.
pub enum VirtualDomEvent {
    /// Initialize the VirtualDom (perform initial rebuild).
    Initialize,

    /// Previous edits have been acknowledged by the webview.
    /// The VirtualDom can now render new mutations.
    EditsAcknowledged,

    /// Hot reload message from devtools.
    #[cfg(all(feature = "devtools", debug_assertions))]
    HotReload(dioxus_devtools::HotReloadMsg),
}

/// Commands sent from the VirtualDom thread to the main thread.
pub enum MainThreadCommand {
    /// Serialized mutations ready to be sent to the webview.
    Mutations(Vec<u8>),
}

/// Handle to communicate with a VirtualDom running on a dedicated thread.
///
/// This handle only contains the sender for sending events to the VirtualDom.
/// Commands from the VirtualDom are received via the shared `dom_command_rx` in `SharedContext`.
#[derive(Clone)]
pub struct VirtualDomHandle {
    /// Send events to the VirtualDom thread.
    pub event_tx: tokio_mpsc::UnboundedSender<VirtualDomEvent>,
}

impl VirtualDomHandle {
    /// Create a new handle with the given event sender.
    pub fn new(event_tx: tokio_mpsc::UnboundedSender<VirtualDomEvent>) -> Self {
        Self { event_tx }
    }

    /// Send an event to the VirtualDom thread.
    pub fn send_event(&self, event: VirtualDomEvent) {
        let _ = self.event_tx.send(event);
    }
}

/// Run the VirtualDom in the current async context (called from wry-bindgen app thread).
///
/// This creates the VirtualDom and runs its event loop until completion.
/// Also sets up the wasm-bindgen event handler for direct JS->Rust event calls.
pub async fn run_virtual_dom<F>(
    make_dom: F,
    event_rx: tokio_mpsc::UnboundedReceiver<VirtualDomEvent>,
    command_tx: futures_mpsc::UnboundedSender<MainThreadCommand>,
) where
    F: FnOnce() -> VirtualDom + Send + 'static,
{
    let dom = make_dom();
    crate::wry_bindgen_bridge::setup_event_handler(dom.runtime());
    let history_provider: Rc<dyn History> = Rc::new(MemoryHistory::default());
    // let document_provider: Rc<dyn Document> = Rc::new(DesktopDocument::default());
    dom.in_scope(ScopeId::ROOT, || {
        provide_context(history_provider);
        // provide_context(document_provider);
    });
    run_virtual_dom_loop(dom, event_rx, command_tx).await;
}

/// The main event loop for the VirtualDom running on its dedicated thread.
async fn run_virtual_dom_loop(
    mut dom: VirtualDom,
    mut event_rx: tokio_mpsc::UnboundedReceiver<VirtualDomEvent>,
    command_tx: futures_mpsc::UnboundedSender<MainThreadCommand>,
) {
    let mut mutations = MutationState::default();
    let mut waiting_for_edits_ack = false;
    let mut initialized = false;

    loop {
        // Normal operation: wait for work or events
        tokio::select! {
            biased;

            // Check for incoming events from main thread first
            event = event_rx.recv() => {
                let Some(event) = event else {
                    // Channel closed
                    return;
                };
                match event {
                    VirtualDomEvent::Initialize => {
                        if !initialized {
                            initialized = true;
                            // Perform initial rebuild
                            dom.rebuild(&mut mutations);
                            if let Some(edits) = take_edits(&mut mutations) {
                                let _ = command_tx.unbounded_send(MainThreadCommand::Mutations(edits));
                                waiting_for_edits_ack = true;
                            }
                        }
                    }
                    VirtualDomEvent::EditsAcknowledged => {
                        waiting_for_edits_ack = false;
                    }
                    #[cfg(all(feature = "devtools", debug_assertions))]
                    VirtualDomEvent::HotReload(msg) => {
                        dioxus_devtools::apply_changes(&dom, &msg);
                    }
                }
            }

            // Wait for the VirtualDom to have work ready
            _ = dom.wait_for_work(), if initialized && !waiting_for_edits_ack => {
                // Render and send mutations
                dom.render_immediate(&mut mutations);
                if let Some(edits) = take_edits(&mut mutations) {
                    let _ = command_tx.unbounded_send(MainThreadCommand::Mutations(edits));
                    waiting_for_edits_ack = true;
                }
            }
        }
    }
}

/// Export mutations from the MutationState if there are any.
fn take_edits(mutations: &mut MutationState) -> Option<Vec<u8>> {
    let bytes = mutations.export_memory();
    if bytes.is_empty() {
        None
    } else {
        Some(bytes)
    }
}
