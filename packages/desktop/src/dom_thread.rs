//! VirtualDom thread management for desktop.
//!
//! This module handles running the VirtualDom on a dedicated thread to improve
//! responsiveness. The main thread continues to run the tao event loop and manage
//! windows, while VirtualDom polling and rendering happens as separate tasks on
//! a dedicated DOM thread.

use crate::AssetRequest;
use crate::desktop_context::DesktopContext;
use crate::document::DesktopDocument;
use crate::edits::EditWebsocket;
use crate::file_upload::NativeFileHover;
use crate::ipc::{UserWindowEvent, UserWindowEventVariant, WindowHandle};
use dioxus_core::{ElementId, ScopeId, VirtualDom, provide_context};
use dioxus_history::{History, MemoryHistory};
use dioxus_interpreter_js::MutationState;
use futures_channel::oneshot;
use futures_util::FutureExt;
use futures_util::future::OptionFuture;
use slotmap::SlotMap;
use std::panic::AssertUnwindSafe;
use std::sync::Arc;
use std::{any::Any, cell::RefCell, collections::HashMap, future::Future, pin::Pin, rc::Rc};
use tao::{event_loop::EventLoopProxy, window::WindowId};
use tokio::sync::mpsc::{self as tokio_mpsc, UnboundedSender};
use tokio::task::AbortHandle;
use wry::RequestAsyncResponder;

/// Events sent from the main thread to the VirtualDom thread.
pub(crate) enum VirtualDomEvent {
    /// Initialize the VirtualDom (perform initial rebuild).
    Initialize,

    /// Hot reload message from devtools.
    #[cfg(all(feature = "devtools", debug_assertions))]
    HotReload(dioxus_devtools::HotReloadMsg),

    /// Run a callback on the DOM thread.
    ///
    /// This is used for the inverted callback pattern where closures stay on the
    /// DOM thread and the main thread invokes them via message passing.
    RunCallback(Box<dyn FnOnce(&SharedCallbackRegistry) + Send>),
}

/// A type-erased callback stored on the DOM thread. The wrapper created in
/// [`SharedCallbackRegistry::register`] downcasts the argument back to its concrete type.
type StoredCallback = Box<dyn FnMut(Box<dyn Any>)>;

slotmap::new_key_type! {
    /// Unique identifier for a callback stored on the DOM thread. Ids are generational: a
    /// removed callback's id can never dispatch to a different callback reusing its slot.
    pub struct DomCallbackId;
}

/// A shared handle to this window's [`DomCallbackRegistry`]. Invocation never holds the inner
/// `RefCell` borrow while user code runs, so callbacks can register/remove callbacks freely.
#[derive(Clone, Default)]
pub(crate) struct SharedCallbackRegistry(Rc<RefCell<DomCallbackRegistry>>);

thread_local! {
    /// Every window's callback registry, keyed by window id, so a `DesktopContext` for *any*
    /// window resolves the registry matching the `dom_tx` it dispatches through.
    static WINDOW_REGISTRIES: RefCell<HashMap<WindowId, SharedCallbackRegistry>> =
        RefCell::new(HashMap::new());
}

/// Make `registry` discoverable by `window_id` until the returned guard drops (the guard lives
/// inside the window's VirtualDom task, so cleanup also covers task aborts).
pub(crate) fn register_window_registry(
    window_id: WindowId,
    registry: SharedCallbackRegistry,
) -> WindowRegistryGuard {
    WINDOW_REGISTRIES.with(|registries| {
        registries.borrow_mut().insert(window_id, registry);
    });
    WindowRegistryGuard { window_id }
}

/// Look up the callback registry for a window. Returns `None` if the window's VirtualDom is not
/// running (window closed, not yet started) or when called off the DOM thread.
pub(crate) fn lookup_window_registry(window_id: WindowId) -> Option<SharedCallbackRegistry> {
    WINDOW_REGISTRIES.with(|registries| registries.borrow().get(&window_id).cloned())
}

/// Removes a window's registry entry from [`WINDOW_REGISTRIES`] on drop.
pub(crate) struct WindowRegistryGuard {
    window_id: WindowId,
}

impl Drop for WindowRegistryGuard {
    fn drop(&mut self) {
        WINDOW_REGISTRIES.with(|registries| {
            registries.borrow_mut().remove(&self.window_id);
        });
    }
}

/// The name of the dedicated DOM thread spawned by [`spawn_dom_thread`].
const DOM_THREAD_NAME: &str = "dioxus-desktop-dom";

/// Registry for callbacks that live on the DOM thread.
///
/// This registry stores non-Send closures that are invoked via the inverted
/// callback pattern. The main thread sends requests to invoke these callbacks,
/// and the DOM thread looks them up and executes them.
#[derive(Default)]
pub(crate) struct DomCallbackRegistry {
    /// Callback storage. A slot is `None` while its callback is checked out for invocation; the
    /// entry stays in the map so the id remains reserved until the callback is restored.
    callbacks: SlotMap<DomCallbackId, Option<StoredCallback>>,
    /// Asset handler names point into the shared callback map.
    asset_handler_names: HashMap<String, DomCallbackId>,
}

/// Erase a callback's argument type so every callback kind shares one storage type. The wrapper
/// restores the concrete type on invocation.
fn erase<A: 'static>(mut f: impl FnMut(A) + 'static) -> StoredCallback {
    Box::new(move |arg| match arg.downcast::<A>() {
        Ok(arg) => f(*arg),
        Err(_) => tracing::warn!("dom callback invoked with an unexpected argument type"),
    })
}

impl SharedCallbackRegistry {
    /// Register a callback taking an argument of type `A`. Invoke it with
    /// [`Self::invoke`]`(id, arg)`.
    pub fn register<A: 'static>(&self, f: impl FnMut(A) + 'static) -> DomCallbackId {
        self.0.borrow_mut().callbacks.insert(Some(erase(f)))
    }

    /// Remove a callback. Works even while the callback is checked out for invocation; the
    /// in-flight restore will see the missing entry and drop the closure.
    pub fn remove(&self, id: DomCallbackId) -> Option<()> {
        self.0.borrow_mut().callbacks.remove(id).map(|_| ())
    }

    /// Invoke a callback without holding the registry borrow while user code runs.
    ///
    /// The callback is taken out of the registry, invoked, and restored afterwards, so the
    /// callback itself may register or remove callbacks (including itself). Returns `false` if
    /// the callback is missing (already removed, or currently running reentrantly).
    pub fn invoke<A: 'static>(&self, id: DomCallbackId, arg: A) -> bool {
        let taken = self
            .0
            .borrow_mut()
            .callbacks
            .get_mut(id)
            .and_then(Option::take);
        let Some(mut callback) = taken else {
            tracing::warn!(
                "dropping invocation of dom callback {id:?}: it was removed or is already running"
            );
            return false;
        };
        callback(Box::new(arg));
        // Put the callback back, unless it was removed (or replaced) while it ran.
        if let Some(slot @ None) = self.0.borrow_mut().callbacks.get_mut(id) {
            *slot = Some(callback);
        }
        true
    }

    /// Register an asset handler under a name, replacing any previous handler with that name.
    pub fn register_asset_handler(
        &self,
        name: String,
        mut handler: impl FnMut(AssetRequest, RequestAsyncResponder) + 'static,
    ) {
        let id = self.register(move |(request, responder)| handler(request, responder));
        let mut registry = self.0.borrow_mut();
        if let Some(old_id) = registry.asset_handler_names.insert(name, id) {
            registry.callbacks.remove(old_id);
        }
    }

    /// Remove an asset handler.
    pub fn remove_asset_handler(&self, name: &str) -> Option<()> {
        let id = self.0.borrow_mut().asset_handler_names.remove(name)?;
        self.remove(id)
    }

    /// Invoke an asset handler if it exists.
    pub fn invoke_asset_handler(
        &self,
        name: &str,
        request: AssetRequest,
        responder: RequestAsyncResponder,
    ) -> bool {
        let Some(id) = self.0.borrow_mut().asset_handler_names.get(name).copied() else {
            return false;
        };
        self.invoke(id, (request, responder))
    }
}

/// Run the VirtualDom's event loop until completion in the current async context (called from
/// the wry-bindgen app task on the DOM thread). Also sets up the wasm-bindgen event handler for
/// direct JS->Rust event calls.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn run_virtual_dom_with_dom(
    dom: VirtualDom,
    event_rx: tokio_mpsc::UnboundedReceiver<VirtualDomEvent>,
    event_tx: tokio_mpsc::UnboundedSender<VirtualDomEvent>,
    websocket: EditWebsocket,
    webview_id: u32,
    window_handle: Arc<WindowHandle>,
    file_hover: NativeFileHover,
) {
    crate::wry_bindgen_bridge::setup_event_handler(dom.runtime(), file_hover);
    let history_provider: Rc<dyn History> = Rc::new(MemoryHistory::default());
    let window_id = window_handle.window_id;
    let desktop_service_proxy = DesktopContext::new(event_tx, window_handle);

    // Create the callback registry for the inverted callback pattern and make it discoverable
    // by window id for the lifetime of this VirtualDom task (the guard cleans up on abort too).
    let callback_registry = SharedCallbackRegistry::default();
    let _registry_guard = register_window_registry(window_id, callback_registry.clone());

    dom.in_scope(ScopeId::ROOT, || {
        provide_context(history_provider);
        provide_context(Rc::new(DesktopDocument::new(desktop_service_proxy.clone()))
            as Rc<dyn dioxus_document::Document>);
        provide_context(desktop_service_proxy);
    });
    run_virtual_dom_loop(dom, event_rx, websocket, webview_id, callback_registry).await;
}

/// The main event loop for the VirtualDom running on its dedicated thread.
async fn run_virtual_dom_loop(
    mut dom: VirtualDom,
    mut event_rx: tokio_mpsc::UnboundedReceiver<VirtualDomEvent>,
    websocket: EditWebsocket,
    webview_id: u32,
    callback_registry: SharedCallbackRegistry,
) {
    let mut mutations = MutationState::default();
    mutations.queue_mounted_events();
    // The receiver for the edits we are currently waiting on the webview to apply. While this is
    // `Some`, we hold off rendering new work so effects don't run before the DOM has updated. The
    // websocket worker resolves it once the webview acks the edits (or drops it if the connection
    // goes away).
    let mut pending_flush: Option<PendingFlush> = None;
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
                    // The webview sends initialize on every page load, so a reload triggers a
                    // fresh rebuild for the new document
                    VirtualDomEvent::Initialize => {
                        initialized = true;
                        // Perform initial rebuild
                        dom.rebuild(&mut mutations);
                        pending_flush = Some(send_edits(
                            &websocket,
                            webview_id,
                            &mut mutations,
                        ));
                    }
                    #[cfg(all(feature = "devtools", debug_assertions))]
                    VirtualDomEvent::HotReload(msg) => {
                        dioxus_devtools::apply_changes(&dom, &msg);
                    }
                    VirtualDomEvent::RunCallback(callback) => {
                        // Run the callback with access to the registry. The registry handle is
                        // passed by reference (not a borrow of its RefCell) so the callback can
                        // register and remove other callbacks while it runs.
                        callback(&callback_registry);
                    }
                }
            }

            // The webview applied the in-flight edits (Ok) or the connection dropped them (Err).
            // Either way we are no longer waiting, so rendering can resume.
            Some(applied) = OptionFuture::from(pending_flush.as_mut().map(|flush| &mut flush.applied)) => {
                let flush = pending_flush.take().expect("this arm only fires while a flush is pending");
                if applied.is_ok() {
                    let runtime = dom.runtime();
                    for id in flush.mounted_events {
                        crate::wry_bindgen_bridge::handle_mounted_event(&runtime, id);
                    }
                }
            }

            // Wait for the VirtualDom to have work ready
            _ = dom.wait_for_work(), if initialized && pending_flush.is_none() => {
                // Render and send mutations straight to the webview's edit websocket
                dom.render_immediate(&mut mutations);
                pending_flush = Some(send_edits(
                    &websocket,
                    webview_id,
                    &mut mutations,
                ));
            }
        }
    }
}

struct PendingFlush {
    applied: oneshot::Receiver<()>,
    mounted_events: Vec<ElementId>,
}

fn send_edits(
    websocket: &EditWebsocket,
    webview_id: u32,
    mutations: &mut MutationState,
) -> PendingFlush {
    let edits = mutations.export_memory();
    let mounted_events = mutations.take_mounted_events();
    PendingFlush {
        applied: websocket.send_edits(webview_id, edits),
        mounted_events,
    }
}

/// Messages for the dom thread. Spawns and aborts share one channel so they arrive in the order
/// the main thread sent them: an abort can never overtake its window's spawn message and get
/// dropped, which would leave an unabortable task (and its window) alive forever.
pub(crate) enum DomThreadMessage {
    /// Spawn the VirtualDom task for a window.
    Spawn(
        WindowId,
        Box<dyn FnOnce() -> Pin<Box<dyn Future<Output = ()>>> + Send>,
    ),
    /// Abort the VirtualDom task for a window.
    Abort(WindowId),
}

/// Handle to spawn tasks on the dom thread and abort them by window ID.
pub(crate) struct DomThreadHandle {
    pub tx: UnboundedSender<DomThreadMessage>,
}

/// Spawn a thread that runs async tasks and supports aborting them by window ID.
pub(crate) fn spawn_dom_thread(proxy: EventLoopProxy<UserWindowEvent>) -> DomThreadHandle {
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

    std::thread::Builder::new()
        .name(DOM_THREAD_NAME.into())
        .spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("Failed to create Tokio runtime for VirtualDom thread");

            runtime.block_on(async move {
                tokio::task::LocalSet::new()
                    .run_until(async {
                        let abort_handles: Rc<RefCell<HashMap<WindowId, AbortHandle>>> =
                            Rc::new(RefCell::new(HashMap::new()));

                        while let Some(message) = rx.recv().await {
                            match message {
                                DomThreadMessage::Abort(window_id) => {
                                    // A missing handle means the task already finished naturally.
                                    let handle = abort_handles.borrow_mut().remove(&window_id);
                                    if let Some(handle) = handle {
                                        handle.abort();
                                    }
                                }

                                DomThreadMessage::Spawn(window_id, spawn_task) => {
                                    let fut = spawn_task();
                                    let proxy = proxy.clone();
                                    let handles = abort_handles.clone();
                                    let join_handle = tokio::task::spawn_local(async move {
                                        _ = AssertUnwindSafe(fut).catch_unwind().await;
                                        // Runs when the VirtualDom task finishes or panics (never
                                        // on abort). Start tearing down the window: hiding a
                                        // window whose VirtualDom is gone would leave a zombie.
                                        handles.borrow_mut().remove(&window_id);
                                        _ = proxy.send_event(
                                            UserWindowEventVariant::DestroyWindow(window_id).into(),
                                        );
                                    });
                                    abort_handles
                                        .borrow_mut()
                                        .insert(window_id, join_handle.abort_handle());
                                }
                            }
                        }
                    })
                    .await;
            });
        })
        .expect("Failed to spawn VirtualDom thread");

    DomThreadHandle { tx }
}
