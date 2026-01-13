//! VirtualDom thread management for desktop.
//!
//! This module handles running the VirtualDom on a dedicated thread to improve
//! responsiveness. The main thread continues to run the tao event loop and manage
//! windows, while VirtualDom polling and rendering happens on separate threads.

use crate::desktop_context::DesktopContext;
use crate::document::DesktopDocument;
use crate::file_upload::NativeFileHover;
use crate::ipc::UserWindowEvent;
use crate::shortcut::HotKeyState;
use crate::AssetRequest;
use dioxus_core::{provide_context, ScopeId, VirtualDom};
use dioxus_history::{History, MemoryHistory};
use dioxus_interpreter_js::MutationState;
use futures_channel::mpsc as futures_mpsc;
use futures_util::FutureExt;
use slab::Slab;
use std::panic::AssertUnwindSafe;
use std::{any::Any, cell::RefCell, collections::HashMap, future::Future, pin::Pin, rc::Rc};
use tao::{event_loop::EventLoopProxy, window::WindowId};
use tokio::sync::mpsc::{self as tokio_mpsc, UnboundedSender};
use tokio::task::AbortHandle;
use wry::RequestAsyncResponder;

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

    /// Run a callback on the DOM thread.
    ///
    /// This is used for the inverted callback pattern where closures stay on the
    /// DOM thread and the main thread invokes them via message passing.
    RunCallback(DomCallbackRequest),
}

/// A request to run a callback on the DOM thread.
///
/// This is used by the inverted callback pattern to invoke non-Send closures
/// that are stored on the DOM thread.
pub struct DomCallbackRequest {
    /// The callback to run. This closure has access to the `DomCallbackRegistry`
    /// and can look up and invoke stored handlers.
    pub callback: Box<dyn FnOnce(&mut DomCallbackRegistry) + Send>,
    /// Optional channel to send the result back to the caller.
    pub result_tx: Option<std::sync::mpsc::SyncSender<Box<dyn Any + Send>>>,
}

/// Unique identifier for a shortcut callback stored on the DOM thread.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DomShortcutId(pub usize);

/// Registry for callbacks that live on the DOM thread.
///
/// This registry stores non-Send closures that are invoked via the inverted
/// callback pattern. The main thread sends requests to invoke these callbacks,
/// and the DOM thread looks them up and executes them.
pub struct DomCallbackRegistry {
    /// Asset handlers keyed by name.
    asset_handlers: HashMap<String, Box<dyn Fn(AssetRequest, RequestAsyncResponder)>>,
    /// Shortcut callbacks stored in a slab for efficient allocation.
    shortcut_callbacks: Slab<Box<dyn FnMut(HotKeyState)>>,
}

impl Default for DomCallbackRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl DomCallbackRegistry {
    /// Create a new empty callback registry.
    pub fn new() -> Self {
        Self {
            asset_handlers: HashMap::new(),
            shortcut_callbacks: Slab::new(),
        }
    }

    /// Register an asset handler.
    pub fn register_asset_handler(
        &mut self,
        name: String,
        handler: Box<dyn Fn(AssetRequest, RequestAsyncResponder)>,
    ) {
        self.asset_handlers.insert(name, handler);
    }

    /// Remove an asset handler.
    pub fn remove_asset_handler(&mut self, name: &str) -> Option<()> {
        self.asset_handlers.remove(name).map(|_| ())
    }

    /// Invoke an asset handler if it exists.
    pub fn invoke_asset_handler(
        &self,
        name: &str,
        request: AssetRequest,
        responder: RequestAsyncResponder,
    ) -> bool {
        if let Some(handler) = self.asset_handlers.get(name) {
            handler(request, responder);
            true
        } else {
            false
        }
    }

    /// Register a shortcut callback and return its ID.
    pub fn register_shortcut_callback(
        &mut self,
        callback: Box<dyn FnMut(HotKeyState)>,
    ) -> DomShortcutId {
        DomShortcutId(self.shortcut_callbacks.insert(callback))
    }

    /// Remove a shortcut callback.
    pub fn remove_shortcut_callback(&mut self, id: DomShortcutId) -> Option<()> {
        if self.shortcut_callbacks.contains(id.0) {
            let _ = self.shortcut_callbacks.remove(id.0);
            Some(())
        } else {
            None
        }
    }

    /// Invoke a shortcut callback if it exists.
    pub fn invoke_shortcut_callback(&mut self, id: DomShortcutId, state: HotKeyState) -> bool {
        if let Some(callback) = self.shortcut_callbacks.get_mut(id.0) {
            callback(state);
            true
        } else {
            false
        }
    }
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
    event_tx: tokio_mpsc::UnboundedSender<VirtualDomEvent>,
    command_tx: futures_mpsc::UnboundedSender<MainThreadCommand>,
    proxy: EventLoopProxy<UserWindowEvent>,
    window_id: WindowId,
    file_hover: NativeFileHover,
) where
    F: FnOnce() -> VirtualDom + Send + 'static,
{
    let dom = make_dom();
    crate::wry_bindgen_bridge::setup_event_handler(dom.runtime(), file_hover);
    let history_provider: Rc<dyn History> = Rc::new(MemoryHistory::default());
    let desktop_service_proxy = DesktopContext::new(proxy, window_id, event_tx);

    // Create the callback registry for the inverted callback pattern
    let callback_registry = Rc::new(RefCell::new(DomCallbackRegistry::new()));

    dom.in_scope(ScopeId::ROOT, || {
        provide_context(history_provider);
        provide_context(Rc::new(DesktopDocument::new(desktop_service_proxy.clone()))
            as Rc<dyn dioxus_document::Document>);
        provide_context(desktop_service_proxy);
        provide_context(callback_registry.clone());
    });
    run_virtual_dom_loop(dom, event_rx, command_tx, callback_registry).await;
}

/// The main event loop for the VirtualDom running on its dedicated thread.
async fn run_virtual_dom_loop(
    mut dom: VirtualDom,
    mut event_rx: tokio_mpsc::UnboundedReceiver<VirtualDomEvent>,
    command_tx: futures_mpsc::UnboundedSender<MainThreadCommand>,
    callback_registry: Rc<RefCell<DomCallbackRegistry>>,
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
                    VirtualDomEvent::RunCallback(request) => {
                        // Run the callback with access to the registry
                        let mut registry = callback_registry.borrow_mut();
                        (request.callback)(&mut registry);
                        // Send result back if requested
                        if let Some(result_tx) = request.result_tx {
                            // The callback should have already set up any result it needs
                            // For now, just send an empty acknowledgment
                            let _ = result_tx.send(Box::new(()));
                        }
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

type SpawnTask = (
    WindowId,
    Box<dyn FnOnce() -> Pin<Box<dyn Future<Output = ()>>> + Send>,
);
type TaskSender = UnboundedSender<SpawnTask>;

/// Handle to spawn tasks on the dom thread and abort them by window ID.
pub(crate) struct DomThreadHandle {
    /// Channel to send tasks to spawn (with associated window ID).
    pub task_tx: TaskSender,
    /// Channel to request task abortion by window ID.
    pub abort_tx: UnboundedSender<WindowId>,
}

/// Spawn a thread that runs async tasks and supports aborting them by window ID.
pub fn spawn_dom_thread(proxy: EventLoopProxy<UserWindowEvent>) -> DomThreadHandle {
    let (task_tx, mut task_rx): (TaskSender, _) = tokio::sync::mpsc::unbounded_channel();
    let (abort_tx, mut abort_rx): (UnboundedSender<WindowId>, _) =
        tokio::sync::mpsc::unbounded_channel();

    std::thread::Builder::new()
        .name("dioxus-desktop-dom".into())
        .spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("Failed to create Tokio runtime for VirtualDom thread");

            runtime.block_on(async move {
                tokio::task::LocalSet::new()
                    .run_until(async {
                        let mut abort_handles: HashMap<WindowId, AbortHandle> = HashMap::new();

                        loop {
                            tokio::select! {
                                biased;

                                // Handle abort requests with priority
                                Some(window_id) = abort_rx.recv() => {
                                    if let Some(handle) = abort_handles.remove(&window_id) {
                                        handle.abort();
                                    }
                                }

                                // Handle new task spawns
                                spawn_result = task_rx.recv() => {
                                    let Some((window_id, spawn_task)) = spawn_result else {
                                        // Channel closed, exit the loop
                                        break;
                                    };
                                    let fut = spawn_task();
                                    let proxy = proxy.clone();
                                    let join_handle = tokio::task::spawn_local(async move {
                                        _ = AssertUnwindSafe(fut).catch_unwind().await;
                                        // Close the window when the task completes (aborted or finished)
                                        proxy.send_event(UserWindowEvent::CloseWindow(window_id)).unwrap();
                                    });
                                    abort_handles.insert(window_id, join_handle.abort_handle());
                                }
                            }
                        }
                    })
                    .await;
            });
        })
        .expect("Failed to spawn VirtualDom thread");

    DomThreadHandle { task_tx, abort_tx }
}
