//! VirtualDom thread management for desktop.
//!
//! This module handles running the VirtualDom on a dedicated thread to improve
//! responsiveness. The main thread continues to run the tao event loop and manage
//! windows, while VirtualDom polling and rendering happens on separate threads.

use dioxus_core::{provide_context, ScopeId, VirtualDom};
use dioxus_document::Document;
use dioxus_history::{History, MemoryHistory};
use dioxus_html::HtmlEvent;
use dioxus_interpreter_js::MutationState;
use std::{
    rc::Rc,
    sync::{mpsc as std_mpsc, Arc, OnceLock},
};
use tokio::sync::mpsc as tokio_mpsc;
use tokio_util::task::LocalPoolHandle;

use crate::document::DesktopDocument;

/// Events sent from the main thread to the VirtualDom thread.
pub enum VirtualDomEvent {
    /// An HTML event from the webview that needs to be handled.
    HtmlEvent {
        event: HtmlEvent,
        prevent_default: Arc<OnceLock<bool>>,
    },

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

    /// Request to evaluate a script in the webview (e.g., update websocket location).
    EvaluateScript(String),
}

/// Handle to communicate with a VirtualDom running on a dedicated thread.
pub struct VirtualDomHandle {
    /// Send events to the VirtualDom thread.
    pub event_tx: tokio_mpsc::UnboundedSender<VirtualDomEvent>,

    /// Receive commands from the VirtualDom thread.
    /// Uses std::sync::mpsc for non-async receiving on main thread.
    pub command_rx: std_mpsc::Receiver<MainThreadCommand>,
}

impl VirtualDomHandle {
    /// Send an event to the VirtualDom thread.
    pub fn send_event(&self, event: VirtualDomEvent) {
        let _ = self.event_tx.send(event);
    }

    /// Try to receive a command from the VirtualDom thread without blocking.
    pub fn try_recv_command(&self) -> Option<MainThreadCommand> {
        self.command_rx.try_recv().ok()
    }
}

/// Spawn a VirtualDom on a dedicated thread using pre-created channels.
///
/// This variant allows the caller to create channels before spawning, which is useful
/// when the event sender needs to be shared with other components (like WebviewEdits)
/// before the VirtualDom thread is spawned.
///
/// Takes both ends of the channels:
/// - event_tx/event_rx: For sending events TO the VirtualDom thread
/// - command_tx/command_rx: For sending commands FROM the VirtualDom thread
pub fn spawn_virtual_dom_with_channels<F>(
    pool: &LocalPoolHandle,
    make_dom: F,
    event_tx: tokio_mpsc::UnboundedSender<VirtualDomEvent>,
    event_rx: tokio_mpsc::UnboundedReceiver<VirtualDomEvent>,
    command_tx: std_mpsc::Sender<MainThreadCommand>,
    command_rx: std_mpsc::Receiver<MainThreadCommand>,
) -> VirtualDomHandle
where
    F: FnOnce() -> VirtualDom + Send + 'static,
{
    let _ = pool.spawn_pinned(move || {
        let dom = make_dom();
        // TODO: Restore once we set up the main thread proxy
        // let provider: Rc<dyn Document> = Rc::new(DesktopDocument::new(desktop_context.clone()));
        let history_provider: Rc<dyn History> = Rc::new(MemoryHistory::default());
        dom.in_scope(ScopeId::ROOT, || {
            // provide_context(desktop_context.clone());
            // provide_context(provider);
            provide_context(history_provider);
        });
        run_virtual_dom_loop(dom, event_rx, command_tx)
    });

    VirtualDomHandle {
        event_tx,
        command_rx,
    }
}

/// The main event loop for the VirtualDom running on its dedicated thread.
async fn run_virtual_dom_loop(
    mut dom: VirtualDom,
    mut event_rx: tokio_mpsc::UnboundedReceiver<VirtualDomEvent>,
    command_tx: std_mpsc::Sender<MainThreadCommand>,
) {
    let mut mutations = MutationState::default();
    let mut waiting_for_edits_ack = false;
    let mut initialized = false;

    // Hot reload messages come via VirtualDomEvent::HotReload from the main thread
    // which has already connected to devtools

    loop {
        // If we're waiting for edits to be acknowledged, only process events
        // that don't require rendering
        if waiting_for_edits_ack {
            if let Some(event) = event_rx.recv().await {
                match event {
                    VirtualDomEvent::EditsAcknowledged => {
                        waiting_for_edits_ack = false;
                    }
                    VirtualDomEvent::HtmlEvent {
                        event,
                        prevent_default,
                    } => {
                        handle_html_event(&dom, event, &prevent_default);
                    }
                    #[cfg(all(feature = "devtools", debug_assertions))]
                    VirtualDomEvent::HotReload(msg) => {
                        dioxus_devtools::apply_changes(&dom, &msg);
                    }
                    VirtualDomEvent::Initialize => {
                        // Already initialized, ignore
                    }
                }
            } else {
                // Channel closed
                return;
            }
            continue;
        }

        // Normal operation: wait for work or events
        tokio::select! {
            biased;

            // Check for incoming events first
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
                                let _ = command_tx.send(MainThreadCommand::Mutations(edits));
                                waiting_for_edits_ack = true;
                            }
                        }
                    }
                    VirtualDomEvent::EditsAcknowledged => {
                        // No-op when not waiting
                    }
                    VirtualDomEvent::HtmlEvent { event, prevent_default } => {
                        handle_html_event(&dom, event, &prevent_default);
                    }
                    #[cfg(all(feature = "devtools", debug_assertions))]
                    VirtualDomEvent::HotReload(msg) => {
                        dioxus_devtools::apply_changes(&dom, &msg);
                    }
                }
            }

            // Wait for the VirtualDom to have work ready
            _ = dom.wait_for_work(), if initialized => {
                // Render and send mutations
                dom.render_immediate(&mut mutations);
                if let Some(edits) = take_edits(&mut mutations) {
                    let _ = command_tx.send(MainThreadCommand::Mutations(edits));
                    waiting_for_edits_ack = true;
                }
            }
        }
    }
}

/// Handle an HTML event from the webview.
///
/// This function converts serialized event data to desktop-specific types
/// that provide full functionality (like file handling for form events).
fn handle_html_event(dom: &VirtualDom, event: HtmlEvent, prevent_default: &OnceLock<bool>) {
    let HtmlEvent {
        element,
        name,
        bubbles,
        data,
    } = event;
    println!("Handling HTML event: {}", name);

    // Convert to desktop-specific event types where needed
    let as_any = data.into_any();

    let event = dioxus_core::Event::new(as_any, bubbles);
    dom.runtime().handle_event(&name, event.clone(), element);

    _ = prevent_default.set(event.default_action_enabled());
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
